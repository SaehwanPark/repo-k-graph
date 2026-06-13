#![allow(clippy::collapsible_if)]

use rkg_core::{Location, Symbol, SymbolKind};
use std::path::Path;
use tree_sitter::{Node, Parser, TreeCursor};

pub const CRATE_NAME: &str = "rkg-lang-fsharp";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FSharpParseError {
  UnsupportedLanguage(String),
  ParseCancelled,
}

impl std::fmt::Display for FSharpParseError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      FSharpParseError::UnsupportedLanguage(message) => {
        write!(f, "failed to initialize fsharp parser: {message}")
      }
      FSharpParseError::ParseCancelled => write!(f, "fsharp parsing was cancelled"),
    }
  }
}

impl std::error::Error for FSharpParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedImport {
  pub target_qualified_name: String,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedCall {
  pub source_symbol_qualified_name: String,
  pub target_name: String,
  pub is_self_call: bool,
  pub method_name: Option<String>,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedTypeReference {
  pub source_symbol_qualified_name: String,
  pub target_type_name: String,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractedTestKind {
  Class,
  Function,
  Fixture,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedTest {
  pub name: String,
  pub qualified_name: String,
  pub kind: ExtractedTestKind,
  pub is_parametrized: bool,
  pub parameters: Vec<String>,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedFSharpRoute {
  pub handler_name: String,
  pub qualified_name: String,
  pub method: String,
  pub path: String,
  pub response_model: Option<String>,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}

fn parse_to_tree(source: &str) -> Result<tree_sitter::Tree, FSharpParseError> {
  let mut parser = Parser::new();
  parser
    .set_language(&tree_sitter_fsharp::LANGUAGE_FSHARP.into())
    .map_err(|e| FSharpParseError::UnsupportedLanguage(e.to_string()))?;
  parser
    .parse(source, None)
    .ok_or(FSharpParseError::ParseCancelled)
}

pub fn extract_symbols_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<Symbol>, FSharpParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let mut symbols = Vec::new();
  let line_count = source.lines().count().max(1);

  // File module fallback name
  let file_stem = Path::new(file_path)
    .file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or(file_path)
    .to_string();

  // Scan top-level children for namespace or top-level module declaration
  let mut has_toplevel_scope = false;
  let mut cursor = root.walk();
  for child in root.children(&mut cursor) {
    let kind = child.kind();
    if kind == "namespace_declaration"
      || kind == "namespace_defn"
      || kind == "named_namespace"
      || kind == "namespace"
    {
      has_toplevel_scope = true;
      break;
    }
    if kind == "module_declaration" || kind == "module_defn" || kind == "module" {
      let text = child.utf8_text(source.as_bytes()).unwrap_or("");
      let first_line = text.lines().next().unwrap_or("");
      if !first_line.contains('=') {
        has_toplevel_scope = true;
        break;
      }
    }
  }

  let mut scope_stack = Vec::new();
  if !has_toplevel_scope {
    // Implicit file module scopes everything in the file
    scope_stack.push(file_stem.clone());
  }

  // Recursive traversal
  let mut traverse_cursor = root.walk();
  traverse(
    &mut traverse_cursor,
    source,
    file_path,
    &mut scope_stack,
    &mut symbols,
  );

  // If no namespace or module symbol was registered at the top level, prepend the file module
  if !has_toplevel_scope {
    symbols.insert(
      0,
      Symbol {
        name: file_stem.clone(),
        qualified_name: file_stem.clone(),
        kind: SymbolKind::Module,
        location: Location {
          file_path: file_path.to_string(),
          start_line: 1,
          end_line: line_count,
          start_column: Some(0),
          end_column: Some(0),
        },
      },
    );
  }

  Ok(symbols)
}

fn get_node_name(node: Node<'_>, source: &str) -> Option<String> {
  let kind = node.kind();
  if kind == "wildcard_pattern" {
    return None;
  }

  // Prioritize active patterns
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    let txt = child.utf8_text(source.as_bytes()).unwrap_or("").trim();
    if txt.starts_with("(|") && txt.ends_with("|)") {
      return Some(txt.to_string());
    }
  }

  // Try getting by field name first
  if let Some(name_node) = node.child_by_field_name("name") {
    let txt = name_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
    if !txt.is_empty() && txt != "_" {
      return Some(txt.to_string());
    }
  }

  // Iterate over children to find suitable name nodes
  let kinds = [
    "identifier",
    "dotted_name",
    "type_name",
    "long_identifier",
    "value_declaration_left",
    "class_identifier",
  ];

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    let k = child.kind();
    if kinds.contains(&k) {
      let txt = child.utf8_text(source.as_bytes()).unwrap_or("").trim();
      if !txt.is_empty() && txt != "_" {
        return Some(txt.to_string());
      }
    }
  }

  // Deep search first identifier, skipping RHS of bindings and specific ignored kinds
  if kind == "module_declaration"
    || kind == "module_defn"
    || kind == "namespace_declaration"
    || kind == "namespace_defn"
    || kind == "named_namespace"
    || kind == "namespace"
  {
    return None;
  }

  let mut cursor = node.walk();
  let mut found_eq = false;
  for child in node.children(&mut cursor) {
    let k = child.kind();
    if k == "=" {
      found_eq = true;
    }
    if found_eq {
      continue;
    }
    if k == "attribute_list"
      || k == "attribute"
      || k == "generic_parameter_list"
      || k == "type_annotation"
      || k == "type_argument_list"
    {
      continue;
    }
    if let Some(name) = get_node_name(child, source).filter(|n| n != "_") {
      return Some(name);
    }
  }

  None
}

fn clean_active_pattern_name(name: &str) -> String {
  name.replace(['(', ')'], "").trim().to_string()
}

fn clean_member_name(name: &str) -> String {
  if let Some(dot_idx) = name.find('.') {
    name[dot_idx + 1..].to_string()
  } else {
    name.to_string()
  }
}

fn clean_generic_type_name(name: &str) -> String {
  if let Some(pos) = name.find('<') {
    name[..pos].trim().to_string()
  } else {
    name.to_string()
  }
}

fn collect_comment_byte_ranges(node: Node<'_>, ranges: &mut Vec<(usize, usize)>) {
  let kind = node.kind();
  if kind == "comment"
    || kind == "line_comment"
    || kind == "block_comment"
    || kind.contains("comment")
  {
    ranges.push((node.start_byte(), node.end_byte()));
  }
  let mut cursor = node.walk();
  if cursor.goto_first_child() {
    loop {
      collect_comment_byte_ranges(cursor.node(), ranges);
      if !cursor.goto_next_sibling() {
        break;
      }
    }
  }
}

fn strip_comments_using_tree(node: Node<'_>, source: &str) -> String {
  let mut comment_ranges = Vec::new();
  collect_comment_byte_ranges(node, &mut comment_ranges);

  let start_byte = node.start_byte();
  let mut bytes = node
    .utf8_text(source.as_bytes())
    .unwrap_or("")
    .as_bytes()
    .to_vec();

  for (c_start, c_end) in comment_ranges {
    if c_start >= start_byte && c_end >= c_start {
      let relative_start = c_start - start_byte;
      let relative_end = c_end - start_byte;
      if relative_end <= bytes.len() {
        for b in &mut bytes[relative_start..relative_end] {
          if *b != b'\n' && *b != b'\r' {
            *b = b' ';
          }
        }
      }
    }
  }

  String::from_utf8(bytes).unwrap_or_else(|_| "".to_string())
}

fn traverse(
  cursor: &mut TreeCursor<'_>,
  source: &str,
  file_path: &str,
  scope_stack: &mut Vec<String>,
  symbols: &mut Vec<Symbol>,
) {
  let node = cursor.node();
  let kind = node.kind();
  let mut scope_pushed_count = 0;

  match kind {
    "namespace" | "namespace_declaration" | "namespace_defn" | "named_namespace" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let name = raw_name.trim().to_string();
        if !name.is_empty() {
          let segments: Vec<String> = name.split('.').map(|s| s.to_string()).collect();
          for seg in &segments {
            scope_stack.push(seg.clone());
            scope_pushed_count += 1;
          }

          let qname = scope_stack.join(".");
          let start = node.start_position();
          let end = node.end_position();

          symbols.push(Symbol {
            name,
            qualified_name: qname,
            kind: SymbolKind::Module,
            location: Location {
              file_path: file_path.to_string(),
              start_line: start.row + 1,
              end_line: end.row + 1,
              start_column: Some(start.column),
              end_column: Some(end.column),
            },
          });
        }
      }
    }
    "module_declaration" | "module_defn" | "module" | "nested_module" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let name = raw_name.trim().to_string();
        if !name.is_empty() {
          let segments: Vec<String> = name.split('.').map(|s| s.to_string()).collect();
          for seg in &segments {
            scope_stack.push(seg.clone());
            scope_pushed_count += 1;
          }

          let qname = scope_stack.join(".");
          let start = node.start_position();
          let end = node.end_position();

          symbols.push(Symbol {
            name,
            qualified_name: qname,
            kind: SymbolKind::Module,
            location: Location {
              file_path: file_path.to_string(),
              start_line: start.row + 1,
              end_line: end.row + 1,
              start_column: Some(start.column),
              end_column: Some(end.column),
            },
          });
        }
      }
    }
    "type_definition" | "type_defn" | "type_declaration" | "type" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let name = clean_generic_type_name(raw_name.trim());
        if !name.is_empty() {
          scope_stack.push(name.clone());
          scope_pushed_count += 1;

          let qname = scope_stack.join(".");
          let start = node.start_position();
          let end = node.end_position();

          let clean_text = strip_comments_using_tree(node, source);
          let first_line = clean_text.lines().next().unwrap_or("");
          let sig_part = first_line.split('=').next().unwrap_or("");
          let has_constructor = sig_part.contains('(') && sig_part.contains(')');

          let is_interface = if has_constructor {
            false
          } else {
            let has_interface_keyword =
              clean_text.contains("abstract") || clean_text.contains("interface");
            let implements_interface =
              clean_text.contains("interface") && clean_text.contains("with");
            let has_concrete_members = clean_text.contains("member this.")
              || clean_text.contains("member __.")
              || clean_text.contains("member x.");

            has_interface_keyword && !implements_interface && !has_concrete_members
          };

          let symbol_kind = if is_interface {
            SymbolKind::Interface
          } else {
            SymbolKind::Class
          };

          symbols.push(Symbol {
            name,
            qualified_name: qname,
            kind: symbol_kind,
            location: Location {
              file_path: file_path.to_string(),
              start_line: start.row + 1,
              end_line: end.row + 1,
              start_column: Some(start.column),
              end_column: Some(end.column),
            },
          });
        }
      }
    }
    "function_or_value_defn" | "value_declaration" | "let_binding" | "function_defn" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let mut name = raw_name.trim().to_string();
        if !name.is_empty() {
          if name.starts_with("(|") && name.ends_with("|)") {
            name = clean_active_pattern_name(&name);
          }

          let qname = if scope_stack.is_empty() {
            name.clone()
          } else {
            format!("{}.{}", scope_stack.join("."), name)
          };

          let start = node.start_position();
          let end = node.end_position();

          symbols.push(Symbol {
            name,
            qualified_name: qname,
            kind: SymbolKind::Function,
            location: Location {
              file_path: file_path.to_string(),
              start_line: start.row + 1,
              end_line: end.row + 1,
              start_column: Some(start.column),
              end_column: Some(end.column),
            },
          });
        }
      }
    }
    "member_declaration" | "member_defn" | "member" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let mut name = clean_member_name(raw_name.trim());
        if name.starts_with("(|") && name.ends_with("|)") {
          name = clean_active_pattern_name(&name);
        }
        if !name.is_empty() {
          let qname = if scope_stack.is_empty() {
            name.clone()
          } else {
            format!("{}.{}", scope_stack.join("."), name)
          };

          let start = node.start_position();
          let end = node.end_position();

          symbols.push(Symbol {
            name,
            qualified_name: qname,
            kind: SymbolKind::Method,
            location: Location {
              file_path: file_path.to_string(),
              start_line: start.row + 1,
              end_line: end.row + 1,
              start_column: Some(start.column),
              end_column: Some(end.column),
            },
          });
        }
      }
    }
    _ => {}
  }

  if cursor.goto_first_child() {
    loop {
      traverse(cursor, source, file_path, scope_stack, symbols);
      if !cursor.goto_next_sibling() {
        break;
      }
    }
    cursor.goto_parent();
  }

  for _ in 0..scope_pushed_count {
    scope_stack.pop();
  }
}

