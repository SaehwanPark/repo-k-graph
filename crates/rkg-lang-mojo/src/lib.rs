#![allow(clippy::collapsible_if)]
use std::fmt::{Display, Formatter};
use std::path::Path;

use rkg_core::{Location, Symbol, SymbolKind};
use tree_sitter::{Node, Parser};

pub const CRATE_NAME: &str = "rkg-lang-mojo";

unsafe extern "C" {
  pub fn tree_sitter_mojo() -> tree_sitter::Language;
}

pub fn language() -> tree_sitter::Language {
  unsafe { tree_sitter_mojo() }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MojoParseError {
  UnsupportedLanguage(String),
  ParseCancelled,
}

impl Display for MojoParseError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      MojoParseError::UnsupportedLanguage(message) => {
        write!(f, "failed to initialize mojo parser: {message}")
      }
      MojoParseError::ParseCancelled => write!(f, "mojo parsing was cancelled"),
    }
  }
}

impl std::error::Error for MojoParseError {}

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

fn parse_to_tree(source: &str) -> Result<tree_sitter::Tree, MojoParseError> {
  let mut parser = Parser::new();
  parser
    .set_language(&language())
    .map_err(|e| MojoParseError::UnsupportedLanguage(e.to_string()))?;
  parser
    .parse(source, None)
    .ok_or(MojoParseError::ParseCancelled)
}

fn module_name_from_path(file_path: &str) -> String {
  let path_without_ext = file_path
    .strip_suffix(".mojo")
    .or_else(|| file_path.strip_suffix(".🔥"))
    .unwrap_or(file_path);
  path_without_ext.replace(['/', '\\'], ".")
}

fn current_scope_qname(module_name: &str, scope_stack: &[(String, SymbolKind)]) -> String {
  if scope_stack.is_empty() {
    module_name.to_string()
  } else {
    let mut qname = module_name.to_string();
    qname.push_str("::");
    for (i, (name, _kind)) in scope_stack.iter().enumerate() {
      if i > 0 {
        qname.push('.');
      }
      qname.push_str(name);
    }
    qname
  }
}

// ----------------------------------------
// 1. Symbol Extraction
// ----------------------------------------

pub fn extract_symbols_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<Symbol>, MojoParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let module_name = module_name_from_path(file_path);
  let mut symbols = Vec::new();

  let line_count = source.lines().count().max(1);
  let module_symbol_name = Path::new(file_path)
    .file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or(file_path)
    .to_string();

  symbols.push(Symbol {
    name: module_symbol_name,
    qualified_name: module_name.clone(),
    kind: SymbolKind::Module,
    location: Location {
      file_path: file_path.to_string(),
      start_line: 1,
      end_line: line_count,
      start_column: Some(0),
      end_column: Some(0),
    },
  });

  let mut scope_stack = Vec::new();
  traverse_symbols(
    root,
    source,
    file_path,
    &module_name,
    &mut scope_stack,
    &mut symbols,
  );

  Ok(symbols)
}

fn traverse_symbols(
  node: Node<'_>,
  source: &str,
  file_path: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, SymbolKind)>,
  symbols: &mut Vec<Symbol>,
) {
  let kind = node.kind();
  let mut name_pushed = false;

  match kind {
    "class_definition" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let class_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string();
        if !class_name.is_empty() {
          let scope_path = scope_stack
            .iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>()
            .join(".");
          let qname = if scope_path.is_empty() {
            format!("{}::{}", module_name, class_name)
          } else {
            format!("{}::{}.{}", module_name, scope_path, class_name)
          };

          let start = node.start_position();
          let end = node.end_position();

          let is_struct = {
            let mut cursor = node.walk();
            node.children(&mut cursor).any(|c| c.kind() == "struct")
          };
          let symbol_kind = if is_struct {
            SymbolKind::Struct
          } else {
            SymbolKind::Class
          };

          symbols.push(Symbol {
            name: class_name.clone(),
            qualified_name: qname.clone(),
            kind: symbol_kind.clone(),
            location: Location {
              file_path: file_path.to_string(),
              start_line: start.row + 1,
              end_line: end.row + 1,
              start_column: Some(start.column),
              end_column: Some(end.column),
            },
          });

          scope_stack.push((class_name, symbol_kind));
          name_pushed = true;
        }
      }
    }
    "trait_definition" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let trait_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string();
        if !trait_name.is_empty() {
          let scope_path = scope_stack
            .iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>()
            .join(".");
          let qname = if scope_path.is_empty() {
            format!("{}::{}", module_name, trait_name)
          } else {
            format!("{}::{}.{}", module_name, scope_path, trait_name)
          };

          let start = node.start_position();
          let end = node.end_position();

          symbols.push(Symbol {
            name: trait_name.clone(),
            qualified_name: qname.clone(),
            kind: SymbolKind::Interface,
            location: Location {
              file_path: file_path.to_string(),
              start_line: start.row + 1,
              end_line: end.row + 1,
              start_column: Some(start.column),
              end_column: Some(end.column),
            },
          });

          scope_stack.push((trait_name, SymbolKind::Interface));
          name_pushed = true;
        }
      }
    }
    "function_definition" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let func_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string();
        if !func_name.is_empty() {
          let is_method = scope_stack
            .last()
            .map(|(_, kind)| {
              *kind == SymbolKind::Class
                || *kind == SymbolKind::Struct
                || *kind == SymbolKind::Interface
            })
            .unwrap_or(false);

          let symbol_kind = if is_method {
            SymbolKind::Method
          } else {
            SymbolKind::Function
          };

          let scope_path = scope_stack
            .iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>()
            .join(".");
          let qname = if scope_path.is_empty() {
            format!("{}::{}", module_name, func_name)
          } else {
            format!("{}::{}.{}", module_name, scope_path, func_name)
          };

          let start = node.start_position();
          let end = node.end_position();

          symbols.push(Symbol {
            name: func_name.clone(),
            qualified_name: qname,
            kind: symbol_kind.clone(),
            location: Location {
              file_path: file_path.to_string(),
              start_line: start.row + 1,
              end_line: end.row + 1,
              start_column: Some(start.column),
              end_column: Some(end.column),
            },
          });

          scope_stack.push((func_name, symbol_kind));
          name_pushed = true;
        }
      }
    }
    _ => {}
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_symbols(child, source, file_path, module_name, scope_stack, symbols);
  }

  if name_pushed {
    scope_stack.pop();
  }
}

// ----------------------------------------
// 2. Import Extraction
// ----------------------------------------

pub fn extract_imports_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedImport>, MojoParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let module_name = module_name_from_path(file_path);
  let mut imports = Vec::new();

  traverse_imports(root, source, &module_name, &mut imports);

  Ok(imports)
}