pub fn extract_imports_from_source(
  source: &str,
  _file_path: &str,
) -> Result<Vec<ExtractedImport>, FSharpParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();
  let mut imports = Vec::new();
  let mut cursor = root.walk();
  traverse_imports(&mut cursor, source, &mut imports);
  Ok(imports)
}

fn traverse_imports(cursor: &mut TreeCursor<'_>, source: &str, imports: &mut Vec<ExtractedImport>) {
  let node = cursor.node();
  let kind = node.kind();
  if kind == "import_decl" && cursor.goto_first_child() {
    loop {
      let child = cursor.node();
      if child.kind() == "long_identifier" {
        let name = child
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string();
        if !name.is_empty() {
          let start = child.start_position();
          let end = child.end_position();
          imports.push(ExtractedImport {
            target_qualified_name: name,
            start_line: start.row + 1,
            start_column: start.column,
            end_line: end.row + 1,
            end_column: end.column,
          });
        }
      }
      if !cursor.goto_next_sibling() {
        break;
      }
    }
    cursor.goto_parent();
  }

  if cursor.goto_first_child() {
    loop {
      traverse_imports(cursor, source, imports);
      if !cursor.goto_next_sibling() {
        break;
      }
    }
    cursor.goto_parent();
  }
}

pub fn extract_calls_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedCall>, FSharpParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let file_stem = Path::new(file_path)
    .file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or(file_path)
    .to_string();

  // Scan top-level children for namespace or top-level module declaration
  let mut has_toplevel_scope = false;
  let mut cursor = root.walk();
  for child in root.children(&mut cursor) {
    let kind = child.kind();
    if kind == "namespace_declaration"
      || kind == "namespace_defn"
      || kind == "named_namespace"
      || kind == "namespace"
    {
      has_toplevel_scope = true;
      break;
    }
    if kind == "module_declaration" || kind == "module_defn" || kind == "module" {
      let text = child.utf8_text(source.as_bytes()).unwrap_or("");
      let first_line = text.lines().next().unwrap_or("");
      if !first_line.contains('=') {
        has_toplevel_scope = true;
        break;
      }
    }
  }

  let mut scope_stack = Vec::new();
  if !has_toplevel_scope {
    scope_stack.push(file_stem.clone());
  }

  let mut calls = Vec::new();
  let mut traverse_cursor = root.walk();
  traverse_calls(
    &mut traverse_cursor,
    source,
    &mut scope_stack,
    &mut calls,
    false,
  );
  Ok(calls)
}

fn traverse_calls(
  cursor: &mut TreeCursor<'_>,
  source: &str,
  scope_stack: &mut Vec<String>,
  calls: &mut Vec<ExtractedCall>,
  in_ignored_context: bool,
) {
  let node = cursor.node();
  let kind = node.kind();
  let mut scope_pushed_count = 0;

  match kind {
    "namespace" | "namespace_declaration" | "namespace_defn" | "named_namespace" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let name = raw_name.trim().to_string();
        if !name.is_empty() {
          let segments: Vec<String> = name.split('.').map(|s| s.to_string()).collect();
          for seg in &segments {
            scope_stack.push(seg.clone());
            scope_pushed_count += 1;
          }
        }
      }
    }
    "module_declaration" | "module_defn" | "module" | "nested_module" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let name = raw_name.trim().to_string();
        if !name.is_empty() {
          let segments: Vec<String> = name.split('.').map(|s| s.to_string()).collect();
          for seg in &segments {
            scope_stack.push(seg.clone());
            scope_pushed_count += 1;
          }
        }
      }
    }
    "type_definition" | "type_defn" | "type_declaration" | "type" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let name = clean_generic_type_name(raw_name.trim());
        if !name.is_empty() {
          scope_stack.push(name);
          scope_pushed_count += 1;
        }
      }
    }
    "function_or_value_defn" | "value_declaration" | "let_binding" | "function_defn" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let mut name = raw_name.trim().to_string();
        if !name.is_empty() {
          if name.starts_with("(|") && name.ends_with("|)") {
            name = clean_active_pattern_name(&name);
          }
          scope_stack.push(name);
          scope_pushed_count += 1;
        }
      }
    }
    "member_declaration" | "member_defn" | "member" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let mut name = clean_member_name(raw_name.trim());
        if name.starts_with("(|") && name.ends_with("|)") {
          name = clean_active_pattern_name(&name);
        }
        if !name.is_empty() {
          scope_stack.push(name);
          scope_pushed_count += 1;
        }
      }
    }
    _ => {}
  }

  let is_ignored_type = kind == "value_declaration_left"
    || kind == "function_declaration_left"
    || kind == "type_name"
    || kind == "namespace"
    || kind == "namespace_declaration"
    || kind == "namespace_defn"
    || kind == "named_namespace"
    || kind == "module_declaration"
    || kind == "module_defn"
    || kind == "module"
    || kind == "nested_module"
    || kind == "import_decl"
    || kind == "record_field"
    || kind == "union_type_case"
    || kind == "simple_type"
    || kind == "typed_pattern"
    || kind == "object_expression"
    || kind.contains("comment");

  let next_ignored_context = in_ignored_context || is_ignored_type;

  if (kind == "long_identifier" || kind == "long_identifier_or_op") && !next_ignored_context {
    let raw_name = node
      .utf8_text(source.as_bytes())
      .unwrap_or("")
      .trim()
      .to_string();
    if !raw_name.is_empty() && raw_name != "_" {
      let source_symbol_qualified_name = scope_stack.join(".");
      let start = node.start_position();
      let end = node.end_position();

      calls.push(ExtractedCall {
        source_symbol_qualified_name,
        target_name: raw_name,
        is_self_call: false,
        method_name: None,
        start_line: start.row + 1,
        start_column: start.column,
        end_line: end.row + 1,
        end_column: end.column,
      });
    }
  }

  if cursor.goto_first_child() {
    loop {
      traverse_calls(cursor, source, scope_stack, calls, next_ignored_context);
      if !cursor.goto_next_sibling() {
        break;
      }
    }
    cursor.goto_parent();
  }

  for _ in 0..scope_pushed_count {
    scope_stack.pop();
  }
}