fn traverse_imports(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  imports: &mut Vec<ExtractedImport>,
) {
  let kind = node.kind();
  match kind {
    "import_statement" => {
      let mut cursor = node.walk();
      for child in node.children(&mut cursor) {
        if child.is_named() {
          let mut target_name = None;
          if child.kind() == "dotted_name" {
            target_name = Some(
              child
                .utf8_text(source.as_bytes())
                .unwrap_or("")
                .trim()
                .to_string(),
            );
          } else if child.kind() == "aliased_import" {
            target_name = child.child_by_field_name("name").map(|name_node| {
              name_node
                .utf8_text(source.as_bytes())
                .unwrap_or("")
                .trim()
                .to_string()
            });
          }
          if let Some(tname) = target_name.filter(|n| !n.is_empty()) {
            let start = child.start_position();
            let end = child.end_position();
            imports.push(ExtractedImport {
              target_qualified_name: tname,
              start_line: start.row + 1,
              start_column: start.column,
              end_line: end.row + 1,
              end_column: end.column,
            });
          }
        }
      }
    }
    "import_from_statement" => {
      let mut source_module_text = String::new();
      let mut has_import_keyword = false;
      let mut cursor = node.walk();
      for child in node.children(&mut cursor) {
        if child.kind() == "import" {
          has_import_keyword = true;
          break;
        }
        if child.kind() == "from" {
          continue;
        }
        source_module_text.push_str(child.utf8_text(source.as_bytes()).unwrap_or(""));
      }
      let source_module_text = source_module_text.trim().to_string();

      if has_import_keyword {
        let is_relative = source_module_text.starts_with('.');
        let dots_count = if is_relative {
          source_module_text.chars().take_while(|c| *c == '.').count()
        } else {
          0
        };
        let rest_name = if is_relative {
          source_module_text[dots_count..].trim().to_string()
        } else {
          source_module_text
        };

        let base_module = if is_relative {
          let parts: Vec<&str> = module_name.split('.').collect();
          let parent_parts = if parts.len() > 1 {
            &parts[0..parts.len() - 1]
          } else {
            &[]
          };
          let remaining_len = parent_parts.len().saturating_sub(dots_count - 1);
          let base_pkg = parent_parts[0..remaining_len].join(".");
          if rest_name.is_empty() {
            base_pkg
          } else if base_pkg.is_empty() {
            rest_name
          } else {
            format!("{base_pkg}.{rest_name}")
          }
        } else {
          rest_name
        };

        let mut imported_names = Vec::new();
        let mut cursor = node.walk();
        let mut past_import = false;
        for child in node.children(&mut cursor) {
          if child.kind() == "import" {
            past_import = true;
            continue;
          }
          if past_import {
            collect_imported_names(child, source, &mut imported_names);
          }
        }

        for (name, name_node) in imported_names {
          if !name.is_empty() {
            let target_qname = if name == "*" {
              base_module.clone()
            } else if base_module.is_empty() {
              name
            } else {
              format!("{base_module}.{name}")
            };
            let start = name_node.start_position();
            let end = name_node.end_position();
            imports.push(ExtractedImport {
              target_qualified_name: target_qname,
              start_line: start.row + 1,
              start_column: start.column,
              end_line: end.row + 1,
              end_column: end.column,
            });
          }
        }
      }
    }
    _ => {
      let mut cursor = node.walk();
      for child in node.children(&mut cursor) {
        traverse_imports(child, source, module_name, imports);
      }
    }
  }
}

fn collect_imported_names<'a>(node: Node<'a>, source: &str, names: &mut Vec<(String, Node<'a>)>) {
  let kind = node.kind();
  if kind == "dotted_name" {
    names.push((
      node
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .trim()
        .to_string(),
      node,
    ));
  } else if kind == "aliased_import" {
    if let Some(name_node) = node.child_by_field_name("name") {
      names.push((
        name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string(),
        node,
      ));
    }
  } else if kind == "wildcard_import" || kind == "*" {
    names.push(("*".to_string(), node));
  } else {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
      collect_imported_names(child, source, names);
    }
  }
}

// ----------------------------------------
// 3. Call Extraction
// ----------------------------------------

pub fn extract_calls_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedCall>, MojoParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let module_name = module_name_from_path(file_path);
  let mut calls = Vec::new();
  let mut scope_stack = Vec::new();

  traverse_calls(root, source, &module_name, &mut scope_stack, &mut calls);

  Ok(calls)
}

fn traverse_calls(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, SymbolKind)>,
  calls: &mut Vec<ExtractedCall>,
) {
  let kind = node.kind();
  let mut name_pushed = false;

  match kind {
    "class_definition" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let class_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !class_name.is_empty() {
          let is_struct = {
            let mut cursor = node.walk();
            node.children(&mut cursor).any(|c| c.kind() == "struct")
          };
          let symbol_kind = if is_struct {
            SymbolKind::Struct
          } else {
            SymbolKind::Class
          };
          scope_stack.push((class_name, symbol_kind));
          name_pushed = true;
        }
      }
    }
    "trait_definition" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let trait_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !trait_name.is_empty() {
          scope_stack.push((trait_name, SymbolKind::Interface));
          name_pushed = true;
        }
      }
    }
    "function_definition" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let func_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !func_name.is_empty() {
          let is_method = scope_stack
            .last()
            .map(|(_, kind)| {
              *kind == SymbolKind::Class
                || *kind == SymbolKind::Struct
                || *kind == SymbolKind::Interface
            })
            .unwrap_or(false);

          let symbol_kind = if is_method {
            SymbolKind::Method
          } else {
            SymbolKind::Function
          };

          scope_stack.push((func_name, symbol_kind));
          name_pushed = true;
        }
      }
    }
    "call" => {
      if let Some(func_node) = node.child_by_field_name("function") {
        let target_name = func_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string();
        if !target_name.is_empty() {
          let mut is_self_call = false;
          let mut method_name = None;

          if func_node.kind() == "attribute" {
            if let Some(obj_node) = func_node.child_by_field_name("object") {
              let obj_text = obj_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
              if obj_text == "self" {
                is_self_call = true;
              }
            }
            if let Some(attr_node) = func_node.child_by_field_name("attribute") {
              method_name = Some(
                attr_node
                  .utf8_text(source.as_bytes())
                  .unwrap_or("")
                  .trim()
                  .to_string(),
              );
            }
          }

          let start = node.start_position();
          let end = node.end_position();
          let source_symbol_qualified_name = current_scope_qname(module_name, scope_stack);

          calls.push(ExtractedCall {
            source_symbol_qualified_name: source_symbol_qualified_name.clone(),
            target_name: target_name.clone(),
            is_self_call,
            method_name: method_name.clone(),
            start_line: start.row + 1,
            start_column: start.column,
            end_line: end.row + 1,
            end_column: end.column,
          });
        }
      }
    }
    _ => {}
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_calls(child, source, module_name, scope_stack, calls);
  }

  if name_pushed {
    scope_stack.pop();
  }
}

// ----------------------------------------
// 4. Type Reference Extraction
// ----------------------------------------

pub fn extract_type_references_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedTypeReference>, MojoParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let module_name = module_name_from_path(file_path);
  let mut type_refs = Vec::new();
  let mut scope_stack = Vec::new();

  traverse_type_references(root, source, &module_name, &mut scope_stack, &mut type_refs);

  Ok(type_refs)
}