pub fn extract_type_references_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedTypeReference>, FSharpParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let file_stem = Path::new(file_path)
    .file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or(file_path)
    .to_string();

  // Scan top-level children for namespace or top-level module declaration
  let mut has_toplevel_scope = false;
  let mut cursor = root.walk();
  for child in root.children(&mut cursor) {
    let kind = child.kind();
    if kind == "namespace_declaration"
      || kind == "namespace_defn"
      || kind == "named_namespace"
      || kind == "namespace"
    {
      has_toplevel_scope = true;
      break;
    }
    if kind == "module_declaration" || kind == "module_defn" || kind == "module" {
      let text = child.utf8_text(source.as_bytes()).unwrap_or("");
      let first_line = text.lines().next().unwrap_or("");
      if !first_line.contains('=') {
        has_toplevel_scope = true;
        break;
      }
    }
  }

  let mut scope_stack = Vec::new();
  if !has_toplevel_scope {
    scope_stack.push(file_stem.clone());
  }

  let mut type_refs = Vec::new();
  let mut traverse_cursor = root.walk();
  traverse_type_references(
    &mut traverse_cursor,
    source,
    &mut scope_stack,
    &mut type_refs,
  );
  Ok(type_refs)
}

fn is_primitive_type(name: &str) -> bool {
  let lower = name.to_lowercase();
  let primitives = [
    "int",
    "int32",
    "uint32",
    "int64",
    "uint64",
    "int16",
    "uint16",
    "byte",
    "sbyte",
    "float",
    "double",
    "decimal",
    "bool",
    "string",
    "char",
    "unit",
    "obj",
    "object",
    "exn",
    "exception",
    "nativeint",
    "unativeint",
    "float32",
    "single",
  ];
  primitives.contains(&lower.as_str())
}

fn traverse_type_references(
  cursor: &mut TreeCursor<'_>,
  source: &str,
  scope_stack: &mut Vec<String>,
  type_refs: &mut Vec<ExtractedTypeReference>,
) {
  let node = cursor.node();
  let kind = node.kind();
  let mut scope_pushed_count = 0;

  match kind {
    "namespace" | "namespace_declaration" | "namespace_defn" | "named_namespace" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let name = raw_name.trim().to_string();
        if !name.is_empty() {
          let segments: Vec<String> = name.split('.').map(|s| s.to_string()).collect();
          for seg in &segments {
            scope_stack.push(seg.clone());
            scope_pushed_count += 1;
          }
        }
      }
    }
    "module_declaration" | "module_defn" | "module" | "nested_module" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let name = raw_name.trim().to_string();
        if !name.is_empty() {
          let segments: Vec<String> = name.split('.').map(|s| s.to_string()).collect();
          for seg in &segments {
            scope_stack.push(seg.clone());
            scope_pushed_count += 1;
          }
        }
      }
    }
    "type_definition" | "type_defn" | "type_declaration" | "type" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let name = clean_generic_type_name(raw_name.trim());
        if !name.is_empty() {
          scope_stack.push(name);
          scope_pushed_count += 1;
        }
      }
    }
    "function_or_value_defn" | "value_declaration" | "let_binding" | "function_defn" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let mut name = raw_name.trim().to_string();
        if !name.is_empty() {
          if name.starts_with("(|") && name.ends_with("|)") {
            name = clean_active_pattern_name(&name);
          }
          scope_stack.push(name);
          scope_pushed_count += 1;
        }
      }
    }
    "member_declaration" | "member_defn" | "member" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let mut name = clean_member_name(raw_name.trim());
        if name.starts_with("(|") && name.ends_with("|)") {
          name = clean_active_pattern_name(&name);
        }
        if !name.is_empty() {
          scope_stack.push(name);
          scope_pushed_count += 1;
        }
      }
    }
    _ => {}
  }

  if kind == "simple_type" && cursor.goto_first_child() {
    loop {
      let child = cursor.node();
      if child.kind() == "long_identifier" {
        let raw_name = child
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string();
        if !raw_name.is_empty() && !is_primitive_type(&raw_name) {
          let source_symbol_qualified_name = scope_stack.join(".");
          let start = child.start_position();
          let end = child.end_position();
          type_refs.push(ExtractedTypeReference {
            source_symbol_qualified_name,
            target_type_name: raw_name,
            start_line: start.row + 1,
            start_column: start.column,
            end_line: end.row + 1,
            end_column: end.column,
          });
        }
      }
      if !cursor.goto_next_sibling() {
        break;
      }
    }
    cursor.goto_parent();
  }

  if kind == "object_expression" && cursor.goto_first_child() {
    loop {
      let child = cursor.node();
      let ck = child.kind();
      if ck == "long_identifier" || ck == "long_identifier_or_op" {
        let raw_name = child
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string();
        if !raw_name.is_empty() && !is_primitive_type(&raw_name) {
          let source_symbol_qualified_name = scope_stack.join(".");
          let start = child.start_position();
          let end = child.end_position();
          type_refs.push(ExtractedTypeReference {
            source_symbol_qualified_name,
            target_type_name: raw_name,
            start_line: start.row + 1,
            start_column: start.column,
            end_line: end.row + 1,
            end_column: end.column,
          });
        }
      }
      if !cursor.goto_next_sibling() {
        break;
      }
    }
    cursor.goto_parent();
  }

  if cursor.goto_first_child() {
    loop {
      traverse_type_references(cursor, source, scope_stack, type_refs);
      if !cursor.goto_next_sibling() {
        break;
      }
    }
    cursor.goto_parent();
  }

  for _ in 0..scope_pushed_count {
    scope_stack.pop();
  }
}

fn find_quoted_string_in_node(node: Node<'_>, source: &str) -> Option<String> {
  let kind = node.kind();
  if kind == "string" || kind == "string_literal" || kind == "literal" || kind == "const" {
    let text = node.utf8_text(source.as_bytes()).unwrap_or("").trim();
    if (text.starts_with('"') && text.ends_with('"'))
      || (text.starts_with('\'') && text.ends_with('\''))
    {
      if text.len() >= 2 {
        return Some(text[1..text.len() - 1].to_string());
      }
    }
  }
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if let Some(s) = find_quoted_string_in_node(child, source) {
      return Some(s);
    }
  }
  None
}

fn extract_from_attributes_node(node: Node<'_>, source: &str, attrs: &mut Vec<String>) {
  let mut attr_cursor = node.walk();
  for attr in node.children(&mut attr_cursor) {
    if attr.kind() == "attribute" {
      let mut sub_cursor = attr.walk();
      let attr_name = attr
        .children(&mut sub_cursor)
        .find(|c| {
          c.kind() == "simple_type" || c.kind() == "long_identifier" || c.kind() == "identifier"
        })
        .and_then(|c| {
          if c.kind() == "simple_type" {
            let mut type_cursor = c.walk();
            c.children(&mut type_cursor)
              .find(|gc| gc.kind() == "long_identifier" || gc.kind() == "identifier")
              .and_then(|gc| gc.utf8_text(source.as_bytes()).ok())
          } else {
            c.utf8_text(source.as_bytes()).ok()
          }
        })
        .map(|t| t.trim().to_string());
      if let Some(name) = attr_name {
        if !name.is_empty() {
          attrs.push(name);
        }
      }
    }
  }
}