fn traverse_type_references(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, SymbolKind)>,
  type_refs: &mut Vec<ExtractedTypeReference>,
) {
  let kind = node.kind();
  let mut name_pushed = false;

  match kind {
    "class_definition" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let class_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !class_name.is_empty() {
          let is_struct = {
            let mut cursor = node.walk();
            node.children(&mut cursor).any(|c| c.kind() == "struct")
          };
          let symbol_kind = if is_struct {
            SymbolKind::Struct
          } else {
            SymbolKind::Class
          };
          scope_stack.push((class_name, symbol_kind));
          name_pushed = true;

          // Extract superclasses
          if let Some(arg_list) = node.child_by_field_name("superclasses") {
            let mut cursor = arg_list.walk();
            for child in arg_list.children(&mut cursor) {
              if child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
                let mut extracted = Vec::new();
                collect_types_from_annotation_node(child, source, &mut extracted);
                for (name, range) in extracted {
                  if !is_builtin_or_typing_type(&name) {
                    let source_symbol_qualified_name =
                      current_scope_qname(module_name, scope_stack);
                    type_refs.push(ExtractedTypeReference {
                      source_symbol_qualified_name,
                      target_type_name: name,
                      start_line: range.start_point.row + 1,
                      start_column: range.start_point.column,
                      end_line: range.end_point.row + 1,
                      end_column: range.end_point.column,
                    });
                  }
                }
              }
            }
          }
        }
      }
    }
    "trait_definition" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let trait_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !trait_name.is_empty() {
          scope_stack.push((trait_name, SymbolKind::Interface));
          name_pushed = true;

          // Extract supertraits
          if let Some(supertraits_list) = node.child_by_field_name("supertraits") {
            let mut cursor = supertraits_list.walk();
            for child in supertraits_list.children(&mut cursor) {
              if child.kind() == "identifier" {
                let name = child
                  .utf8_text(source.as_bytes())
                  .unwrap_or("")
                  .trim()
                  .to_string();
                if !name.is_empty() && !is_builtin_or_typing_type(&name) {
                  let source_symbol_qualified_name = current_scope_qname(module_name, scope_stack);
                  type_refs.push(ExtractedTypeReference {
                    source_symbol_qualified_name,
                    target_type_name: name,
                    start_line: child.start_position().row + 1,
                    start_column: child.start_position().column,
                    end_line: child.end_position().row + 1,
                    end_column: child.end_position().column,
                  });
                }
              }
            }
          }
        }
      }
    }
    "function_definition" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let func_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !func_name.is_empty() {
          let is_method = scope_stack
            .last()
            .map(|(_, kind)| {
              *kind == SymbolKind::Class
                || *kind == SymbolKind::Struct
                || *kind == SymbolKind::Interface
            })
            .unwrap_or(false);

          let symbol_kind = if is_method {
            SymbolKind::Method
          } else {
            SymbolKind::Function
          };

          scope_stack.push((func_name, symbol_kind));
          name_pushed = true;

          // Extract return_type
          let mut cursor = node.walk();
          for child in node.children(&mut cursor) {
            if child.kind() == "type" {
              let mut extracted = Vec::new();
              collect_types_from_annotation_node(child, source, &mut extracted);
              for (name, range) in extracted {
                if !is_builtin_or_typing_type(&name) {
                  let source_symbol_qualified_name = current_scope_qname(module_name, scope_stack);
                  type_refs.push(ExtractedTypeReference {
                    source_symbol_qualified_name,
                    target_type_name: name,
                    start_line: range.start_point.row + 1,
                    start_column: range.start_point.column,
                    end_line: range.end_point.row + 1,
                    end_column: range.end_point.column,
                  });
                }
              }
            }
          }
        }
      }
    }
    "typed_parameter" | "typed_default_parameter" => {
      let mut cursor = node.walk();
      for child in node.children(&mut cursor) {
        if child.kind() == "type" {
          let mut extracted = Vec::new();
          collect_types_from_annotation_node(child, source, &mut extracted);
          for (name, range) in extracted {
            if !is_builtin_or_typing_type(&name) {
              let source_symbol_qualified_name = current_scope_qname(module_name, scope_stack);
              type_refs.push(ExtractedTypeReference {
                source_symbol_qualified_name,
                target_type_name: name,
                start_line: range.start_point.row + 1,
                start_column: range.start_point.column,
                end_line: range.end_point.row + 1,
                end_column: range.end_point.column,
              });
            }
          }
        }
      }
    }
    "assignment" => {
      let mut cursor = node.walk();
      for child in node.children(&mut cursor) {
        if child.kind() == "type" {
          let mut extracted = Vec::new();
          collect_types_from_annotation_node(child, source, &mut extracted);
          for (name, range) in extracted {
            if !is_builtin_or_typing_type(&name) {
              let source_symbol_qualified_name = current_scope_qname(module_name, scope_stack);
              type_refs.push(ExtractedTypeReference {
                source_symbol_qualified_name,
                target_type_name: name,
                start_line: range.start_point.row + 1,
                start_column: range.start_point.column,
                end_line: range.end_point.row + 1,
                end_column: range.end_point.column,
              });
            }
          }
        }
      }
    }
    _ => {}
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_type_references(child, source, module_name, scope_stack, type_refs);
  }

  if name_pushed {
    scope_stack.pop();
  }
}