fn get_node_attributes(node: Node<'_>, source: &str) -> Vec<String> {
  let mut attrs = Vec::new();

  if let Some(parent) = node.parent() {
    if parent.kind() == "declaration_expression" {
      let mut cursor = parent.walk();
      for child in parent.children(&mut cursor) {
        if child.kind() == "attributes" {
          extract_from_attributes_node(child, source, &mut attrs);
        }
      }
    }
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if child.kind() == "attributes" || child.kind() == "attribute_list" {
      extract_from_attributes_node(child, source, &mut attrs);
    }
  }

  attrs
}

fn is_test_attribute(attr: &str) -> bool {
  let parts: Vec<&str> = attr.split('.').collect();
  let last_part = parts.last().unwrap_or(&attr);
  let mut lower = last_part.to_lowercase();
  if let Some(stripped) = lower.strip_suffix("attribute") {
    lower = stripped.to_string();
  }
  lower == "fact"
    || lower == "theory"
    || lower == "test"
    || lower == "testcase"
    || lower == "property"
}

fn get_call_string_arg(ident_node: Node<'_>, source: &str) -> Option<String> {
  if let Some(parent) = ident_node.parent() {
    if let Some(s) = find_quoted_string_in_node(parent, source) {
      return Some(s);
    }
    if let Some(grandparent) = parent.parent() {
      if let Some(s) = find_quoted_string_in_node(grandparent, source) {
        return Some(s);
      }
    }
  }
  None
}

fn collect_identifiers_in_param_list(node: Node<'_>, source: &str, parameters: &mut Vec<String>) {
  let kind = node.kind();
  if kind == "simple_type" || kind == "type" || kind == "type_annotation" {
    return;
  }

  if kind == "identifier" {
    let name = node
      .utf8_text(source.as_bytes())
      .unwrap_or("")
      .trim()
      .to_string();
    if !name.is_empty() && name != "this" && name != "self" && name != "()" {
      if !parameters.contains(&name) {
        parameters.push(name);
      }
    }
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    collect_identifiers_in_param_list(child, source, parameters);
  }
}

fn collect_parameters_from_node(node: Node<'_>, source: &str, parameters: &mut Vec<String>) {
  let kind = node.kind();
  if kind == "paren_pattern"
    || kind == "argument_patterns"
    || kind == "parameter"
    || kind == "argument"
  {
    collect_identifiers_in_param_list(node, source, parameters);
    return;
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if child.kind() == "=" {
      break;
    }
    collect_parameters_from_node(child, source, parameters);
  }
}

fn traverse_tests(
  cursor: &mut TreeCursor<'_>,
  source: &str,
  scope_stack: &mut Vec<String>,
  tests: &mut Vec<ExtractedTest>,
) {
  let node = cursor.node();
  let kind = node.kind();
  let mut scope_pushed_count = 0;

  match kind {
    "namespace" | "namespace_declaration" | "namespace_defn" | "named_namespace" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let name = raw_name.trim().to_string();
        if !name.is_empty() {
          let segments: Vec<String> = name.split('.').map(|s| s.to_string()).collect();
          for seg in &segments {
            scope_stack.push(seg.clone());
            scope_pushed_count += 1;
          }
        }
      }
    }
    "module_declaration" | "module_defn" | "module" | "nested_module" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let name = raw_name.trim().to_string();
        if !name.is_empty() {
          let segments: Vec<String> = name.split('.').map(|s| s.to_string()).collect();
          for seg in &segments {
            scope_stack.push(seg.clone());
            scope_pushed_count += 1;
          }
        }
      }
    }
    "type_definition" | "type_defn" | "type_declaration" | "type" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let name = clean_generic_type_name(raw_name.trim());
        if !name.is_empty() {
          scope_stack.push(name);
          scope_pushed_count += 1;
        }
      }
    }
    "function_or_value_defn" | "value_declaration" | "let_binding" | "function_defn" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let name = raw_name.trim().to_string();
        if !name.is_empty() {
          scope_stack.push(name);
          scope_pushed_count += 1;
        }
      }
    }
    "member_declaration" | "member_defn" | "member" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let name = clean_member_name(raw_name.trim());
        if !name.is_empty() {
          scope_stack.push(name);
          scope_pushed_count += 1;
        }
      }
    }
    _ => {}
  }

  let is_test_candidate = kind == "function_or_value_defn"
    || kind == "value_declaration"
    || kind == "let_binding"
    || kind == "function_defn"
    || kind == "member_declaration"
    || kind == "member_defn"
    || kind == "member"
    || kind == "type_definition"
    || kind == "type_defn"
    || kind == "type_declaration"
    || kind == "type";

  if is_test_candidate {
    let attrs = get_node_attributes(node, source);
    let has_test_attr = attrs.iter().any(|a| is_test_attribute(a));
    if has_test_attr {
      let is_parametrized = attrs.iter().any(|a| {
        let lower = a.to_lowercase();
        lower == "theory" || lower == "testcase" || lower == "property"
      });

      let test_kind = if kind.contains("type") {
        ExtractedTestKind::Class
      } else {
        ExtractedTestKind::Function
      };

      if let Some(raw_name) = get_node_name(node, source) {
        let mut name = raw_name.trim().to_string();
        if name.starts_with("(|") && name.ends_with("|)") {
          name = clean_active_pattern_name(&name);
        }
        name = clean_member_name(&name);

        if !name.is_empty() {
          let qname = scope_stack.join(".");
          let start = node.start_position();
          let end = node.end_position();

          let mut parameters = Vec::new();
          collect_parameters_from_node(node, source, &mut parameters);

          tests.push(ExtractedTest {
            name,
            qualified_name: qname,
            kind: test_kind,
            is_parametrized,
            parameters,
            start_line: start.row + 1,
            start_column: start.column,
            end_line: end.row + 1,
            end_column: end.column,
          });
        }
      }
    }
  }

  if kind == "long_identifier" || kind == "long_identifier_or_op" {
    let raw_name = node
      .utf8_text(source.as_bytes())
      .unwrap_or("")
      .trim()
      .to_string();

    let is_expecto_fn =
      raw_name == "testCase" || raw_name == "testCaseAsync" || raw_name == "testProperty";
    let is_expecto_list = raw_name == "testList";

    if is_expecto_fn || is_expecto_list {
      if let Some(test_name) = get_call_string_arg(node, source) {
        let parent_qname = scope_stack.join(".");
        let qualified_name = if parent_qname.is_empty() {
          test_name.clone()
        } else {
          format!("{parent_qname}.{test_name}")
        };

        let start = node.start_position();
        let end = node.end_position();
        let is_parametrized = raw_name == "testProperty";

        let test_kind = if is_expecto_list {
          ExtractedTestKind::Class
        } else {
          ExtractedTestKind::Function
        };

        tests.push(ExtractedTest {
          name: test_name,
          qualified_name,
          kind: test_kind,
          is_parametrized,
          parameters: Vec::new(),
          start_line: start.row + 1,
          start_column: start.column,
          end_line: end.row + 1,
          end_column: end.column,
        });
      }
    }
  }

  if cursor.goto_first_child() {
    loop {
      traverse_tests(cursor, source, scope_stack, tests);
      if !cursor.goto_next_sibling() {
        break;
      }
    }
    cursor.goto_parent();
  }

  for _ in 0..scope_pushed_count {
    scope_stack.pop();
  }
}

pub fn extract_tests_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedTest>, FSharpParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let mut tests = Vec::new();

  let file_stem = Path::new(file_path)
    .file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or(file_path)
    .to_string();

  let mut has_toplevel_scope = false;
  let mut cursor = root.walk();
  for child in root.children(&mut cursor) {
    let kind = child.kind();
    if kind == "namespace_declaration"
      || kind == "namespace_defn"
      || kind == "named_namespace"
      || kind == "namespace"
    {
      has_toplevel_scope = true;
      break;
    }
    if kind == "module_declaration" || kind == "module_defn" || kind == "module" {
      let text = child.utf8_text(source.as_bytes()).unwrap_or("");
      let first_line = text.lines().next().unwrap_or("");
      if !first_line.contains('=') {
        has_toplevel_scope = true;
        break;
      }
    }
  }

  let mut scope_stack = Vec::new();
  if !has_toplevel_scope {
    scope_stack.push(file_stem.clone());
  }

  let mut traverse_cursor = root.walk();
  traverse_tests(&mut traverse_cursor, source, &mut scope_stack, &mut tests);

  Ok(tests)
}

fn determine_giraffe_method(node: Node<'_>, source: &str) -> String {
  let methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD"];
  if let Some(parent) = node.parent() {
    let mut cursor = parent.walk();
    for child in parent.children(&mut cursor) {
      if child.kind() == "long_identifier" || child.kind() == "identifier" {
        let name = child.utf8_text(source.as_bytes()).unwrap_or("").trim();
        let upper = name.to_uppercase();
        if methods.contains(&upper.as_str()) {
          return upper;
        }
      }
    }
    if let Some(grandparent) = parent.parent() {
      let mut cursor = grandparent.walk();
      for child in grandparent.children(&mut cursor) {
        if child.kind() == "long_identifier" || child.kind() == "identifier" {
          let name = child.utf8_text(source.as_bytes()).unwrap_or("").trim();
          let upper = name.to_uppercase();
          if methods.contains(&upper.as_str()) {
            return upper;
          }
        }
      }
    }
  }
  "GET".to_string()
}

fn get_attribute_string_arg(attr_node: Node<'_>, source: &str) -> Option<String> {
  find_quoted_string_in_node(attr_node, source)
}