fn collect_types_from_annotation_node(
  node: Node<'_>,
  source: &str,
  extracted: &mut Vec<(String, tree_sitter::Range)>,
) {
  let kind = node.kind();
  if kind == "attribute" {
    if let (Some(obj_node), Some(attr_node)) = (
      node.child_by_field_name("object"),
      node.child_by_field_name("attribute"),
    ) {
      let obj_text = obj_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
      let attr_text = attr_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
      if !obj_text.is_empty() && !attr_text.is_empty() {
        let full_attr = format!("{}.{}", obj_text, attr_text);
        extracted.push((full_attr, node.range()));
        return;
      }
    }
  } else if kind == "identifier" {
    let text = node
      .utf8_text(source.as_bytes())
      .unwrap_or("")
      .trim()
      .to_string();
    if !text.is_empty() {
      extracted.push((text, node.range()));
    }
    return;
  } else if kind == "string" {
    let raw_text = node.utf8_text(source.as_bytes()).unwrap_or("").trim();
    if raw_text.len() >= 2 {
      let first = raw_text.chars().next().unwrap();
      let last = raw_text.chars().last().unwrap();
      if (first == '\'' && last == '\'') || (first == '"' && last == '"') {
        let inner = raw_text[1..raw_text.len() - 1].trim();
        let is_valid = !inner.is_empty()
          && inner
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.');
        if is_valid {
          extracted.push((inner.to_string(), node.range()));
        }
      }
    }
    return;
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    collect_types_from_annotation_node(child, source, extracted);
  }
}

fn is_builtin_or_typing_type(name: &str) -> bool {
  let lower = name.to_lowercase();
  let builtins = [
    // Python built-ins
    "int",
    "str",
    "float",
    "bool",
    "list",
    "dict",
    "set",
    "tuple",
    "any",
    "none",
    "object",
    "bytes",
    "callable",
    "iterable",
    "iterator",
    "generator",
    // Mojo built-ins
    "void",
    "simd",
    "dtype",
    "scalar",
    "index",
    "anytype",
    "reference",
    "float16",
    "float32",
    "float64",
    "bfloat16",
    "int8",
    "int16",
    "int32",
    "int64",
    "uint8",
    "uint16",
    "uint32",
    "uint64",
    "string",
    "stringref",
    "stringliteral",
    "pointer",
    "dtypepointer",
    // Direct top-level typing structures
    "optional",
    "union",
  ];
  if builtins.contains(&lower.as_str()) {
    return true;
  }

  let typing_wrappers = [
    "optional",
    "union",
    "list",
    "dict",
    "tuple",
    "set",
    "any",
    "type",
    "generic",
    "typevar",
    "callable",
    "iterable",
    "iterator",
    "generator",
  ];

  let parts: Vec<&str> = name.split('.').collect();
  if parts.len() == 2 && parts[0] == "typing" {
    let lower_wrapper = parts[1].to_lowercase();
    if typing_wrappers.contains(&lower_wrapper.as_str()) {
      return true;
    }
  }

  false
}

// ----------------------------------------
// 5. Test Extraction
// ----------------------------------------

pub fn extract_tests_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedTest>, MojoParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let module_name = module_name_from_path(file_path);
  let mut tests = Vec::new();
  let mut scope_stack = Vec::new();

  traverse_tests(root, source, &module_name, &mut scope_stack, &mut tests);

  Ok(tests)
}