fn traverse_routes(
  cursor: &mut TreeCursor<'_>,
  source: &str,
  scope_stack: &mut Vec<String>,
  routes: &mut Vec<ExtractedFSharpRoute>,
) {
  let node = cursor.node();
  let kind = node.kind();
  let mut scope_pushed_count = 0;

  match kind {
    "namespace" | "namespace_declaration" | "namespace_defn" | "named_namespace" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let name = raw_name.trim().to_string();
        if !name.is_empty() {
          let segments: Vec<String> = name.split('.').map(|s| s.to_string()).collect();
          for seg in &segments {
            scope_stack.push(seg.clone());
            scope_pushed_count += 1;
          }
        }
      }
    }
    "module_declaration" | "module_defn" | "module" | "nested_module" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let name = raw_name.trim().to_string();
        if !name.is_empty() {
          let segments: Vec<String> = name.split('.').map(|s| s.to_string()).collect();
          for seg in &segments {
            scope_stack.push(seg.clone());
            scope_pushed_count += 1;
          }
        }
      }
    }
    "type_definition" | "type_defn" | "type_declaration" | "type" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let name = clean_generic_type_name(raw_name.trim());
        if !name.is_empty() {
          scope_stack.push(name);
          scope_pushed_count += 1;
        }
      }
    }
    "function_or_value_defn" | "value_declaration" | "let_binding" | "function_defn" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let name = raw_name.trim().to_string();
        if !name.is_empty() {
          scope_stack.push(name);
          scope_pushed_count += 1;
        }
      }
    }
    "member_declaration" | "member_defn" | "member" => {
      if let Some(raw_name) = get_node_name(node, source) {
        let name = clean_member_name(raw_name.trim());
        if !name.is_empty() {
          scope_stack.push(name);
          scope_pushed_count += 1;
        }
      }
    }
    _ => {}
  }

  if kind == "long_identifier" || kind == "long_identifier_or_op" {
    let raw_name = node
      .utf8_text(source.as_bytes())
      .unwrap_or("")
      .trim()
      .to_string();

    if raw_name == "route" || raw_name == "routef" {
      if let Some(path) = get_call_string_arg(node, source) {
        let method = determine_giraffe_method(node, source);
        let start = node.start_position();
        let end = node.end_position();
        let handler_name = scope_stack
          .last()
          .cloned()
          .unwrap_or_else(|| "webApp".to_string());
        let qualified_name = scope_stack.join(".");

        routes.push(ExtractedFSharpRoute {
          handler_name,
          qualified_name,
          method,
          path,
          response_model: None,
          start_line: start.row + 1,
          start_column: start.column,
          end_line: end.row + 1,
          end_column: end.column,
        });
      }
    }

    let is_saturn_method = [
      "get", "post", "put", "delete", "patch", "options", "head", "GET", "POST", "PUT", "DELETE",
      "PATCH", "OPTIONS", "HEAD",
    ]
    .contains(&raw_name.as_str());
    if is_saturn_method {
      if let Some(path) = get_call_string_arg(node, source) {
        if path.starts_with('/') {
          let method = raw_name.to_uppercase();
          let start = node.start_position();
          let end = node.end_position();

          let mut handler_name = scope_stack
            .last()
            .cloned()
            .unwrap_or_else(|| "router".to_string());
          let mut qualified_name = scope_stack.join(".");

          // Try to extract Saturn handler name from parent application expression siblings
          if let Some(parent) = node.parent() {
            let mut cursor = parent.walk();
            for child in parent.children(&mut cursor) {
              if child.id() != node.id()
                && (child.kind() == "long_identifier" || child.kind() == "identifier")
              {
                let name = child
                  .utf8_text(source.as_bytes())
                  .unwrap_or("")
                  .trim()
                  .to_string();
                if !name.is_empty()
                  && name != "get"
                  && name != "post"
                  && name != "put"
                  && name != "delete"
                {
                  handler_name = name.clone();
                  if scope_stack.len() >= 2 {
                    qualified_name = format!(
                      "{}.{}",
                      scope_stack[..scope_stack.len() - 1].join("."),
                      name
                    );
                  } else {
                    qualified_name = name;
                  }
                  break;
                }
              }
            }
          }

          routes.push(ExtractedFSharpRoute {
            handler_name,
            qualified_name,
            method,
            path,
            response_model: None,
            start_line: start.row + 1,
            start_column: start.column,
            end_line: end.row + 1,
            end_column: end.column,
          });
        }
      }
    }
  }

  if kind == "attribute" {
    if let Some(ident_node) = node.child_by_field_name("name").or_else(|| {
      let mut sub_cursor = node.walk();
      node
        .children(&mut sub_cursor)
        .find(|c| c.kind() == "long_identifier" || c.kind() == "identifier")
    }) {
      let attr_name = ident_node
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .trim()
        .to_string();
      let lower = attr_name.to_lowercase();
      let is_aspnet_route = lower.starts_with("http") || lower == "route";

      if is_aspnet_route {
        let method = if lower.contains("get") {
          "GET".to_string()
        } else if lower.contains("post") {
          "POST".to_string()
        } else if lower.contains("put") {
          "PUT".to_string()
        } else if lower.contains("delete") {
          "DELETE".to_string()
        } else if lower.contains("patch") {
          "PATCH".to_string()
        } else if lower.contains("options") {
          "OPTIONS".to_string()
        } else if lower.contains("head") {
          "HEAD".to_string()
        } else {
          "GET".to_string()
        };

        if let Some(path) = get_attribute_string_arg(node, source) {
          let start = node.start_position();
          let end = node.end_position();
          let handler_name = scope_stack
            .last()
            .cloned()
            .unwrap_or_else(|| "handler".to_string());
          let qualified_name = scope_stack.join(".");

          routes.push(ExtractedFSharpRoute {
            handler_name,
            qualified_name,
            method,
            path,
            response_model: None,
            start_line: start.row + 1,
            start_column: start.column,
            end_line: end.row + 1,
            end_column: end.column,
          });
        }
      }
    }
  }

  if cursor.goto_first_child() {
    loop {
      traverse_routes(cursor, source, scope_stack, routes);
      if !cursor.goto_next_sibling() {
        break;
      }
    }
    cursor.goto_parent();
  }

  for _ in 0..scope_pushed_count {
    scope_stack.pop();
  }
}

pub fn extract_routes_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedFSharpRoute>, FSharpParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let mut routes = Vec::new();

  let file_stem = Path::new(file_path)
    .file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or(file_path)
    .to_string();

  let mut has_toplevel_scope = false;
  let mut cursor = root.walk();
  for child in root.children(&mut cursor) {
    let kind = child.kind();
    if kind == "namespace_declaration"
      || kind == "namespace_defn"
      || kind == "named_namespace"
      || kind == "namespace"
    {
      has_toplevel_scope = true;
      break;
    }
    if kind == "module_declaration" || kind == "module_defn" || kind == "module" {
      let text = child.utf8_text(source.as_bytes()).unwrap_or("");
      let first_line = text.lines().next().unwrap_or("");
      if !first_line.contains('=') {
        has_toplevel_scope = true;
        break;
      }
    }
  }

  let mut scope_stack = Vec::new();
  if !has_toplevel_scope {
    scope_stack.push(file_stem.clone());
  }

  let mut traverse_cursor = root.walk();
  traverse_routes(&mut traverse_cursor, source, &mut scope_stack, &mut routes);

  Ok(routes)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_extract_fsharp_symbols() {
    let source = r#"namespace MyCompany.Core

type PatientId = PatientId of string

type Patient = {
    Id: PatientId
    Name: string
    Age: int
}

module PatientValidation =
    let validatePatient (patient: Patient) = true
"#;
    let symbols = extract_symbols_from_source(source, "src/patient.fs").unwrap();

    assert!(
      symbols
        .iter()
        .any(|s| s.qualified_name == "MyCompany.Core" && s.kind == SymbolKind::Module)
    );
    assert!(
      symbols
        .iter()
        .any(|s| s.qualified_name == "MyCompany.Core.PatientId" && s.kind == SymbolKind::Class)
    );
    assert!(
      symbols
        .iter()
        .any(|s| s.qualified_name == "MyCompany.Core.Patient" && s.kind == SymbolKind::Class)
    );
    assert!(symbols.iter().any(
      |s| s.qualified_name == "MyCompany.Core.PatientValidation" && s.kind == SymbolKind::Module
    ));
    assert!(symbols.iter().any(|s| s.qualified_name
      == "MyCompany.Core.PatientValidation.validatePatient"
      && s.kind == SymbolKind::Function));
  }

  #[test]
  fn test_extract_fsharp_symbols_edge_cases() {
    let source = r#"namespace MyCompany.Core

// Comment with abstract or interface
type PatientValidator(version: string) =
    inherit BaseValidator()
    interface IValidator with
        member this.Validate(patient) = true

type IMarker = interface end

type Container<'T> = { Value: 'T }

[<Struct>]
type MyPoint = { X: float; Y: float }

let _ = sideEffect()
"#;
    let symbols = extract_symbols_from_source(source, "src/patient.fs").unwrap();

    // 1. PatientValidator must be classified as Class, and version parameter constructor does not break it
    let validator = symbols
      .iter()
      .find(|s| s.name == "PatientValidator")
      .unwrap();
    assert_eq!(validator.kind, SymbolKind::Class);

    // 2. IMarker must be classified as Interface
    let marker = symbols.iter().find(|s| s.name == "IMarker").unwrap();
    assert_eq!(marker.kind, SymbolKind::Interface);

    // 3. Container<'T> generic type must be cleaned to "Container"
    let container = symbols.iter().find(|s| s.name == "Container").unwrap();
    assert_eq!(container.kind, SymbolKind::Class);

    // 4. Attributed type MyPoint must be named "MyPoint", not "Struct"
    let mypoint = symbols.iter().find(|s| s.name == "MyPoint").unwrap();
    assert_eq!(mypoint.kind, SymbolKind::Class);

    // 5. Wildcard binding let _ = sideEffect() must NOT create a symbol for sideEffect or _
    assert!(
      !symbols
        .iter()
        .any(|s| s.name == "sideEffect" || s.name == "_")
    );
  }

  #[test]
  fn test_extract_fsharp_imports() {
    let source = r#"
open System
open MyCompany.Core.PatientValidation
"#;
    let imports = extract_imports_from_source(source, "src/main.fs").unwrap();
    assert_eq!(imports.len(), 2);
    assert_eq!(imports[0].target_qualified_name, "System");
    assert_eq!(
      imports[1].target_qualified_name,
      "MyCompany.Core.PatientValidation"
    );
  }

  #[test]
  fn test_extract_fsharp_calls() {
    let source = r#"
let sum x y = x + y
let pipelineResult = 5 |> sum 10 |> printfn "%d"
let composed = sum 1 >> sum 2
async {
    printfn "Async computation expression"
}
"#;
    let calls = extract_calls_from_source(source, "src/main.fs").unwrap();
    assert!(calls.iter().any(|c| c.target_name == "sum"));
    assert!(calls.iter().any(|c| c.target_name == "printfn"));
    assert!(calls.iter().any(|c| c.target_name == "async"));
  }

  #[test]
  fn test_extract_fsharp_type_references() {
    let source = r#"
type PatientValidator() =
    inherit BaseValidator()
    interface IValidator with
        member this.Validate(patient: Patient) = true
let objExpr = { new System.IDisposable with member this.Dispose() = () }
"#;
    let type_refs = extract_type_references_from_source(source, "src/main.fs").unwrap();
    assert!(
      type_refs
        .iter()
        .any(|tr| tr.target_type_name == "BaseValidator")
    );
    assert!(
      type_refs
        .iter()
        .any(|tr| tr.target_type_name == "IValidator")
    );
    assert!(type_refs.iter().any(|tr| tr.target_type_name == "Patient"));
    assert!(
      type_refs
        .iter()
        .any(|tr| tr.target_type_name == "System.IDisposable")
    );
  }

  #[test]
  fn test_extract_fsharp_tests() {
    let source = r#"
namespace MyCompany.Tests

open Xunit
open NUnit.Framework
open Expecto

[<Fact>]
let my_xunit_test () =
    Assert.Equal(1, 1)

type MyNUnitTestClass() =
    [<Test>]
    member this.MyTestMethod(x: int) =
        Assert.Pass()

let expectoTests =
    testList "Expecto Suite" [
        testCase "expecto 1" (fun () -> ())
        testProperty "expecto 2" (fun x -> x = x)
    ]
"#;
    let tests = extract_tests_from_source(source, "tests/my_test.fs").unwrap();

    // 1. Verify xUnit test function
    let xunit = tests.iter().find(|t| t.name == "my_xunit_test").unwrap();
    assert_eq!(xunit.qualified_name, "MyCompany.Tests.my_xunit_test");
    assert_eq!(xunit.kind, ExtractedTestKind::Function);
    assert!(!xunit.is_parametrized);

    // 2. Verify NUnit member test function
    let nunit = tests.iter().find(|t| t.name == "MyTestMethod").unwrap();
    assert_eq!(
      nunit.qualified_name,
      "MyCompany.Tests.MyNUnitTestClass.MyTestMethod"
    );
    assert_eq!(nunit.kind, ExtractedTestKind::Function);
    assert_eq!(nunit.parameters, vec!["x"]);

    // 3. Verify Expecto test suite (Class)
    let expecto_suite = tests.iter().find(|t| t.name == "Expecto Suite").unwrap();
    assert_eq!(
      expecto_suite.qualified_name,
      "MyCompany.Tests.expectoTests.Expecto Suite"
    );
    assert_eq!(expecto_suite.kind, ExtractedTestKind::Class);

    // 4. Verify Expecto test case (Function)
    let expecto_case = tests.iter().find(|t| t.name == "expecto 1").unwrap();
    assert_eq!(
      expecto_case.qualified_name,
      "MyCompany.Tests.expectoTests.expecto 1"
    );
    assert_eq!(expecto_case.kind, ExtractedTestKind::Function);
  }

  #[test]
  fn test_extract_fsharp_routes() {
    let source = r#"
namespace MyCompany.Web

open Giraffe
open Saturn

let giraffeApp =
    choose [
        GET >=> route "/api/items" >=> text "items"
        POST >=> routef "/api/item/%d" (fun id -> text "item")
    ]

let saturnApp = router {
    get "/saturn/get" indexHandler
    post "/saturn/post" postHandler
}
"#;
    let routes = extract_routes_from_source(source, "src/api.fs").unwrap();

    // 1. Giraffe routes
    let get_route = routes.iter().find(|r| r.path == "/api/items").unwrap();
    assert_eq!(get_route.method, "GET");
    assert_eq!(get_route.qualified_name, "MyCompany.Web.giraffeApp");

    let post_route = routes.iter().find(|r| r.path == "/api/item/%d").unwrap();
    assert_eq!(post_route.method, "POST");
    assert_eq!(post_route.qualified_name, "MyCompany.Web.giraffeApp");

    // 2. Saturn routes
    let saturn_get = routes.iter().find(|r| r.path == "/saturn/get").unwrap();
    assert_eq!(saturn_get.method, "GET");

    let saturn_post = routes.iter().find(|r| r.path == "/saturn/post").unwrap();
    assert_eq!(saturn_post.method, "POST");
  }
}