fn traverse_tests(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, SymbolKind)>,
  tests: &mut Vec<ExtractedTest>,
) {
  let kind = node.kind();
  let mut name_pushed = false;

  match kind {
    "class_definition" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let class_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !class_name.is_empty() {
          let is_struct = {
            let mut cursor = node.walk();
            node.children(&mut cursor).any(|c| c.kind() == "struct")
          };
          let symbol_kind = if is_struct {
            SymbolKind::Struct
          } else {
            SymbolKind::Class
          };
          scope_stack.push((class_name.clone(), symbol_kind));
          name_pushed = true;

          if class_name.starts_with("Test") {
            let qname = current_scope_qname(module_name, scope_stack);
            let start = node.start_position();
            let end = node.end_position();
            tests.push(ExtractedTest {
              name: class_name,
              qualified_name: qname,
              kind: ExtractedTestKind::Class,
              is_parametrized: false,
              parameters: Vec::new(),
              start_line: start.row + 1,
              start_column: start.column,
              end_line: end.row + 1,
              end_column: end.column,
            });
          }
        }
      }
    }
    "function_definition" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let func_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !func_name.is_empty() {
          let is_method = scope_stack
            .last()
            .map(|(_, k)| {
              *k == SymbolKind::Class || *k == SymbolKind::Struct || *k == SymbolKind::Interface
            })
            .unwrap_or(false);

          let symbol_kind = if is_method {
            SymbolKind::Method
          } else {
            SymbolKind::Function
          };

          scope_stack.push((func_name.clone(), symbol_kind));
          name_pushed = true;

          if func_name.starts_with("test_") {
            let qname = current_scope_qname(module_name, scope_stack);
            let start = node.start_position();
            let end = node.end_position();
            let parameters = get_parameter_names(node, source);

            tests.push(ExtractedTest {
              name: func_name,
              qualified_name: qname,
              kind: ExtractedTestKind::Function,
              is_parametrized: false,
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
    _ => {}
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_tests(child, source, module_name, scope_stack, tests);
  }

  if name_pushed {
    scope_stack.pop();
  }
}

fn get_parameter_names(node: Node<'_>, source: &str) -> Vec<String> {
  let mut params = Vec::new();
  if let Some(param_list) = node.child_by_field_name("parameters") {
    let mut cursor = param_list.walk();
    for child in param_list.children(&mut cursor) {
      let kind = child.kind();
      if kind == "parameter"
        || kind == "typed_parameter"
        || kind == "default_parameter"
        || kind == "typed_default_parameter"
        || kind == "identifier"
      {
        let name_node = if let Some(n) = child.child_by_field_name("name") {
          Some(n)
        } else {
          find_first_identifier(child)
        };
        if let Some(n) = name_node {
          let name = n
            .utf8_text(source.as_bytes())
            .unwrap_or("")
            .trim()
            .to_string();
          if !name.is_empty() && name != "self" {
            params.push(name);
          }
        }
      }
    }
  }
  params
}

fn find_first_identifier(node: Node<'_>) -> Option<Node<'_>> {
  if node.kind() == "identifier" {
    return Some(node);
  }
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if let Some(id) = find_first_identifier(child) {
      return Some(id);
    }
  }
  None
}

// ----------------------------------------
// 6. Unit Tests
// ----------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_extract_mojo_symbols() {
    let source = r#"
fn global_fn(x: Int) -> Bool:
    return True

struct S:
    fn __init__(out self):
        pass

    fn s_method(read x: Float) -> Int:
        return 0

trait T:
    fn t_method(self) -> Void:
        pass
"#;
    let symbols = extract_symbols_from_source(source, "src/lib.mojo").unwrap();

    // 1. Module
    let module = symbols
      .iter()
      .find(|s| s.kind == SymbolKind::Module)
      .unwrap();
    assert_eq!(module.name, "lib");
    assert_eq!(module.qualified_name, "src.lib");

    // 2. Global function
    let global_fn = symbols.iter().find(|s| s.name == "global_fn").unwrap();
    assert_eq!(global_fn.kind, SymbolKind::Function);
    assert_eq!(global_fn.qualified_name, "src.lib::global_fn");

    // 3. Struct
    let struct_s = symbols.iter().find(|s| s.name == "S").unwrap();
    assert_eq!(struct_s.kind, SymbolKind::Struct);
    assert_eq!(struct_s.qualified_name, "src.lib::S");

    // 4. Methods
    let init = symbols.iter().find(|s| s.name == "__init__").unwrap();
    assert_eq!(init.kind, SymbolKind::Method);
    assert_eq!(init.qualified_name, "src.lib::S.__init__");

    let s_method = symbols.iter().find(|s| s.name == "s_method").unwrap();
    assert_eq!(s_method.kind, SymbolKind::Method);
    assert_eq!(s_method.qualified_name, "src.lib::S.s_method");

    // 5. Trait
    let trait_t = symbols.iter().find(|s| s.name == "T").unwrap();
    assert_eq!(trait_t.kind, SymbolKind::Interface);
    assert_eq!(trait_t.qualified_name, "src.lib::T");

    let t_method = symbols.iter().find(|s| s.name == "t_method").unwrap();
    assert_eq!(t_method.kind, SymbolKind::Method);
    assert_eq!(t_method.qualified_name, "src.lib::T.t_method");
  }

  #[test]
  fn test_extract_mojo_imports() {
    let source = r#"
import sys
import numpy as np
from python import Python
from .helper import helper_func
"#;
    let imports = extract_imports_from_source(source, "src/lib.mojo").unwrap();
    assert!(imports.iter().any(|imp| imp.target_qualified_name == "sys"));
    assert!(
      imports
        .iter()
        .any(|imp| imp.target_qualified_name == "numpy")
    );
    assert!(
      imports
        .iter()
        .any(|imp| imp.target_qualified_name == "python.Python")
    );
    assert!(
      imports
        .iter()
        .any(|imp| imp.target_qualified_name == "src.helper.helper_func")
    );
  }

  #[test]
  fn test_extract_mojo_calls() {
    let source = r#"
fn caller():
    callee()
    self.method_callee()
"#;
    let calls = extract_calls_from_source(source, "src/lib.mojo").unwrap();
    assert!(calls.iter().any(|c| c.target_name == "callee"));
    let self_call = calls
      .iter()
      .find(|c| c.target_name.contains("method_callee"))
      .unwrap();
    assert!(self_call.is_self_call);
    assert_eq!(self_call.method_name.as_deref(), Some("method_callee"));
  }

  #[test]
  fn test_extract_mojo_type_references() {
    let source = r#"
struct MyClass(BaseClass):
    pass

fn compute(x: Patient, y: Validator) -> Result:
    pass
"#;
    let refs = extract_type_references_from_source(source, "src/lib.mojo").unwrap();
    assert!(refs.iter().any(|r| r.target_type_name == "BaseClass"));
    assert!(refs.iter().any(|r| r.target_type_name == "Patient"));
    assert!(refs.iter().any(|r| r.target_type_name == "Validator"));
    assert!(refs.iter().any(|r| r.target_type_name == "Result"));
  }

  #[test]
  fn test_extract_mojo_tests() {
    let source = r#"
struct TestSuite:
    fn test_method(self, fixture_arg: Int):
        pass

fn test_global_func():
    pass
"#;
    let tests = extract_tests_from_source(source, "tests/my_test.mojo").unwrap();

    let suite = tests.iter().find(|t| t.name == "TestSuite").unwrap();
    assert_eq!(suite.kind, ExtractedTestKind::Class);

    let method = tests.iter().find(|t| t.name == "test_method").unwrap();
    assert_eq!(method.kind, ExtractedTestKind::Function);
    assert_eq!(method.parameters, vec!["fixture_arg"]);

    let global_t = tests.iter().find(|t| t.name == "test_global_func").unwrap();
    assert_eq!(global_t.kind, ExtractedTestKind::Function);
    assert!(global_t.parameters.is_empty());
  }

  #[test]
  fn test_mojo_parameter_conventions() {
    let source = r#"
fn test_params(borrowed a: Int, inout b: Float, owned c: String, mut d: Bool, read e: Int, ref f: Float):
    pass
"#;
    let tree = parse_to_tree(source).unwrap();
    let root = tree.root_node();
    let params = get_parameter_names(root.child(0).unwrap(), source);
    assert_eq!(params, vec!["a", "b", "c", "d", "e", "f"]);
  }
}
