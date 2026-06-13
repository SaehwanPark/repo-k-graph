#![allow(clippy::collapsible_if)]
use std::fmt::{Display, Formatter};
use std::path::Path;

use rkg_core::{Location, Symbol, SymbolKind};
use tree_sitter::{Node, Parser};

pub const CRATE_NAME: &str = "rkg-lang-python";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseReport {
  pub root_kind: String,
  pub named_node_count: usize,
  pub syntax_errors: Vec<SyntaxError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
  pub message: String,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}

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
  pub ordering: Option<usize>,
  pub placeholders: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedDecorator {
  pub source_symbol_qualified_name: String,
  pub decorator_name: String,
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
pub struct ExtractedDocstring {
  pub symbol_qualified_name: String,
  pub text: String,
  pub start_line: usize,
  pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedRoute {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PythonParseError {
  UnsupportedLanguage(String),
  ParseCancelled,
}

impl Display for PythonParseError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      PythonParseError::UnsupportedLanguage(message) => {
        write!(f, "failed to initialize python parser: {message}")
      }
      PythonParseError::ParseCancelled => write!(f, "python parsing was cancelled"),
    }
  }
}

impl std::error::Error for PythonParseError {}

fn parse_to_tree(source: &str) -> Result<tree_sitter::Tree, PythonParseError> {
  let mut parser = Parser::new();
  parser
    .set_language(&tree_sitter_python::LANGUAGE.into())
    .map_err(|e| PythonParseError::UnsupportedLanguage(e.to_string()))?;
  parser
    .parse(source, None)
    .ok_or(PythonParseError::ParseCancelled)
}

pub fn parse_python_source(source: &str) -> Result<ParseReport, PythonParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let mut named_node_count = 0usize;
  let mut syntax_errors = Vec::new();
  collect_diagnostics(root, &mut named_node_count, &mut syntax_errors);

  Ok(ParseReport {
    root_kind: root.kind().to_string(),
    named_node_count,
    syntax_errors,
  })
}

fn collect_diagnostics(
  root: Node<'_>,
  named_node_count: &mut usize,
  syntax_errors: &mut Vec<SyntaxError>,
) {
  let mut cursor = root.walk();
  loop {
    collect_node_diagnostic(cursor.node(), named_node_count, syntax_errors);

    if cursor.goto_first_child() {
      continue;
    }

    loop {
      if cursor.goto_next_sibling() {
        break;
      }
      if !cursor.goto_parent() {
        return;
      }
    }
  }
}

fn collect_node_diagnostic(
  node: Node<'_>,
  named_node_count: &mut usize,
  syntax_errors: &mut Vec<SyntaxError>,
) {
  if node.is_named() {
    *named_node_count += 1;
  }
  if node.is_error() || node.is_missing() {
    let start = node.start_position();
    let end = node.end_position();
    syntax_errors.push(SyntaxError {
      message: if node.is_missing() {
        format!("missing {}", node.kind())
      } else {
        format!("error {}", node.kind())
      },
      start_line: one_based(start.row),
      start_column: one_based(start.column),
      end_line: one_based(end.row),
      end_column: one_based(end.column),
    });
  }
}

fn one_based(value: usize) -> usize {
  value + 1
}

pub fn extract_symbols_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<Symbol>, PythonParseError> {
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
  traverse(
    root,
    source,
    file_path,
    &module_name,
    &mut scope_stack,
    &mut symbols,
  );

  Ok(symbols)
}

fn clean_docstring(raw: &str) -> String {
  let trimmed = raw.trim();
  let is_triple_quoted = (trimmed.starts_with("\"\"\"") && trimmed.ends_with("\"\"\""))
    || (trimmed.starts_with("'''") && trimmed.ends_with("'''"));
  if is_triple_quoted && trimmed.len() >= 6 {
    trimmed[3..trimmed.len() - 3].trim().to_string()
  } else {
    let is_single_quoted = (trimmed.starts_with('"') && trimmed.ends_with('"'))
      || (trimmed.starts_with('\'') && trimmed.ends_with('\''));
    if is_single_quoted && trimmed.len() >= 2 {
      trimmed[1..trimmed.len() - 1].trim().to_string()
    } else {
      trimmed.to_string()
    }
  }
}

#[allow(clippy::collapsible_if)]
fn extract_first_string_child(
  node: Node<'_>,
  source: &str,
) -> Option<(String, tree_sitter::Range)> {
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    let kind = child.kind();
    if kind == "expression_statement" {
      if let Some(str_node) = child.child(0) {
        if str_node.kind() == "string" {
          let raw = str_node.utf8_text(source.as_bytes()).unwrap_or("");
          let cleaned = clean_docstring(raw);
          if !cleaned.is_empty() {
            return Some((cleaned, str_node.range()));
          }
        }
      }
    } else if kind == "block" {
      return extract_first_string_child(child, source);
    } else if child.is_named() {
      break;
    }
  }
  None
}

pub fn extract_docstrings_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedDocstring>, PythonParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let module_name = module_name_from_path(file_path);
  let mut docstrings = Vec::new();

  if let Some(doc) = extract_first_string_child(root, source) {
    docstrings.push(ExtractedDocstring {
      symbol_qualified_name: module_name.clone(),
      text: doc.0,
      start_line: doc.1.start_point.row + 1,
      end_line: doc.1.end_point.row + 1,
    });
  }

  let mut scope_stack = Vec::new();
  traverse_for_docstrings(
    root,
    source,
    &module_name,
    &mut scope_stack,
    &mut docstrings,
  );

  Ok(docstrings)
}

#[allow(clippy::collapsible_if)]
fn traverse_for_docstrings(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, bool)>,
  docstrings: &mut Vec<ExtractedDocstring>,
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
          scope_stack.push((class_name, true));
          name_pushed = true;

          if let Some(body_node) = node.child_by_field_name("body") {
            if let Some(doc) = extract_first_string_child(body_node, source) {
              let qname = current_scope_qname(module_name, scope_stack);
              docstrings.push(ExtractedDocstring {
                symbol_qualified_name: qname,
                text: doc.0,
                start_line: doc.1.start_point.row + 1,
                end_line: doc.1.end_point.row + 1,
              });
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
          scope_stack.push((func_name, false));
          name_pushed = true;

          if let Some(body_node) = node.child_by_field_name("body") {
            if let Some(doc) = extract_first_string_child(body_node, source) {
              let qname = current_scope_qname(module_name, scope_stack);
              docstrings.push(ExtractedDocstring {
                symbol_qualified_name: qname,
                text: doc.0,
                start_line: doc.1.start_point.row + 1,
                end_line: doc.1.end_point.row + 1,
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
    traverse_for_docstrings(child, source, module_name, scope_stack, docstrings);
  }

  if name_pushed {
    scope_stack.pop();
  }
}

fn module_name_from_path(file_path: &str) -> String {
  let path_without_ext = file_path.strip_suffix(".py").unwrap_or(file_path);
  path_without_ext.replace(['/', '\\'], ".")
}

fn is_pytorch_module_class(node: Node<'_>, source: &str) -> bool {
  if let Some(arg_list) = node.child_by_field_name("superclasses") {
    let mut cursor = arg_list.walk();
    for child in arg_list.children(&mut cursor) {
      let super_text = child.utf8_text(source.as_bytes()).unwrap_or("").trim();
      if super_text == "Module"
        || super_text == "nn.Module"
        || super_text == "torch.nn.Module"
        || super_text.ends_with(".Module")
      {
        return true;
      }
    }
  }
  if let Some(body_node) = node.child_by_field_name("body") {
    let mut cursor = body_node.walk();
    for child in body_node.children(&mut cursor) {
      if child.kind() == "function_definition" {
        if let Some(name_node) = child.child_by_field_name("name") {
          let name = name_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
          if name == "forward" {
            return true;
          }
        }
      }
    }
  }
  false
}

fn find_init_method<'a>(class_node: Node<'a>, source: &str) -> Option<Node<'a>> {
  if let Some(body_node) = class_node.child_by_field_name("body") {
    let mut cursor = body_node.walk();
    for child in body_node.children(&mut cursor) {
      if child.kind() == "function_definition" {
        if let Some(name_node) = child.child_by_field_name("name") {
          let name = name_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
          if name == "__init__" {
            return Some(child);
          }
        }
      }
    }
  }
  None
}

fn collect_self_assignments<'a>(
  node: Node<'a>,
  source: &str,
  assignments: &mut Vec<(String, String, Node<'a>, Node<'a>)>,
) {
  if node.kind() == "assignment" {
    if let Some(left_node) = node.child(0) {
      if left_node.kind() == "attribute" {
        if let Some(obj_node) = left_node.child_by_field_name("object") {
          let obj_text = obj_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
          if obj_text == "self" {
            if let Some(attr_node) = left_node.child_by_field_name("attribute") {
              let attr_name = attr_node
                .utf8_text(source.as_bytes())
                .unwrap_or("")
                .trim()
                .to_string();
              let mut has_equals = false;
              let mut right_node = None;
              let mut val_cursor = node.walk();
              for child in node.children(&mut val_cursor) {
                if child.kind() == "=" {
                  has_equals = true;
                } else if has_equals {
                  right_node = Some(child);
                  break;
                }
              }
              if let Some(r_node) = right_node {
                let right_text = r_node
                  .utf8_text(source.as_bytes())
                  .unwrap_or("")
                  .trim()
                  .to_string();
                if r_node.kind() == "call" {
                  assignments.push((attr_name, right_text, r_node, node));
                }
              }
            }
          }
        }
      }
    }
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    collect_self_assignments(child, source, assignments);
  }
}

fn collect_column_names_from_arguments(
  node: Node<'_>,
  source: &str,
  columns: &mut Vec<(String, tree_sitter::Range)>,
) {
  // 1. Walk 1: Recursively find all pl.col(...) calls at any depth in the argument list
  fn find_col_calls(n: Node<'_>, source: &str, cols: &mut Vec<(String, tree_sitter::Range)>) {
    if n.kind() == "call" {
      if let Some(func_node) = n.child_by_field_name("function") {
        let func_text = func_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
        if func_text == "col" || func_text == "pl.col" || func_text.ends_with(".col") {
          let mut cursor = n.walk();
          if let Some(arg_list) = n
            .children(&mut cursor)
            .find(|c| c.kind() == "argument_list")
          {
            let mut arg_cursor = arg_list.walk();
            for child in arg_list.children(&mut arg_cursor) {
              if child.kind() == "string" {
                let text = child.utf8_text(source.as_bytes()).unwrap_or("");
                let trimmed = text.trim();
                let stripped = if (trimmed.starts_with('"') && trimmed.ends_with('"'))
                  || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
                {
                  if trimmed.len() >= 2 {
                    trimmed[1..trimmed.len() - 1].to_string()
                  } else {
                    trimmed.to_string()
                  }
                } else {
                  trimmed.to_string()
                };
                if !stripped.is_empty() {
                  cols.push((stripped, child.range()));
                }
              }
            }
          }
          return; // Stop recursion inside pl.col(...) to prevent duplicate extraction of its string child
        }
      }
    }
    let mut cursor = n.walk();
    for child in n.children(&mut cursor) {
      find_col_calls(child, source, cols);
    }
  }

  find_col_calls(node, source, columns);

  // 2. Walk 2: Find bare string constants that are direct children of the argument_list or direct children of a list child
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if child.kind() == "string" {
      let text = child.utf8_text(source.as_bytes()).unwrap_or("");
      let trimmed = text.trim();
      let stripped = if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
      {
        if trimmed.len() >= 2 {
          trimmed[1..trimmed.len() - 1].to_string()
        } else {
          trimmed.to_string()
        }
      } else {
        trimmed.to_string()
      };
      if !stripped.is_empty() && stripped.chars().all(|c| c.is_alphanumeric() || c == '_') {
        columns.push((stripped, child.range()));
      }
    } else if child.kind() == "list" {
      let mut list_cursor = child.walk();
      for item in child.children(&mut list_cursor) {
        if item.kind() == "string" {
          let text = item.utf8_text(source.as_bytes()).unwrap_or("");
          let trimmed = text.trim();
          let stripped = if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
          {
            if trimmed.len() >= 2 {
              trimmed[1..trimmed.len() - 1].to_string()
            } else {
              trimmed.to_string()
            }
          } else {
            trimmed.to_string()
          };
          if !stripped.is_empty() && stripped.chars().all(|c| c.is_alphanumeric() || c == '_') {
            columns.push((stripped, item.range()));
          }
        }
      }
    }
  }
}

fn traverse(
  node: Node<'_>,
  source: &str,
  file_path: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, bool)>,
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

          symbols.push(Symbol {
            name: class_name.clone(),
            qualified_name: qname.clone(),
            kind: SymbolKind::Class,
            location: Location {
              file_path: file_path.to_string(),
              start_line: start.row + 1,
              end_line: end.row + 1,
              start_column: Some(start.column),
              end_column: Some(end.column),
            },
          });

          // PyTorch submodule static extraction
          if is_pytorch_module_class(node, source) {
            if let Some(init_method) = find_init_method(node, source) {
              let mut assignments = Vec::new();
              collect_self_assignments(init_method, source, &mut assignments);
              for (attr_name, _right_text, _right_node, assign_node) in assignments {
                let sub_qname = format!("{}.{}", qname, attr_name);
                let sub_start = assign_node.start_position();
                let sub_end = assign_node.end_position();
                symbols.push(Symbol {
                  name: attr_name,
                  qualified_name: sub_qname,
                  kind: SymbolKind::Unknown,
                  location: Location {
                    file_path: file_path.to_string(),
                    start_line: sub_start.row + 1,
                    end_line: sub_end.row + 1,
                    start_column: Some(sub_start.column),
                    end_column: Some(sub_end.column),
                  },
                });
              }
            }
          }

          scope_stack.push((class_name, true));
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
            .map(|(_, is_class)| *is_class)
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
            kind: symbol_kind,
            location: Location {
              file_path: file_path.to_string(),
              start_line: start.row + 1,
              end_line: end.row + 1,
              start_column: Some(start.column),
              end_column: Some(end.column),
            },
          });

          scope_stack.push((func_name, false));
          name_pushed = true;
        }
      }
    }
    _ => {}
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse(child, source, file_path, module_name, scope_stack, symbols);
  }

  if name_pushed {
    scope_stack.pop();
  }
}

pub fn extract_imports_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedImport>, PythonParseError> {
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
  }
}

pub fn extract_calls_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedCall>, PythonParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let module_name = module_name_from_path(file_path);
  let mut calls = Vec::new();
  let mut scope_stack = Vec::new();

  let mut file_functions = std::collections::HashMap::new();
  let mut curried_functions = std::collections::HashSet::new();
  collect_file_functions_and_currying(root, source, &mut file_functions, &mut curried_functions);

  traverse_calls(
    root,
    source,
    &module_name,
    &mut scope_stack,
    &mut calls,
    false,
    &file_functions,
    &curried_functions,
  );

  // Group and sort by source position, then assign ordering
  let mut grouped: std::collections::HashMap<String, Vec<&mut ExtractedCall>> =
    std::collections::HashMap::new();
  for call in &mut calls {
    grouped
      .entry(call.source_symbol_qualified_name.clone())
      .or_default()
      .push(call);
  }

  for (_qname, mut scope_calls) in grouped {
    scope_calls.sort_by(|a, b| {
      a.start_line
        .cmp(&b.start_line)
        .then(a.start_column.cmp(&b.start_column))
    });
    for (idx, call) in scope_calls.into_iter().enumerate() {
      call.ordering = Some(idx);
    }
  }

  Ok(calls)
}

fn get_lambda_parameters(lambda_node: Node<'_>, source: &str) -> Vec<String> {
  let mut params = Vec::new();
  if let Some(params_node) = lambda_node.child_by_field_name("parameters") {
    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
      if child.kind() == "identifier" {
        params.push(child.utf8_text(source.as_bytes()).unwrap_or("").to_string());
      }
    }
  }
  params
}

fn compute_placeholders(params: &[String], num_pos: usize, keywords: &[String]) -> Option<String> {
  let mut placeholders = Vec::new();
  for (idx, param) in params.iter().enumerate() {
    if idx < num_pos {
      continue;
    }
    if keywords.contains(param) {
      continue;
    }
    placeholders.push(param.clone());
  }
  if placeholders.is_empty() {
    None
  } else {
    Some(placeholders.join(", "))
  }
}

fn get_call_arguments(args_node: Node<'_>, source: &str) -> (usize, Vec<String>) {
  let mut num_pos = 0;
  let mut keywords = Vec::new();
  let mut cursor = args_node.walk();
  for child in args_node.children(&mut cursor) {
    if child.is_named() {
      if child.kind() == "keyword_argument" {
        if let Some(name_node) = child.child_by_field_name("name") {
          let name = name_node
            .utf8_text(source.as_bytes())
            .unwrap_or("")
            .to_string();
          keywords.push(name);
        }
      } else {
        num_pos += 1;
      }
    }
  }
  (num_pos, keywords)
}

fn is_child_pipeline_stage(
  parent: Node<'_>,
  child: Node<'_>,
  source: &str,
  is_parent_pipeline: bool,
) -> bool {
  if !child.is_named() {
    return false;
  }
  let parent_kind = parent.kind();
  if parent_kind == "argument_list" {
    if let Some(call_node) = parent.parent() {
      if call_node.kind() == "call" {
        return is_child_pipeline_stage(call_node, child, source, is_parent_pipeline);
      }
    }
  }
  if parent_kind == "call" {
    if let Some(func_node) = parent.child_by_field_name("function") {
      let target_name = func_node
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .trim()
        .to_string();
      let mut method_name = None;
      if func_node.kind() == "attribute" {
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

      if let Some(arg_list) = parent.child_by_field_name("arguments") {
        let mut cursor = arg_list.walk();
        let mut child_idx = None;
        let mut idx = 0;
        for c in arg_list.children(&mut cursor) {
          if c.is_named() {
            if c.id() == child.id() {
              child_idx = Some(idx);
            }
            idx += 1;
          }
        }

        if let Some(c_idx) = child_idx {
          if target_name == "flow" || target_name == "pipe" {
            return c_idx > 0;
          }
          if target_name == "compose" {
            return true;
          }
          if target_name == "map"
            || target_name == "filter"
            || target_name == "reduce"
            || target_name == "flatMap"
            || target_name == "apply"
          {
            return c_idx == 0;
          }
          if let Some(ref m_name) = method_name {
            if m_name == "bind" || m_name == "then" || m_name == "pipe" {
              return true;
            }
          }
          return false;
        }
      }
    }
  } else if parent_kind == "binary_operator" {
    let mut op_text = String::new();
    let mut cursor = parent.walk();
    for c in parent.children(&mut cursor) {
      let text = c.utf8_text(source.as_bytes()).unwrap_or("").trim();
      if text == ">>" || text == ">>=" {
        op_text = text.to_string();
      }
    }
    if op_text == ">>" || op_text == ">>=" {
      return true;
    }
  }

  is_parent_pipeline
}

fn collect_file_functions_and_currying(
  node: Node<'_>,
  source: &str,
  file_functions: &mut std::collections::HashMap<String, Vec<String>>,
  curried_functions: &mut std::collections::HashSet<String>,
) {
  let kind = node.kind();
  match kind {
    "function_definition" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let func_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !func_name.is_empty() {
          let params = get_parameter_names(node, source);
          file_functions.insert(func_name.clone(), params);
          let decorators = get_decorators_for_node(node, source);
          if decorators.iter().any(|d| d.contains("curry")) {
            curried_functions.insert(func_name);
          }
        }
      }
    }
    "assignment" => {
      if let Some(left_node) = node.child_by_field_name("left") {
        if left_node.kind() == "identifier" {
          let var_name = left_node
            .utf8_text(source.as_bytes())
            .unwrap_or("")
            .to_string();
          if let Some(right_node) = node.child_by_field_name("value") {
            if right_node.kind() == "call" {
              if let Some(func_node) = right_node.child_by_field_name("function") {
                let func_name = func_node
                  .utf8_text(source.as_bytes())
                  .unwrap_or("")
                  .to_string();
                if func_name.contains("curry") {
                  curried_functions.insert(var_name.clone());
                  if let Some(arg_list) = right_node.child_by_field_name("arguments") {
                    let mut cursor = arg_list.walk();
                    let first_arg = arg_list.children(&mut cursor).find(|c| c.is_named());
                    if let Some(arg_node) = first_arg {
                      if arg_node.kind() == "lambda" {
                        let lambda_params = get_lambda_parameters(arg_node, source);
                        file_functions.insert(var_name, lambda_params);
                      } else if arg_node.kind() == "identifier" {
                        let target_func = arg_node
                          .utf8_text(source.as_bytes())
                          .unwrap_or("")
                          .to_string();
                        if let Some(params) = file_functions.get(&target_func) {
                          file_functions.insert(var_name, params.clone());
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
    _ => {}
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    collect_file_functions_and_currying(child, source, file_functions, curried_functions);
  }
}

fn current_scope_qname(module_name: &str, scope_stack: &[(String, bool)]) -> String {
  if scope_stack.is_empty() {
    module_name.to_string()
  } else {
    let mut qname = module_name.to_string();
    qname.push_str("::");
    for (i, (name, _is_class)) in scope_stack.iter().enumerate() {
      if i > 0 {
        qname.push('.');
      }
      qname.push_str(name);
    }
    qname
  }
}

fn get_callable_reference_name(node: Node<'_>, source: &str) -> Option<String> {
  match node.kind() {
    "identifier" => Some(
      node
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .trim()
        .to_string(),
    ),
    "attribute" => {
      let text = node
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .trim()
        .to_string();
      if !text.is_empty() { Some(text) } else { None }
    }
    "keyword_argument" => {
      if let Some(val_node) = node.child_by_field_name("value") {
        get_callable_reference_name(val_node, source)
      } else {
        None
      }
    }
    _ => None,
  }
}

#[allow(clippy::too_many_arguments)]
fn traverse_calls(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, bool)>,
  calls: &mut Vec<ExtractedCall>,
  is_pipeline_stage: bool,
  file_functions: &std::collections::HashMap<String, Vec<String>>,
  curried_functions: &std::collections::HashSet<String>,
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
          scope_stack.push((class_name, true));
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
          scope_stack.push((func_name, false));
          name_pushed = true;
        }
      }
    }
    "identifier" | "attribute" if is_pipeline_stage => {
      let target_name = node
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .trim()
        .to_string();
      if !target_name.is_empty() {
        let mut placeholders = None;
        if let Some(params) = file_functions.get(&target_name) {
          placeholders = compute_placeholders(params, 1, &[]);
        } else if curried_functions.contains(&target_name) {
          placeholders = Some("?".to_string());
        }
        let start = node.start_position();
        let end = node.end_position();
        let source_symbol_qualified_name = current_scope_qname(module_name, scope_stack);

        let exists = calls.iter().any(|c| {
          c.target_name == target_name
            && c.start_line == start.row + 1
            && c.start_column == start.column
        });

        if !exists {
          calls.push(ExtractedCall {
            source_symbol_qualified_name,
            target_name,
            is_self_call: false,
            method_name: None,
            start_line: start.row + 1,
            start_column: start.column,
            end_line: end.row + 1,
            end_column: end.column,
            ordering: None,
            placeholders,
          });
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

          let mut placeholders = None;
          let extra = if is_pipeline_stage { 1 } else { 0 };

          if target_name == "partial" || target_name == "functools.partial" {
            // Handled separately below, but still push call to partial for standard assertions.
          } else if let Some(arg_list) = node.child_by_field_name("arguments") {
            let (num_pos, keywords) = get_call_arguments(arg_list, source);
            if let Some(params) = file_functions.get(&target_name) {
              if curried_functions.contains(&target_name) {
                placeholders = compute_placeholders(params, num_pos + extra, &keywords);
              }
            } else if curried_functions.contains(&target_name) {
              placeholders = Some("?".to_string());
            }
          }

          let exists = calls.iter().any(|c| {
            c.target_name == target_name
              && c.start_line == start.row + 1
              && c.start_column == start.column
          });

          if !exists {
            calls.push(ExtractedCall {
              source_symbol_qualified_name: source_symbol_qualified_name.clone(),
              target_name: target_name.clone(),
              is_self_call,
              method_name: method_name.clone(),
              start_line: start.row + 1,
              start_column: start.column,
              end_line: end.row + 1,
              end_column: end.column,
              ordering: None,
              placeholders,
            });
          }

          if target_name == "partial" || target_name == "functools.partial" {
            if let Some(arg_list) = node.child_by_field_name("arguments") {
              let mut arg_cursor = arg_list.walk();
              let mut children = arg_list.children(&mut arg_cursor).filter(|c| c.is_named());
              if let Some(first_arg) = children.next() {
                if let Some(callable_name) = get_callable_reference_name(first_arg, source) {
                  let mut num_pos = 0;
                  let mut keywords = Vec::new();
                  for child in children {
                    if child.kind() == "keyword_argument" {
                      if let Some(name_node) = child.child_by_field_name("name") {
                        keywords.push(
                          name_node
                            .utf8_text(source.as_bytes())
                            .unwrap_or("")
                            .to_string(),
                        );
                      }
                    } else {
                      num_pos += 1;
                    }
                  }

                  let partial_placeholders =
                    if let Some(params) = file_functions.get(&callable_name) {
                      compute_placeholders(params, num_pos + extra, &keywords)
                    } else {
                      Some("?".to_string())
                    };

                  let start_arg = first_arg.start_position();
                  let end_arg = first_arg.end_position();
                  calls.push(ExtractedCall {
                    source_symbol_qualified_name: source_symbol_qualified_name.clone(),
                    target_name: callable_name,
                    is_self_call: false,
                    method_name: None,
                    start_line: start_arg.row + 1,
                    start_column: start_arg.column,
                    end_line: end_arg.row + 1,
                    end_column: end_arg.column,
                    ordering: None,
                    placeholders: partial_placeholders,
                  });
                }
              }
            }
          }
        }
      }
    }
    _ => {}
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    let is_child_pipeline = is_child_pipeline_stage(node, child, source, is_pipeline_stage);
    traverse_calls(
      child,
      source,
      module_name,
      scope_stack,
      calls,
      is_child_pipeline,
      file_functions,
      curried_functions,
    );
  }

  if name_pushed {
    scope_stack.pop();
  }
}

fn is_builtin_or_typing_type(name: &str) -> bool {
  let lower = name.to_lowercase();
  let builtins = [
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

  let lower_name = name.to_lowercase();
  if typing_wrappers.contains(&lower_name.as_str()) {
    return true;
  }

  false
}

fn collect_types_from_annotation_node(
  node: tree_sitter::Node<'_>,
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

fn collect_identifiers_from_node(
  node: tree_sitter::Node<'_>,
  source: &str,
  names: &mut Vec<String>,
) {
  if node.kind() == "identifier" {
    if let Ok(text) = node.utf8_text(source.as_bytes()) {
      names.push(text.trim().to_string());
    }
  } else {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
      collect_identifiers_from_node(child, source, names);
    }
  }
}

fn extract_shape_from_subscript(node: tree_sitter::Node<'_>, source: &str) -> Option<String> {
  let text = node.utf8_text(source.as_bytes()).ok()?.trim();
  if let Some(start_idx) = text.find('[') {
    if let Some(end_idx) = text.rfind(']') {
      if start_idx < end_idx {
        let inner = text[start_idx + 1..end_idx].trim();
        let cleaned = if (inner.starts_with('"') && inner.ends_with('"'))
          || (inner.starts_with('\'') && inner.ends_with('\''))
        {
          if inner.len() >= 2 {
            &inner[1..inner.len() - 1]
          } else {
            inner
          }
        } else {
          inner
        };
        let dims: Vec<String> = cleaned
          .split(',')
          .map(|s| s.trim().to_string())
          .filter(|s| !s.is_empty())
          .collect();
        if !dims.is_empty() {
          return Some(format!("[{}]", dims.join(", ")));
        }
      }
    }
  }
  None
}

fn find_assignment_in_node(
  node: tree_sitter::Node<'_>,
  comment_row: usize,
  source: &str,
) -> Option<String> {
  let start_row = node.start_position().row;
  if start_row > comment_row + 1 {
    return None;
  }
  if start_row == comment_row || start_row == comment_row + 1 {
    if node.kind() == "assignment" {
      if let Some(left) = node.child(0) {
        let mut ids = Vec::new();
        collect_identifiers_from_node(left, source, &mut ids);
        if !ids.is_empty() {
          return Some(ids[0].clone());
        }
      }
    }
  }
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if let Some(name) = find_assignment_in_node(child, comment_row, source) {
      return Some(name);
    }
  }
  None
}

fn find_variable_name_near_comment(
  comment_node: tree_sitter::Node<'_>,
  source: &str,
) -> Option<String> {
  let comment_row = comment_node.start_position().row;
  if let Some(parent) = comment_node.parent() {
    return find_assignment_in_node(parent, comment_row, source);
  }
  None
}

fn extract_shape_from_comment(text: &str) -> Option<String> {
  let trimmed = text.trim();
  if let Some(idx) = trimmed.find("shape:") {
    let val = trimmed[idx + 6..].trim();
    let cleaned = val
      .trim_matches(|c| c == '(' || c == ')' || c == '[' || c == ']' || c == '\'' || c == '"')
      .trim();
    let dims: Vec<String> = cleaned
      .split(',')
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty())
      .collect();
    if !dims.is_empty() {
      return Some(format!("[{}]", dims.join(", ")));
    }
  } else if let Some(idx) = trimmed.find("Tensor[") {
    let val = &trimmed[idx + 7..];
    if let Some(end_idx) = val.find(']') {
      let inner = val[..end_idx].trim();
      let cleaned = inner.trim_matches(|c| c == '\'' || c == '"').trim();
      let dims: Vec<String> = cleaned
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
      if !dims.is_empty() {
        return Some(format!("[{}]", dims.join(", ")));
      }
    }
  }
  None
}

fn collect_column_references_in_node(
  n: tree_sitter::Node<'_>,
  source: &str,
  cols: &mut Vec<String>,
) {
  if n.kind() == "call" {
    if let Some(func_node) = n.child_by_field_name("function") {
      let func_text = func_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
      if func_text == "col" || func_text == "pl.col" || func_text.ends_with(".col") {
        if let Some(arg_list) = n
          .children(&mut n.walk())
          .find(|c| c.kind() == "argument_list")
        {
          let mut arg_cursor = arg_list.walk();
          for child in arg_list.children(&mut arg_cursor) {
            if child.kind() == "string" {
              let text = child.utf8_text(source.as_bytes()).unwrap_or("");
              let stripped = text.trim().trim_matches(|c| c == '\'' || c == '"');
              if !stripped.is_empty() {
                cols.push(stripped.to_string());
              }
            }
          }
        }
        return; // Stop recursion inside this col call!
      }
    }
  } else if n.kind() == "string" {
    let text = n.utf8_text(source.as_bytes()).unwrap_or("");
    let stripped = text.trim().trim_matches(|c| c == '\'' || c == '"');
    if !stripped.is_empty() && stripped.chars().all(|c| c.is_alphanumeric() || c == '_') {
      cols.push(stripped.to_string());
    }
    return; // Stop recursion inside string literal!
  }
  let mut cursor = n.walk();
  for child in n.children(&mut cursor) {
    collect_column_references_in_node(child, source, cols);
  }
}

fn traverse_type_references(
  node: tree_sitter::Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, bool)>,
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
          scope_stack.push((class_name.clone(), true));
          name_pushed = true;

          let qname = current_scope_qname(module_name, scope_stack);

          // PyTorch submodule type reference extraction
          if is_pytorch_module_class(node, source) {
            if let Some(init_method) = find_init_method(node, source) {
              let mut assignments = Vec::new();
              collect_self_assignments(init_method, source, &mut assignments);
              for (attr_name, _right_text, right_node, _assign_node) in assignments {
                if let Some(func_node) = right_node.child_by_field_name("function") {
                  let type_name = func_node
                    .utf8_text(source.as_bytes())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                  if !type_name.is_empty() && !is_builtin_or_typing_type(&type_name) {
                    let submodule_qname = format!("{}.{}", qname, attr_name);
                    let sub_start = right_node.start_position();
                    let sub_end = right_node.end_position();
                    type_refs.push(ExtractedTypeReference {
                      source_symbol_qualified_name: submodule_qname,
                      target_type_name: type_name,
                      start_line: sub_start.row + 1,
                      start_column: sub_start.column,
                      end_line: sub_end.row + 1,
                      end_column: sub_end.column,
                    });
                  }
                }
              }
            }
          }

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
    "function_definition" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let func_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !func_name.is_empty() {
          scope_stack.push((func_name.clone(), false));
          name_pushed = true;

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
      let mut param_name = String::new();
      if let Some(id_node) = node.child_by_field_name("name") {
        if let Ok(text) = id_node.utf8_text(source.as_bytes()) {
          param_name = text.trim().to_string();
        }
      } else {
        let mut ids = Vec::new();
        collect_identifiers_from_node(node, source, &mut ids);
        if !ids.is_empty() {
          param_name = ids[0].clone();
        }
      }

      let mut cursor = node.walk();
      for child in node.children(&mut cursor) {
        if child.kind() == "type" {
          let type_text = child.utf8_text(source.as_bytes()).unwrap_or("").trim();
          if type_text.contains("Tensor") || type_text.contains("Parameter") {
            if let Some(shape_dims) = extract_shape_from_subscript(child, source) {
              if !param_name.is_empty() {
                let source_symbol_qualified_name = current_scope_qname(module_name, scope_stack);
                type_refs.push(ExtractedTypeReference {
                  source_symbol_qualified_name: source_symbol_qualified_name.clone(),
                  target_type_name: format!("shape::{}::{}", param_name, shape_dims),
                  start_line: child.start_position().row + 1,
                  start_column: child.start_position().column,
                  end_line: child.end_position().row + 1,
                  end_column: child.end_position().column,
                });
                type_refs.push(ExtractedTypeReference {
                  source_symbol_qualified_name,
                  target_type_name: format!("shape::{}", shape_dims),
                  start_line: child.start_position().row + 1,
                  start_column: child.start_position().column,
                  end_line: child.end_position().row + 1,
                  end_column: child.end_position().column,
                });
              }
            }
          }

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
      let right_node = node.child_by_field_name("value").or_else(|| node.child(2));
      if let Some(right) = right_node {
        let right_text = right.utf8_text(source.as_bytes()).unwrap_or("").trim();
        if right.kind() == "attribute" && right_text.ends_with(".shape") {
          if let Some(obj_node) = right.child_by_field_name("object") {
            let obj_name = obj_node
              .utf8_text(source.as_bytes())
              .unwrap_or("")
              .trim()
              .to_string();
            if !obj_name.is_empty() {
              if let Some(left) = node.child(0) {
                let mut ids = Vec::new();
                collect_identifiers_from_node(left, source, &mut ids);
                if !ids.is_empty() {
                  let shape_dims = format!("[{}]", ids.join(", "));
                  let source_symbol_qualified_name = current_scope_qname(module_name, scope_stack);
                  type_refs.push(ExtractedTypeReference {
                    source_symbol_qualified_name: source_symbol_qualified_name.clone(),
                    target_type_name: format!("shape::{}::{}", obj_name, shape_dims),
                    start_line: node.start_position().row + 1,
                    start_column: node.start_position().column,
                    end_line: node.end_position().row + 1,
                    end_column: node.end_position().column,
                  });
                  type_refs.push(ExtractedTypeReference {
                    source_symbol_qualified_name,
                    target_type_name: format!("shape::{}", shape_dims),
                    start_line: node.start_position().row + 1,
                    start_column: node.start_position().column,
                    end_line: node.end_position().row + 1,
                    end_column: node.end_position().column,
                  });
                }
              }
            }
          }
        }
      }

      let mut is_direct_col_assign = false;
      if let Some(left) = node.child(0) {
        if left.kind() == "subscript" {
          let mut value_node = left.child_by_field_name("value");
          let mut idx_node = left.child_by_field_name("subscript");

          if value_node.is_none() || idx_node.is_none() {
            let mut val_node = None;
            let mut index_node = None;
            let mut found_bracket = false;
            let mut cursor = left.walk();
            for child in left.children(&mut cursor) {
              if child.kind() == "[" {
                found_bracket = true;
              } else if !found_bracket {
                val_node = Some(child);
              } else if found_bracket && child.kind() != "]" && index_node.is_none() {
                index_node = Some(child);
              }
            }
            if value_node.is_none() {
              value_node = val_node;
            }
            if idx_node.is_none() {
              idx_node = index_node;
            }
          }

          if let Some(val_node) = value_node {
            let val_text = val_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
            if val_text.contains("df") || val_text.contains("data") {
              if let Some(i_node) = idx_node {
                let idx_text = i_node.utf8_text(source.as_bytes()).unwrap_or("");
                let derived_col = idx_text.trim().trim_matches(|c| c == '\'' || c == '"');
                if !derived_col.is_empty()
                  && derived_col.chars().all(|c| c.is_alphanumeric() || c == '_')
                {
                  let right_node = node.child_by_field_name("value").or_else(|| node.child(2));
                  if let Some(right) = right_node {
                    let mut src_cols = Vec::new();
                    collect_column_references_in_node(right, source, &mut src_cols);
                    src_cols.sort();
                    src_cols.dedup();
                    if !src_cols.is_empty() {
                      let source_symbol_qualified_name =
                        current_scope_qname(module_name, scope_stack);
                      type_refs.push(ExtractedTypeReference {
                        source_symbol_qualified_name: source_symbol_qualified_name.clone(),
                        target_type_name: format!(
                          "col::{} <- assign(col::{})",
                          derived_col,
                          src_cols.join(", col::")
                        ),
                        start_line: node.start_position().row + 1,
                        start_column: node.start_position().column,
                        end_line: node.end_position().row + 1,
                        end_column: node.end_position().column,
                      });
                      type_refs.push(ExtractedTypeReference {
                        source_symbol_qualified_name,
                        target_type_name: format!("col::{}", derived_col),
                        start_line: node.start_position().row + 1,
                        start_column: node.start_position().column,
                        end_line: node.end_position().row + 1,
                        end_column: node.end_position().column,
                      });
                      is_direct_col_assign = true;
                    }
                  }
                }
              }
            }
          }
        }
      }

      if !is_direct_col_assign {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
          if child.kind() == "type" {
            let type_text = child.utf8_text(source.as_bytes()).unwrap_or("").trim();
            if type_text.contains("Tensor") || type_text.contains("Parameter") {
              if let Some(shape_dims) = extract_shape_from_subscript(child, source) {
                let mut var_name = String::new();
                if let Some(left) = node.child(0) {
                  let mut ids = Vec::new();
                  collect_identifiers_from_node(left, source, &mut ids);
                  if !ids.is_empty() {
                    var_name = ids[0].clone();
                  }
                }
                if !var_name.is_empty() {
                  let source_symbol_qualified_name = current_scope_qname(module_name, scope_stack);
                  type_refs.push(ExtractedTypeReference {
                    source_symbol_qualified_name: source_symbol_qualified_name.clone(),
                    target_type_name: format!("shape::{}::{}", var_name, shape_dims),
                    start_line: child.start_position().row + 1,
                    start_column: child.start_position().column,
                    end_line: child.end_position().row + 1,
                    end_column: child.end_position().column,
                  });
                  type_refs.push(ExtractedTypeReference {
                    source_symbol_qualified_name,
                    target_type_name: format!("shape::{}", shape_dims),
                    start_line: child.start_position().row + 1,
                    start_column: child.start_position().column,
                    end_line: child.end_position().row + 1,
                    end_column: child.end_position().column,
                  });
                }
              }
            }

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
    "call" => {
      if let Some(func_node) = node.child_by_field_name("function") {
        let mut method_name = String::new();
        if func_node.kind() == "attribute" {
          if let Some(attr_node) = func_node.child_by_field_name("attribute") {
            method_name = attr_node
              .utf8_text(source.as_bytes())
              .unwrap_or("")
              .trim()
              .to_string();
          }
        } else if func_node.kind() == "identifier" {
          method_name = func_node
            .utf8_text(source.as_bytes())
            .unwrap_or("")
            .trim()
            .to_string();
        }

        if !method_name.is_empty() {
          if method_name == "select"
            || method_name == "with_columns"
            || method_name == "filter"
            || method_name == "groupby"
            || method_name == "group_by"
            || method_name == "agg"
            || method_name == "drop"
            || method_name == "rename"
            || method_name == "join"
            || method_name == "merge"
            || method_name == "alias"
          {
            if let Some(arg_list) = node
              .children(&mut node.walk())
              .find(|c| c.kind() == "argument_list")
            {
              let source_symbol_qualified_name = current_scope_qname(module_name, scope_stack);

              if method_name == "with_columns" || method_name == "select" || method_name == "agg" {
                let mut cursor = arg_list.walk();
                for child in arg_list.children(&mut cursor) {
                  if child.kind() == "keyword_argument" {
                    if let (Some(name_node), Some(val_node)) = (
                      child.child_by_field_name("name"),
                      child.child_by_field_name("value"),
                    ) {
                      let derived_col = name_node
                        .utf8_text(source.as_bytes())
                        .unwrap_or("")
                        .trim()
                        .to_string();
                      if !derived_col.is_empty() {
                        let mut src_cols = Vec::new();
                        collect_column_references_in_node(val_node, source, &mut src_cols);
                        src_cols.sort();
                        src_cols.dedup();
                        if !src_cols.is_empty() {
                          type_refs.push(ExtractedTypeReference {
                            source_symbol_qualified_name: source_symbol_qualified_name.clone(),
                            target_type_name: format!(
                              "col::{} <- {}(col::{})",
                              derived_col,
                              method_name,
                              src_cols.join(", col::")
                            ),
                            start_line: child.start_position().row + 1,
                            start_column: child.start_position().column,
                            end_line: child.end_position().row + 1,
                            end_column: child.end_position().column,
                          });
                          type_refs.push(ExtractedTypeReference {
                            source_symbol_qualified_name: source_symbol_qualified_name.clone(),
                            target_type_name: format!("col::{}", derived_col),
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

              if method_name == "alias" {
                let mut alias_name = String::new();
                let mut cursor = arg_list.walk();
                for child in arg_list.children(&mut cursor) {
                  if child.kind() == "string" {
                    alias_name = child
                      .utf8_text(source.as_bytes())
                      .unwrap_or("")
                      .trim()
                      .trim_matches(|c| c == '\'' || c == '"')
                      .to_string();
                    break;
                  }
                }
                if !alias_name.is_empty() {
                  if let Some(obj_node) = func_node.child_by_field_name("object") {
                    let mut src_cols = Vec::new();
                    collect_column_references_in_node(obj_node, source, &mut src_cols);
                    src_cols.sort();
                    src_cols.dedup();
                    if !src_cols.is_empty() {
                      type_refs.push(ExtractedTypeReference {
                        source_symbol_qualified_name: source_symbol_qualified_name.clone(),
                        target_type_name: format!(
                          "col::{} <- select(col::{})",
                          alias_name,
                          src_cols.join(", col::")
                        ),
                        start_line: node.start_position().row + 1,
                        start_column: node.start_position().column,
                        end_line: node.end_position().row + 1,
                        end_column: node.end_position().column,
                      });
                      type_refs.push(ExtractedTypeReference {
                        source_symbol_qualified_name: source_symbol_qualified_name.clone(),
                        target_type_name: format!("col::{}", alias_name),
                        start_line: node.start_position().row + 1,
                        start_column: node.start_position().column,
                        end_line: node.end_position().row + 1,
                        end_column: node.end_position().column,
                      });
                    }
                  }
                }
              }

              if method_name == "rename" {
                let mut cursor = arg_list.walk();
                for child in arg_list.children(&mut cursor) {
                  if child.kind() == "dictionary" {
                    let mut dict_cursor = child.walk();
                    for pair in child.children(&mut dict_cursor) {
                      if pair.kind() == "pair" {
                        if let (Some(key), Some(val)) = (pair.child(0), pair.child(2)) {
                          let key_text = key.utf8_text(source.as_bytes()).unwrap_or("");
                          let val_text = val.utf8_text(source.as_bytes()).unwrap_or("");
                          let src_col = key_text.trim().trim_matches(|c| c == '\'' || c == '"');
                          let derived_col = val_text.trim().trim_matches(|c| c == '\'' || c == '"');
                          if !src_col.is_empty() && !derived_col.is_empty() {
                            type_refs.push(ExtractedTypeReference {
                              source_symbol_qualified_name: source_symbol_qualified_name.clone(),
                              target_type_name: format!(
                                "col::{} <- rename(col::{})",
                                derived_col, src_col
                              ),
                              start_line: pair.start_position().row + 1,
                              start_column: pair.start_position().column,
                              end_line: pair.end_position().row + 1,
                              end_column: pair.end_position().column,
                            });
                            type_refs.push(ExtractedTypeReference {
                              source_symbol_qualified_name: source_symbol_qualified_name.clone(),
                              target_type_name: format!("col::{}", derived_col),
                              start_line: pair.start_position().row + 1,
                              start_column: pair.start_position().column,
                              end_line: pair.end_position().row + 1,
                              end_column: pair.end_position().column,
                            });
                          }
                        }
                      }
                    }
                  }
                }
              }

              let mut columns = Vec::new();
              collect_column_names_from_arguments(arg_list, source, &mut columns);
              for (column_name, range) in columns {
                type_refs.push(ExtractedTypeReference {
                  source_symbol_qualified_name: source_symbol_qualified_name.clone(),
                  target_type_name: format!("col::{}", column_name),
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
    "comment" => {
      let text = node.utf8_text(source.as_bytes()).unwrap_or("");
      if let Some(shape_dims) = extract_shape_from_comment(text) {
        let var_name = find_variable_name_near_comment(node, source).unwrap_or_default();
        let source_symbol_qualified_name = current_scope_qname(module_name, scope_stack);
        if !var_name.is_empty() {
          type_refs.push(ExtractedTypeReference {
            source_symbol_qualified_name: source_symbol_qualified_name.clone(),
            target_type_name: format!("shape::{}::{}", var_name, shape_dims),
            start_line: node.start_position().row + 1,
            start_column: node.start_position().column,
            end_line: node.end_position().row + 1,
            end_column: node.end_position().column,
          });
        }
        type_refs.push(ExtractedTypeReference {
          source_symbol_qualified_name,
          target_type_name: format!("shape::{}", shape_dims),
          start_line: node.start_position().row + 1,
          start_column: node.start_position().column,
          end_line: node.end_position().row + 1,
          end_column: node.end_position().column,
        });
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

pub fn extract_type_references_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedTypeReference>, PythonParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let module_name = module_name_from_path(file_path);
  let mut type_refs = Vec::new();
  let mut scope_stack = Vec::new();

  traverse_type_references(root, source, &module_name, &mut scope_stack, &mut type_refs);

  Ok(type_refs)
}

fn extract_decorator_info(
  node: Node<'_>,
  source: &str,
  source_symbol_qname: &str,
) -> Option<ExtractedDecorator> {
  let mut cursor = node.walk();
  let expr_node = node.children(&mut cursor).find(|c| c.kind() != "@")?;

  let decorator_name = if expr_node.kind() == "call" {
    if let Some(func_node) = expr_node.child_by_field_name("function") {
      func_node
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .trim()
        .to_string()
    } else {
      expr_node
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .trim()
        .to_string()
    }
  } else {
    expr_node
      .utf8_text(source.as_bytes())
      .unwrap_or("")
      .trim()
      .to_string()
  };

  if decorator_name.is_empty() {
    return None;
  }

  let start = node.start_position();
  let end = node.end_position();

  Some(ExtractedDecorator {
    source_symbol_qualified_name: source_symbol_qname.to_string(),
    decorator_name,
    start_line: start.row + 1,
    start_column: start.column,
    end_line: end.row + 1,
    end_column: end.column,
  })
}

fn traverse_decorators(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, bool)>,
  decorators: &mut Vec<ExtractedDecorator>,
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
          scope_stack.push((class_name.clone(), true));
          name_pushed = true;

          if let Some(parent) = node.parent().filter(|p| p.kind() == "decorated_definition") {
            let mut cursor = parent.walk();
            for child in parent.children(&mut cursor) {
              if child.kind() == "decorator" {
                let dec_opt = extract_decorator_info(
                  child,
                  source,
                  &current_scope_qname(module_name, scope_stack),
                );
                if let Some(dec) = dec_opt {
                  decorators.push(dec);
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
          scope_stack.push((func_name.clone(), false));
          name_pushed = true;

          if let Some(parent) = node.parent().filter(|p| p.kind() == "decorated_definition") {
            let mut cursor = parent.walk();
            for child in parent.children(&mut cursor) {
              if child.kind() == "decorator" {
                let dec_opt = extract_decorator_info(
                  child,
                  source,
                  &current_scope_qname(module_name, scope_stack),
                );
                if let Some(dec) = dec_opt {
                  decorators.push(dec);
                }
              }
            }
          }
        }
      }
    }
    _ => {}
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_decorators(child, source, module_name, scope_stack, decorators);
  }

  if name_pushed {
    scope_stack.pop();
  }
}

pub fn extract_decorators_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedDecorator>, PythonParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let module_name = module_name_from_path(file_path);
  let mut decorators = Vec::new();
  let mut scope_stack = Vec::new();

  traverse_decorators(
    root,
    source,
    &module_name,
    &mut scope_stack,
    &mut decorators,
  );

  Ok(decorators)
}

fn get_decorators_for_node(node: Node<'_>, source: &str) -> Vec<String> {
  let mut names = Vec::new();
  if let Some(parent) = node.parent().filter(|p| p.kind() == "decorated_definition") {
    let mut cursor = parent.walk();
    for child in parent.children(&mut cursor) {
      if child.kind() == "decorator" {
        let dec_opt = extract_decorator_info(child, source, "");
        if let Some(dec) = dec_opt {
          names.push(dec.decorator_name);
        }
      }
    }
  }
  names
}

fn get_parameter_names(node: Node<'_>, source: &str) -> Vec<String> {
  let mut params = Vec::new();
  if let Some(params_node) = node.child_by_field_name("parameters") {
    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
      match child.kind() {
        "identifier" => {
          let name = child
            .utf8_text(source.as_bytes())
            .unwrap_or("")
            .trim()
            .to_string();
          if !name.is_empty() && name != "self" {
            params.push(name);
          }
        }
        "typed_parameter" => {
          if let Some(id_node) = child.child_by_field_name("name") {
            let name = id_node
              .utf8_text(source.as_bytes())
              .unwrap_or("")
              .trim()
              .to_string();
            if !name.is_empty() && name != "self" {
              params.push(name);
            }
          }
        }
        "default_parameter" | "typed_default_parameter" => {
          if let Some(id_node) = child.child_by_field_name("name") {
            let name = id_node
              .utf8_text(source.as_bytes())
              .unwrap_or("")
              .trim()
              .to_string();
            if !name.is_empty() && name != "self" {
              params.push(name);
            }
          }
        }
        _ => {}
      }
    }
  }
  params
}

fn get_parametrized_args(node: Node<'_>, source: &str) -> Vec<String> {
  let mut args = Vec::new();
  if let Some(parent) = node.parent().filter(|p| p.kind() == "decorated_definition") {
    let mut cursor = parent.walk();
    for child in parent.children(&mut cursor) {
      if child.kind() == "decorator" {
        let mut expr_cursor = child.walk();
        let call_and_func = child
          .children(&mut expr_cursor)
          .find(|c| c.kind() == "call")
          .and_then(|call| {
            call
              .child_by_field_name("function")
              .map(|func| (call, func))
          });

        if let Some((call_node, func_node)) = call_and_func {
          let func_name = func_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
          let is_parametrize = func_name == "parametrize" || func_name == "pytest.mark.parametrize";
          let arg_list_opt = if is_parametrize {
            call_node.child_by_field_name("arguments")
          } else {
            None
          };

          if let Some(arg_list) = arg_list_opt {
            let mut arg_cursor = arg_list.walk();
            if let Some(first_arg) = arg_list
              .children(&mut arg_cursor)
              .find(|c| c.kind() != "(" && c.kind() != ")" && c.kind() != ",")
            {
              match first_arg.kind() {
                "string" => {
                  let raw_text = first_arg.utf8_text(source.as_bytes()).unwrap_or("").trim();
                  if raw_text.len() >= 2 {
                    let inner = &raw_text[1..raw_text.len() - 1];
                    for part in inner.split(',') {
                      let clean = part.trim().to_string();
                      if !clean.is_empty() {
                        args.push(clean);
                      }
                    }
                  }
                }
                "list" | "tuple" => {
                  let mut list_cursor = first_arg.walk();
                  for item in first_arg.children(&mut list_cursor) {
                    if item.kind() == "string" {
                      let raw_text = item.utf8_text(source.as_bytes()).unwrap_or("").trim();
                      if raw_text.len() >= 2 {
                        let clean = raw_text[1..raw_text.len() - 1].trim().to_string();
                        if !clean.is_empty() {
                          args.push(clean);
                        }
                      }
                    }
                  }
                }
                _ => {}
              }
            }
          }
        }
      }
    }
  }
  args
}

fn traverse_tests(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, bool)>,
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
          scope_stack.push((class_name.clone(), true));
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
          scope_stack.push((func_name.clone(), false));
          name_pushed = true;

          let decorators = get_decorators_for_node(node, source);
          let is_fixture = decorators
            .iter()
            .any(|d| d == "fixture" || d == "pytest.fixture");
          let is_test_function = func_name.starts_with("test_");

          if is_fixture || is_test_function {
            let qname = current_scope_qname(module_name, scope_stack);
            let is_parametrized = decorators
              .iter()
              .any(|d| d == "parametrize" || d == "pytest.mark.parametrize");
            let test_kind = if is_fixture {
              ExtractedTestKind::Fixture
            } else {
              ExtractedTestKind::Function
            };

            let start = node.start_position();
            let end = node.end_position();
            let mut parameters = get_parameter_names(node, source);
            let parametrized_args = get_parametrized_args(node, source);
            parameters.retain(|p| !parametrized_args.contains(p));

            tests.push(ExtractedTest {
              name: func_name,
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

pub fn extract_tests_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedTest>, PythonParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let module_name = module_name_from_path(file_path);
  let mut tests = Vec::new();
  let mut scope_stack = Vec::new();

  traverse_tests(root, source, &module_name, &mut scope_stack, &mut tests);

  Ok(tests)
}

#[allow(clippy::collapsible_if)]
fn traverse_for_blueprints(
  node: Node<'_>,
  source: &str,
  blueprints: &mut std::collections::HashMap<String, String>,
) {
  if node.kind() == "assignment" {
    if let Some(left_node) = node.child(0) {
      let var_name = left_node
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .trim()
        .to_string();
      if let Some(right_node) = node.child(node.child_count() - 1) {
        if right_node.kind() == "call" {
          if let Some(func_node) = right_node.child_by_field_name("function") {
            let func_name = func_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
            if func_name == "Blueprint" || func_name.ends_with(".Blueprint") {
              let mut url_prefix = String::new();
              if let Some(arg_list) = right_node.child_by_field_name("arguments") {
                let mut arg_cursor = arg_list.walk();
                for child_arg in arg_list.children(&mut arg_cursor) {
                  if child_arg.kind() == "keyword_argument" {
                    if let (Some(k), Some(v)) = (
                      child_arg.child_by_field_name("name"),
                      child_arg.child_by_field_name("value"),
                    ) {
                      let k_text = k.utf8_text(source.as_bytes()).unwrap_or("").trim();
                      if k_text == "url_prefix" {
                        let raw_prefix = v.utf8_text(source.as_bytes()).unwrap_or("").trim();
                        url_prefix = if (raw_prefix.starts_with('\'') && raw_prefix.ends_with('\''))
                          || (raw_prefix.starts_with('"') && raw_prefix.ends_with('"'))
                        {
                          if raw_prefix.len() >= 2 {
                            raw_prefix[1..raw_prefix.len() - 1].to_string()
                          } else {
                            raw_prefix.to_string()
                          }
                        } else {
                          raw_prefix.to_string()
                        };
                      }
                    }
                  }
                }
              }
              blueprints.insert(var_name, url_prefix);
            }
          }
        }
      }
    }
  } else if node.kind() == "call" {
    if let Some(func_node) = node.child_by_field_name("function") {
      let func_text = func_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
      if func_text.ends_with(".register_blueprint") {
        let mut bp_var_name = None;
        let mut url_prefix = None;
        if let Some(arg_list) = node.child_by_field_name("arguments") {
          let mut arg_cursor = arg_list.walk();
          let mut is_first = true;
          for child_arg in arg_list.children(&mut arg_cursor) {
            let kind_arg = child_arg.kind();
            if kind_arg != "(" && kind_arg != ")" && kind_arg != "," {
              if kind_arg == "keyword_argument" {
                if let (Some(k), Some(v)) = (
                  child_arg.child_by_field_name("name"),
                  child_arg.child_by_field_name("value"),
                ) {
                  let k_text = k.utf8_text(source.as_bytes()).unwrap_or("").trim();
                  if k_text == "blueprint" {
                    if v.kind() == "identifier" {
                      bp_var_name = Some(
                        v.utf8_text(source.as_bytes())
                          .unwrap_or("")
                          .trim()
                          .to_string(),
                      );
                    }
                  } else if k_text == "url_prefix" {
                    let raw_prefix = v.utf8_text(source.as_bytes()).unwrap_or("").trim();
                    url_prefix = Some(
                      if (raw_prefix.starts_with('\'') && raw_prefix.ends_with('\''))
                        || (raw_prefix.starts_with('"') && raw_prefix.ends_with('"'))
                      {
                        if raw_prefix.len() >= 2 {
                          raw_prefix[1..raw_prefix.len() - 1].to_string()
                        } else {
                          raw_prefix.to_string()
                        }
                      } else {
                        raw_prefix.to_string()
                      },
                    );
                  }
                }
              } else if is_first {
                if kind_arg == "identifier" {
                  bp_var_name = Some(
                    child_arg
                      .utf8_text(source.as_bytes())
                      .unwrap_or("")
                      .trim()
                      .to_string(),
                  );
                }
                is_first = false;
              }
            }
          }
        }
        if let (Some(bp_var), Some(prefix)) = (bp_var_name, url_prefix) {
          blueprints.insert(bp_var, prefix);
        }
      }
    }
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_for_blueprints(child, source, blueprints);
  }
}

#[allow(clippy::type_complexity)]
pub fn extract_routes_and_dependencies_from_source(
  source: &str,
  file_path: &str,
) -> Result<(Vec<ExtractedRoute>, Vec<(String, String)>), PythonParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let module_name = module_name_from_path(file_path);
  let mut routes = Vec::new();
  let mut dependencies = Vec::new();
  let mut scope_stack = Vec::new();

  let mut blueprints = std::collections::HashMap::new();
  traverse_for_blueprints(root, source, &mut blueprints);

  traverse_routes(
    root,
    source,
    &module_name,
    &mut scope_stack,
    &mut routes,
    &mut dependencies,
    &blueprints,
  );

  Ok((routes, dependencies))
}

#[allow(clippy::collapsible_if)]
fn traverse_routes(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, bool)>,
  routes: &mut Vec<ExtractedRoute>,
  dependencies: &mut Vec<(String, String)>,
  blueprints: &std::collections::HashMap<String, String>,
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
          scope_stack.push((class_name, true));
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
          scope_stack.push((func_name.clone(), false));
          name_pushed = true;

          let qname = current_scope_qname(module_name, scope_stack);

          if let Some(parent) = node.parent().filter(|p| p.kind() == "decorated_definition") {
            let mut cursor = parent.walk();
            for child in parent.children(&mut cursor) {
              if child.kind() == "decorator" {
                let mut expr_cursor = child.walk();
                if let Some(expr_node) = child.children(&mut expr_cursor).find(|c| c.kind() != "@")
                {
                  if expr_node.kind() == "call" {
                    if let Some(func_node) = expr_node.child_by_field_name("function") {
                      let decorator_name = func_node
                        .utf8_text(source.as_bytes())
                        .unwrap_or("")
                        .trim()
                        .to_string();

                      let lower = decorator_name.to_lowercase();
                      let is_route = lower.ends_with(".get")
                        || lower == "get"
                        || lower.ends_with(".post")
                        || lower == "post"
                        || lower.ends_with(".put")
                        || lower == "put"
                        || lower.ends_with(".delete")
                        || lower == "delete"
                        || lower.ends_with(".patch")
                        || lower == "patch"
                        || lower.ends_with(".options")
                        || lower == "options"
                        || lower.ends_with(".head")
                        || lower == "head"
                        || lower.ends_with(".route")
                        || lower == "route";

                      if is_route {
                        let methods = if lower.ends_with(".get") || lower == "get" {
                          vec!["GET".to_string()]
                        } else if lower.ends_with(".post") || lower == "post" {
                          vec!["POST".to_string()]
                        } else if lower.ends_with(".put") || lower == "put" {
                          vec!["PUT".to_string()]
                        } else if lower.ends_with(".delete") || lower == "delete" {
                          vec!["DELETE".to_string()]
                        } else if lower.ends_with(".patch") || lower == "patch" {
                          vec!["PATCH".to_string()]
                        } else if lower.ends_with(".options") || lower == "options" {
                          vec!["OPTIONS".to_string()]
                        } else if lower.ends_with(".head") || lower == "head" {
                          vec!["HEAD".to_string()]
                        } else {
                          // Check for methods argument in keywords
                          let mut custom_methods = Vec::new();
                          if let Some(arg_list) = expr_node.child_by_field_name("arguments") {
                            let mut kw_cursor = arg_list.walk();
                            for child_arg in arg_list.children(&mut kw_cursor) {
                              if child_arg.kind() == "keyword_argument" {
                                if let (Some(k), Some(v)) = (
                                  child_arg.child_by_field_name("name"),
                                  child_arg.child_by_field_name("value"),
                                ) {
                                  let k_text = k.utf8_text(source.as_bytes()).unwrap_or("").trim();
                                  if k_text == "methods" {
                                    match v.kind() {
                                      "string" => {
                                        let raw_text =
                                          v.utf8_text(source.as_bytes()).unwrap_or("").trim();
                                        if raw_text.len() >= 2 {
                                          let clean =
                                            raw_text[1..raw_text.len() - 1].trim().to_uppercase();
                                          if !clean.is_empty() {
                                            custom_methods.push(clean);
                                          }
                                        }
                                      }
                                      "list" | "tuple" => {
                                        let mut list_cursor = v.walk();
                                        for item in v.children(&mut list_cursor) {
                                          if item.kind() == "string" {
                                            let raw_text = item
                                              .utf8_text(source.as_bytes())
                                              .unwrap_or("")
                                              .trim();
                                            if raw_text.len() >= 2 {
                                              let clean = raw_text[1..raw_text.len() - 1]
                                                .trim()
                                                .to_uppercase();
                                              if !clean.is_empty() {
                                                custom_methods.push(clean);
                                              }
                                            }
                                          }
                                        }
                                      }
                                      _ => {
                                        let v_text =
                                          v.utf8_text(source.as_bytes()).unwrap_or("").trim();
                                        if v_text.contains("POST") || v_text.contains("post") {
                                          custom_methods.push("POST".to_string());
                                        }
                                        if v_text.contains("PUT") || v_text.contains("put") {
                                          custom_methods.push("PUT".to_string());
                                        }
                                        if v_text.contains("DELETE") || v_text.contains("delete") {
                                          custom_methods.push("DELETE".to_string());
                                        }
                                        if v_text.contains("PATCH") || v_text.contains("patch") {
                                          custom_methods.push("PATCH".to_string());
                                        }
                                        if v_text.contains("GET") || v_text.contains("get") {
                                          custom_methods.push("GET".to_string());
                                        }
                                      }
                                    }
                                  }
                                }
                              }
                            }
                          }
                          if custom_methods.is_empty() {
                            vec!["GET".to_string()]
                          } else {
                            custom_methods
                          }
                        };

                        let mut path = "/".to_string();
                        let mut response_model = None;
                        if let Some(arg_list) = expr_node.child_by_field_name("arguments") {
                          let mut arg_cursor = arg_list.walk();
                          let mut is_first = true;
                          for child_arg in arg_list.children(&mut arg_cursor) {
                            let kind_arg = child_arg.kind();
                            if kind_arg != "(" && kind_arg != ")" && kind_arg != "," {
                              if kind_arg == "keyword_argument" {
                                if let (Some(k), Some(v)) = (
                                  child_arg.child_by_field_name("name"),
                                  child_arg.child_by_field_name("value"),
                                ) {
                                  let k_text = k.utf8_text(source.as_bytes()).unwrap_or("").trim();
                                  if k_text == "response_model" {
                                    response_model = Some(
                                      v.utf8_text(source.as_bytes())
                                        .unwrap_or("")
                                        .trim()
                                        .to_string(),
                                    );
                                  }
                                }
                              } else if is_first {
                                let raw_path =
                                  child_arg.utf8_text(source.as_bytes()).unwrap_or("").trim();
                                path = if (raw_path.starts_with('\'') && raw_path.ends_with('\''))
                                  || (raw_path.starts_with('"') && raw_path.ends_with('"'))
                                {
                                  if raw_path.len() >= 2 {
                                    raw_path[1..raw_path.len() - 1].to_string()
                                  } else {
                                    raw_path.to_string()
                                  }
                                } else {
                                  raw_path.to_string()
                                };
                                is_first = false;
                              }
                            }
                          }
                        }

                        // Apply blueprint url_prefix if applicable
                        if let Some(dot_idx) = decorator_name.find('.') {
                          let var_name = &decorator_name[..dot_idx];
                          if let Some(prefix) = blueprints.get(var_name) {
                            if !prefix.is_empty() {
                              let clean_prefix = prefix.trim_end_matches('/');
                              let clean_path = path.trim_start_matches('/');
                              path = format!("{}/{}", clean_prefix, clean_path);
                            }
                          }
                        }
                        if !path.starts_with('/') {
                          path = format!("/{}", path);
                        }

                        let start = node.start_position();
                        let end = node.end_position();

                        for method in methods {
                          routes.push(ExtractedRoute {
                            handler_name: func_name.clone(),
                            qualified_name: qname.clone(),
                            method,
                            path: path.clone(),
                            response_model: response_model.clone(),
                            start_line: start.row + 1,
                            start_column: start.column,
                            end_line: end.row + 1,
                            end_column: end.column,
                          });
                        }

                        // Extract dependencies for this route handler
                        extract_dependencies_from_params(node, source, &qname, dependencies);
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
    _ => {}
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_routes(
      child,
      source,
      module_name,
      scope_stack,
      routes,
      dependencies,
      blueprints,
    );
  }

  if name_pushed {
    scope_stack.pop();
  }
}

#[allow(clippy::collapsible_if)]
fn extract_dependencies_from_params(
  func_node: Node<'_>,
  source: &str,
  handler_qname: &str,
  dependencies: &mut Vec<(String, String)>,
) {
  if let Some(params_node) = func_node.child_by_field_name("parameters") {
    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
      if child.kind() == "default_parameter" || child.kind() == "typed_default_parameter" {
        if let Some(value_node) = child.child_by_field_name("value") {
          if value_node.kind() == "call" {
            if let Some(func_call_node) = value_node.child_by_field_name("function") {
              let func_call_text = func_call_node
                .utf8_text(source.as_bytes())
                .unwrap_or("")
                .trim();
              if func_call_text == "Depends" || func_call_text.ends_with(".Depends") {
                let mut dep_target = None;
                if let Some(arg_list) = value_node.child_by_field_name("arguments") {
                  let mut arg_cursor = arg_list.walk();
                  if let Some(first_arg) = arg_list
                    .children(&mut arg_cursor)
                    .find(|c| c.kind() != "(" && c.kind() != ")" && c.kind() != ",")
                  {
                    let arg_text = first_arg.utf8_text(source.as_bytes()).unwrap_or("").trim();
                    if !arg_text.is_empty() {
                      dep_target = Some(arg_text.to_string());
                    }
                  }
                }

                if dep_target.is_none() {
                  if let Some(type_node) = child.child_by_field_name("type") {
                    let type_text = type_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
                    if !type_text.is_empty() {
                      dep_target = Some(type_text.to_string());
                    }
                  }
                }

                if let Some(dep) = dep_target {
                  let clean_dep = if (dep.starts_with('\'') && dep.ends_with('\''))
                    || (dep.starts_with('"') && dep.ends_with('"'))
                  {
                    if dep.len() >= 2 {
                      dep[1..dep.len() - 1].to_string()
                    } else {
                      dep
                    }
                  } else {
                    dep
                  };
                  dependencies.push((handler_qname.to_string(), clean_dep));
                }
              }
            }
          }
        }
      }
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedPydanticField {
  pub name: String,
  pub type_annotation: String,
  pub is_required: bool,
  pub default_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedPydanticValidator {
  pub name: String,
  pub validator_type: String, // "field" or "model"
  pub target_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedPydanticModel {
  pub name: String,
  pub qualified_name: String,
  pub fields: Vec<ExtractedPydanticField>,
  pub validators: Vec<ExtractedPydanticValidator>,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}

pub fn extract_pydantic_models_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedPydanticModel>, PythonParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let module_name = module_name_from_path(file_path);
  let mut models = Vec::new();
  let mut scope_stack = Vec::new();

  traverse_pydantic_models(root, source, &module_name, &mut scope_stack, &mut models);

  Ok(models)
}

fn is_pydantic_superclass(node: Node<'_>, source: &str) -> bool {
  if let Some(arg_list) = node.child_by_field_name("superclasses") {
    let mut cursor = arg_list.walk();
    for child in arg_list.children(&mut cursor) {
      let super_text = child.utf8_text(source.as_bytes()).unwrap_or("").trim();
      if super_text == "BaseModel" || super_text.ends_with(".BaseModel") {
        return true;
      }
    }
  }
  false
}

fn parse_pydantic_fields(class_body: Node<'_>, source: &str) -> Vec<ExtractedPydanticField> {
  let mut fields = Vec::new();
  let mut cursor = class_body.walk();

  for stmt in class_body.children(&mut cursor) {
    let mut parse_assignment = |assign_node: Node<'_>| {
      if let Some(left_node) = assign_node.child(0) {
        let field_name = left_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string();

        if !field_name.is_empty() && !field_name.starts_with('_') && field_name != "model_config" {
          let mut type_annotation = "Any".to_string();
          let mut default_value = None;
          let mut has_type = false;

          // Find the type annotation
          let mut inner_cursor = assign_node.walk();
          for child in assign_node.children(&mut inner_cursor) {
            if child.kind() == "type" {
              let t_text = child.utf8_text(source.as_bytes()).unwrap_or("").trim();
              type_annotation = t_text.to_string();
              has_type = true;
            }
          }

          if has_type {
            // Find the default value using the = operator search
            let mut has_equals = false;
            let mut value_node = None;
            let mut val_cursor = assign_node.walk();
            for child in assign_node.children(&mut val_cursor) {
              if child.kind() == "=" {
                has_equals = true;
              } else if has_equals {
                value_node = Some(child);
                break;
              }
            }

            let is_required = if let Some(val_node) = value_node {
              let val_text = val_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
              default_value = Some(val_text.to_string());
              val_text == "..." || val_text.contains("...") || val_text.contains("Ellipsis")
            } else {
              true
            };

            fields.push(ExtractedPydanticField {
              name: field_name,
              type_annotation,
              is_required,
              default_value,
            });
          }
        }
      }
    };

    if stmt.kind() == "assignment" {
      parse_assignment(stmt);
    } else if stmt.kind() == "expression_statement" {
      let mut expr_cursor = stmt.walk();
      for child in stmt.children(&mut expr_cursor) {
        if child.kind() == "assignment" {
          parse_assignment(child);
        }
      }
    }
  }

  fields
}

fn parse_pydantic_validators(
  class_body: Node<'_>,
  source: &str,
) -> Vec<ExtractedPydanticValidator> {
  let mut validators = Vec::new();
  let mut cursor = class_body.walk();

  for stmt in class_body.children(&mut cursor) {
    let mut process_fn = |fn_node: Node<'_>, decorators: Vec<String>, dec_nodes: Vec<Node<'_>>| {
      if let Some(name_node) = fn_node.child_by_field_name("name") {
        let fn_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string();

        if !fn_name.is_empty() {
          for (dec, dec_node) in decorators.into_iter().zip(dec_nodes) {
            let dec_lower = dec.to_lowercase();
            let is_pydantic_val = dec_lower == "validator"
              || dec_lower.ends_with(".validator")
              || dec_lower == "field_validator"
              || dec_lower.ends_with(".field_validator")
              || dec_lower == "model_validator"
              || dec_lower.ends_with(".model_validator")
              || dec_lower == "root_validator"
              || dec_lower.ends_with(".root_validator");

            if is_pydantic_val {
              let validator_type =
                if dec_lower.contains("model_validator") || dec_lower.contains("root_validator") {
                  "model".to_string()
                } else {
                  "field".to_string()
                };

              let mut target_fields = Vec::new();
              // Parse arguments if it is a call
              if let Some(arg_list) = dec_node
                .child_by_field_name("arguments")
                .filter(|_| dec_node.kind() == "call")
              {
                let mut arg_cursor = arg_list.walk();
                for child_arg in arg_list.children(&mut arg_cursor) {
                  if child_arg.kind() == "string" {
                    let raw_text = child_arg.utf8_text(source.as_bytes()).unwrap_or("").trim();
                    if raw_text.len() >= 2 {
                      let clean = raw_text[1..raw_text.len() - 1].trim().to_string();
                      if !clean.is_empty() {
                        target_fields.push(clean);
                      }
                    }
                  }
                }
              }

              validators.push(ExtractedPydanticValidator {
                name: fn_name.clone(),
                validator_type,
                target_fields,
              });
            }
          }
        }
      }
    };

    if stmt.kind() == "function_definition" {
      // undecorated function inside class has no validators
    } else if stmt.kind() == "decorated_definition" {
      let mut dec_list = Vec::new();
      let mut dec_nodes = Vec::new();
      let mut fn_def_opt = None;

      let mut inner_cursor = stmt.walk();
      for child in stmt.children(&mut inner_cursor) {
        if child.kind() == "decorator" {
          let mut expr_cursor = child.walk();
          if let Some(expr_node) = child.children(&mut expr_cursor).find(|c| c.kind() != "@") {
            let dec_name = if expr_node.kind() == "call" {
              if let Some(func_node) = expr_node.child_by_field_name("function") {
                func_node
                  .utf8_text(source.as_bytes())
                  .unwrap_or("")
                  .trim()
                  .to_string()
              } else {
                expr_node
                  .utf8_text(source.as_bytes())
                  .unwrap_or("")
                  .trim()
                  .to_string()
              }
            } else {
              expr_node
                .utf8_text(source.as_bytes())
                .unwrap_or("")
                .trim()
                .to_string()
            };
            dec_list.push(dec_name);
            dec_nodes.push(expr_node);
          }
        } else if child.kind() == "function_definition" {
          fn_def_opt = Some(child);
        }
      }

      if let Some(fn_def) = fn_def_opt {
        process_fn(fn_def, dec_list, dec_nodes);
      }
    }
  }

  validators
}

fn traverse_pydantic_models(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, bool)>,
  models: &mut Vec<ExtractedPydanticModel>,
) {
  let kind = node.kind();
  let mut name_pushed = false;

  if let Some(name_node) = node
    .child_by_field_name("name")
    .filter(|_| kind == "class_definition")
  {
    let class_name = name_node
      .utf8_text(source.as_bytes())
      .unwrap_or("")
      .to_string();
    if !class_name.is_empty() {
      scope_stack.push((class_name.clone(), true));
      name_pushed = true;

      if is_pydantic_superclass(node, source) {
        let qname = current_scope_qname(module_name, scope_stack);
        let start = node.start_position();
        let end = node.end_position();

        let mut fields = Vec::new();
        let mut validators = Vec::new();

        if let Some(body_node) = node.child_by_field_name("body") {
          fields = parse_pydantic_fields(body_node, source);
          validators = parse_pydantic_validators(body_node, source);
        }

        models.push(ExtractedPydanticModel {
          name: class_name,
          qualified_name: qname,
          fields,
          validators,
          start_line: start.row + 1,
          start_column: start.column,
          end_line: end.row + 1,
          end_column: end.column,
        });
      }
    }
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_pydantic_models(child, source, module_name, scope_stack, models);
  }

  if name_pushed {
    scope_stack.pop();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_valid_python_without_errors() {
    let report =
      parse_python_source("def validate_patient(patient):\n  return bool(patient.get('id'))\n")
        .expect("valid python should parse");

    assert_eq!(report.root_kind, "module");
    assert!(report.named_node_count > 0);
    assert!(report.syntax_errors.is_empty());
  }

  #[test]
  fn reports_syntax_errors_for_invalid_python() {
    let report = parse_python_source("def broken(:\n  return 1\n")
      .expect("invalid python should still produce a tree report");

    assert!(!report.syntax_errors.is_empty());
    let first = &report.syntax_errors[0];
    assert!(first.message.starts_with("error") || first.message.starts_with("missing"));
    assert!(first.start_line >= 1);
    assert!(first.start_column >= 1);
  }

  #[test]
  fn traversal_is_deterministic_for_same_source() {
    let source = "class Patient:\n  def validate(self):\n    return True\n";

    let a = parse_python_source(source).expect("parse should succeed");
    let b = parse_python_source(source).expect("parse should succeed");

    assert_eq!(a.named_node_count, b.named_node_count);
    assert_eq!(a.syntax_errors, b.syntax_errors);
  }

  #[test]
  fn does_not_duplicate_named_nodes_or_errors() {
    let report = parse_python_source("def broken(:\n  return 1\n")
      .expect("invalid python should still produce a tree report");

    assert!(report.named_node_count > 0);
    let unique_errors: std::collections::HashSet<_> = report
      .syntax_errors
      .iter()
      .map(|e| {
        (
          e.message.as_str(),
          e.start_line,
          e.start_column,
          e.end_line,
          e.end_column,
        )
      })
      .collect();
    assert_eq!(unique_errors.len(), report.syntax_errors.len());
  }

  #[test]
  fn extracts_symbols_correctly_from_python_source() {
    let source = r#"
class Patient:
  def __init__(self, patient_id):
    self.id = patient_id

  async def validate_patient(self):
    def inner_helper():
      return True
    return inner_helper()

def global_fn():
  pass
"#;

    let symbols =
      extract_symbols_from_source(source, "src/patient.py").expect("extraction should succeed");

    let qnames: Vec<&str> = symbols.iter().map(|s| s.qualified_name.as_str()).collect();
    assert_eq!(
      qnames,
      vec![
        "src.patient",
        "src.patient::Patient",
        "src.patient::Patient.__init__",
        "src.patient::Patient.validate_patient",
        "src.patient::Patient.validate_patient.inner_helper",
        "src.patient::global_fn",
      ]
    );

    assert_eq!(symbols[0].kind, SymbolKind::Module);
    assert_eq!(symbols[1].kind, SymbolKind::Class);
    assert_eq!(symbols[2].kind, SymbolKind::Method);
    assert_eq!(symbols[3].kind, SymbolKind::Method);
    assert_eq!(symbols[4].kind, SymbolKind::Function);
    assert_eq!(symbols[5].kind, SymbolKind::Function);

    assert_eq!(symbols[0].location.start_line, 1);
    assert!(symbols[0].location.end_line >= 12);
  }

  #[test]
  fn extracts_imports_correctly_from_python_source() {
    let source = r#"
import os, sys
import numpy as np
from datetime import datetime as dt, timezone
from os import *
from . import sibling
from ..parent.child import helper
"#;

    let imports =
      extract_imports_from_source(source, "src/pkg/module.py").expect("extraction should succeed");

    let extracted: Vec<(&str, usize)> = imports
      .iter()
      .map(|imp| (imp.target_qualified_name.as_str(), imp.start_line))
      .collect();

    assert_eq!(
      extracted,
      vec![
        ("os", 2),
        ("sys", 2),
        ("numpy", 3),
        ("datetime.datetime", 4),
        ("datetime.timezone", 4),
        ("os", 5),
        ("src.pkg.sibling", 6),
        ("src.parent.child.helper", 7),
      ]
    );
  }

  #[test]
  fn extracts_calls_correctly_from_python_source() {
    let source = r#"
class Patient:
  def __init__(self, patient_id):
    self.id = patient_id
    self.setup_logging()

  def validate(self):
    return self.check_fields()

def helper_fn():
  return global_val()

helper_fn()
"#;

    let calls =
      extract_calls_from_source(source, "src/patient.py").expect("extraction should succeed");

    assert_eq!(calls.len(), 4);

    // Call 1: self.setup_logging() in Patient.__init__
    assert_eq!(
      calls[0].source_symbol_qualified_name,
      "src.patient::Patient.__init__"
    );
    assert_eq!(calls[0].target_name, "self.setup_logging");
    assert!(calls[0].is_self_call);
    assert_eq!(calls[0].method_name, Some("setup_logging".to_string()));

    // Call 2: self.check_fields() in Patient.validate
    assert_eq!(
      calls[1].source_symbol_qualified_name,
      "src.patient::Patient.validate"
    );
    assert_eq!(calls[1].target_name, "self.check_fields");
    assert!(calls[1].is_self_call);
    assert_eq!(calls[1].method_name, Some("check_fields".to_string()));

    // Call 3: global_val() in helper_fn
    assert_eq!(
      calls[2].source_symbol_qualified_name,
      "src.patient::helper_fn"
    );
    assert_eq!(calls[2].target_name, "global_val");
    assert!(!calls[2].is_self_call);
    assert_eq!(calls[2].method_name, None);

    // Call 4: helper_fn() at module-level
    assert_eq!(calls[3].source_symbol_qualified_name, "src.patient");
    assert_eq!(calls[3].target_name, "helper_fn");
    assert!(!calls[3].is_self_call);
    assert_eq!(calls[3].method_name, None);
  }

  #[test]
  fn extracts_type_references_correctly_from_python_source() {
    let source = r#"
class Patient(BaseModel, dict):
  def validate(self, age: int, record: "MedicalRecord") -> "bool":
    x: "Diagnosis" = Diagnosis()
    y: list[Patient] = []
    z: typing.Optional[Patient] = None
"#;

    let refs = extract_type_references_from_source(source, "src/patient.py")
      .expect("extraction should succeed");

    let extracted: Vec<(&str, &str, usize)> = refs
      .iter()
      .map(|r| {
        (
          r.source_symbol_qualified_name.as_str(),
          r.target_type_name.as_str(),
          r.start_line,
        )
      })
      .collect();

    assert_eq!(
      extracted,
      vec![
        ("src.patient::Patient", "BaseModel", 2),
        ("src.patient::Patient.validate", "MedicalRecord", 3),
        ("src.patient::Patient.validate", "Diagnosis", 4),
        ("src.patient::Patient.validate", "Patient", 5),
        ("src.patient::Patient.validate", "Patient", 6),
      ]
    );
  }

  #[test]
  fn extracts_decorators_correctly_from_python_source() {
    let source = r#"
@singleton
class PatientService:
  @route("/patient")
  def validate_patient(self):
    pass

  @pytest.fixture(scope="module")
  @custom_decorator
  def helper(self):
    pass
"#;

    let decorators = extract_decorators_from_source(source, "src/service.py")
      .expect("decorator extraction should succeed");
    let extracted: Vec<(&str, &str, usize)> = decorators
      .iter()
      .map(|d| {
        (
          d.source_symbol_qualified_name.as_str(),
          d.decorator_name.as_str(),
          d.start_line,
        )
      })
      .collect();

    assert_eq!(
      extracted,
      vec![
        ("src.service::PatientService", "singleton", 2),
        ("src.service::PatientService.validate_patient", "route", 4),
        ("src.service::PatientService.helper", "pytest.fixture", 8),
        ("src.service::PatientService.helper", "custom_decorator", 9),
      ]
    );
  }

  #[test]
  fn extracts_tests_correctly_from_python_source() {
    let source = r#"
@pytest.fixture
def db_connection():
  pass

class TestDatabase:
  @pytest.mark.parametrize("query,expected", [("A", 1)])
  def test_query(self, db_connection, query, expected):
    pass

  def helper_method(self):
    pass

def test_standalone():
  pass
"#;

    let tests = extract_tests_from_source(source, "tests/test_db.py")
      .expect("test extraction should succeed");

    assert_eq!(tests.len(), 4);

    assert_eq!(tests[0].name, "db_connection");
    assert_eq!(tests[0].qualified_name, "tests.test_db::db_connection");
    assert_eq!(tests[0].kind, ExtractedTestKind::Fixture);
    assert!(!tests[0].is_parametrized);
    assert_eq!(tests[0].parameters, Vec::<String>::new());

    assert_eq!(tests[1].name, "TestDatabase");
    assert_eq!(tests[1].qualified_name, "tests.test_db::TestDatabase");
    assert_eq!(tests[1].kind, ExtractedTestKind::Class);
    assert_eq!(tests[1].parameters, Vec::<String>::new());

    assert_eq!(tests[2].name, "test_query");
    assert_eq!(
      tests[2].qualified_name,
      "tests.test_db::TestDatabase.test_query"
    );
    assert_eq!(tests[2].kind, ExtractedTestKind::Function);
    assert!(tests[2].is_parametrized);
    assert_eq!(tests[2].parameters, vec!["db_connection".to_string()]);

    assert_eq!(tests[3].name, "test_standalone");
    assert_eq!(tests[3].qualified_name, "tests.test_db::test_standalone");
    assert_eq!(tests[3].kind, ExtractedTestKind::Function);
    assert!(!tests[3].is_parametrized);
    assert_eq!(tests[3].parameters, Vec::<String>::new());
  }

  #[test]
  fn extracts_docstrings_correctly_from_python_source() {
    let source = r#"
"""
Module docstring
"""

class Patient:
    """Class docstring"""
    def __init__(self):
        '''Method docstring'''
        pass

def helper():
    "Single quoted docstring"
    pass
"#;

    let docs =
      extract_docstrings_from_source(source, "src/patient.py").expect("extraction should succeed");

    assert_eq!(docs.len(), 4);

    assert_eq!(docs[0].symbol_qualified_name, "src.patient");
    assert_eq!(docs[0].text, "Module docstring");

    assert_eq!(docs[1].symbol_qualified_name, "src.patient::Patient");
    assert_eq!(docs[1].text, "Class docstring");

    assert_eq!(
      docs[2].symbol_qualified_name,
      "src.patient::Patient.__init__"
    );
    assert_eq!(docs[2].text, "Method docstring");

    assert_eq!(docs[3].symbol_qualified_name, "src.patient::helper");
    assert_eq!(docs[3].text, "Single quoted docstring");
  }

  #[test]
  fn extracts_routes_and_dependencies_correctly() {
    let source = r#"
@app.get("/items/{item_id}", response_model=ItemResponse)
def read_item(item_id: int, db: Session = Depends(get_db)):
  pass

@router.post("/items/")
def create_item(db: Session = Depends(get_db), current_user: User = Depends()):
  pass

@app.route("/legacy", methods=["GET", "POST"])
def legacy_endpoint():
  pass
"#;

    let (routes, deps) = extract_routes_and_dependencies_from_source(source, "src/api.py")
      .expect("extraction must succeed");

    assert_eq!(routes.len(), 4);

    // Route 1
    assert_eq!(routes[0].handler_name, "read_item");
    assert_eq!(routes[0].qualified_name, "src.api::read_item");
    assert_eq!(routes[0].method, "GET");
    assert_eq!(routes[0].path, "/items/{item_id}");
    assert_eq!(routes[0].response_model, Some("ItemResponse".to_string()));

    // Route 2
    assert_eq!(routes[1].handler_name, "create_item");
    assert_eq!(routes[1].qualified_name, "src.api::create_item");
    assert_eq!(routes[1].method, "POST");
    assert_eq!(routes[1].path, "/items/");
    assert_eq!(routes[1].response_model, None);

    // Route 3 (Legacy app.route with methods - GET)
    assert_eq!(routes[2].handler_name, "legacy_endpoint");
    assert_eq!(routes[2].qualified_name, "src.api::legacy_endpoint");
    assert_eq!(routes[2].method, "GET");
    assert_eq!(routes[2].path, "/legacy");

    // Route 4 (Legacy app.route with methods - POST)
    assert_eq!(routes[3].handler_name, "legacy_endpoint");
    assert_eq!(routes[3].qualified_name, "src.api::legacy_endpoint");
    assert_eq!(routes[3].method, "POST");
    assert_eq!(routes[3].path, "/legacy");

    // Dependencies
    assert_eq!(deps.len(), 3);
    assert_eq!(
      deps[0],
      ("src.api::read_item".to_string(), "get_db".to_string())
    );
    assert_eq!(
      deps[1],
      ("src.api::create_item".to_string(), "get_db".to_string())
    );
    assert_eq!(
      deps[2],
      ("src.api::create_item".to_string(), "User".to_string())
    );
  }

  #[test]
  fn extracts_flask_routes_with_blueprints_correctly() {
    let source = r#"
from flask import Blueprint, Flask

app = Flask(__name__)
auth_bp = Blueprint('auth', __name__, url_prefix='/auth')
admin_bp = Blueprint('admin', __name__, url_prefix='/admin/')
no_prefix_bp = Blueprint('no_prefix', __name__)

@app.route('/home', methods=['GET'])
def home():
  pass

@auth_bp.route('/login', methods=['GET', 'POST'])
def login():
  pass

@auth_bp.get('/logout')
def logout():
  pass

@admin_bp.route('/dashboard')
def dashboard():
  pass

@no_prefix_bp.route('/info')
def info():
  pass
"#;

    let (routes, _deps) = extract_routes_and_dependencies_from_source(source, "src/api.py")
      .expect("extraction must succeed");

    assert_eq!(routes.len(), 6);

    // Route 1: home (no blueprint)
    assert_eq!(routes[0].handler_name, "home");
    assert_eq!(routes[0].qualified_name, "src.api::home");
    assert_eq!(routes[0].method, "GET");
    assert_eq!(routes[0].path, "/home");

    // Route 2: login (GET)
    assert_eq!(routes[1].handler_name, "login");
    assert_eq!(routes[1].qualified_name, "src.api::login");
    assert_eq!(routes[1].method, "GET");
    assert_eq!(routes[1].path, "/auth/login");

    // Route 3: login (POST)
    assert_eq!(routes[2].handler_name, "login");
    assert_eq!(routes[2].qualified_name, "src.api::login");
    assert_eq!(routes[2].method, "POST");
    assert_eq!(routes[2].path, "/auth/login");

    // Route 4: logout
    assert_eq!(routes[3].handler_name, "logout");
    assert_eq!(routes[3].qualified_name, "src.api::logout");
    assert_eq!(routes[3].method, "GET");
    assert_eq!(routes[3].path, "/auth/logout");

    // Route 5: dashboard (prefix /admin/ + path /dashboard should normalize correctly to /admin/dashboard)
    assert_eq!(routes[4].handler_name, "dashboard");
    assert_eq!(routes[4].qualified_name, "src.api::dashboard");
    assert_eq!(routes[4].method, "GET");
    assert_eq!(routes[4].path, "/admin/dashboard");

    // Route 6: info (no prefix blueprint)
    assert_eq!(routes[5].handler_name, "info");
    assert_eq!(routes[5].qualified_name, "src.api::info");
    assert_eq!(routes[5].method, "GET");
    assert_eq!(routes[5].path, "/info");
  }

  #[test]
  fn extracts_flask_routes_with_registered_blueprints_correctly() {
    let source = r#"
from flask import Blueprint, Flask

app = Flask(__name__)
auth_bp = Blueprint('auth', __name__)
admin_bp = Blueprint('admin', __name__)

app.register_blueprint(auth_bp, url_prefix='/auth')
app.register_blueprint(blueprint=admin_bp, url_prefix='/admin/')

@auth_bp.route('/login', methods=['GET'])
def login():
  pass

@admin_bp.route('/dashboard')
def dashboard():
  pass
"#;

    let (routes, _deps) = extract_routes_and_dependencies_from_source(source, "src/api.py")
      .expect("extraction must succeed");

    assert_eq!(routes.len(), 2);

    // Route 1: login (prefix /auth from register_blueprint)
    assert_eq!(routes[0].handler_name, "login");
    assert_eq!(routes[0].qualified_name, "src.api::login");
    assert_eq!(routes[0].method, "GET");
    assert_eq!(routes[0].path, "/auth/login");

    // Route 2: dashboard (prefix /admin/ from register_blueprint with keyword arg)
    assert_eq!(routes[1].handler_name, "dashboard");
    assert_eq!(routes[1].qualified_name, "src.api::dashboard");
    assert_eq!(routes[1].method, "GET");
    assert_eq!(routes[1].path, "/admin/dashboard");
  }

  #[test]
  fn extracts_pydantic_models_correctly_from_python_source() {
    let source = r#"
from pydantic import BaseModel, Field, field_validator, model_validator

class Patient(BaseModel):
    id: int
    name: str = Field(..., max_length=50)
    age: int = 30
    address: Address

    @field_validator("name")
    def name_must_contain_space(cls, v):
        return v

    @model_validator(mode="before")
    def check_all(cls, data):
        return data
"#;

    let models = extract_pydantic_models_from_source(source, "src/models.py")
      .expect("extraction should succeed");

    assert_eq!(models.len(), 1);
    let m = &models[0];
    assert_eq!(m.name, "Patient");
    assert_eq!(m.qualified_name, "src.models::Patient");

    // Fields
    assert_eq!(m.fields.len(), 4);
    assert_eq!(m.fields[0].name, "id");
    assert_eq!(m.fields[0].type_annotation, "int");
    assert!(m.fields[0].is_required);

    assert_eq!(m.fields[1].name, "name");
    assert_eq!(m.fields[1].type_annotation, "str");
    assert!(m.fields[1].is_required);

    assert_eq!(m.fields[2].name, "age");
    assert_eq!(m.fields[2].type_annotation, "int");
    assert!(!m.fields[2].is_required);
    assert_eq!(m.fields[2].default_value.as_deref(), Some("30"));

    assert_eq!(m.fields[3].name, "address");
    assert_eq!(m.fields[3].type_annotation, "Address");
    assert!(m.fields[3].is_required);

    // Validators
    assert_eq!(m.validators.len(), 2);
    assert_eq!(m.validators[0].name, "name_must_contain_space");
    assert_eq!(m.validators[0].validator_type, "field");
    assert_eq!(m.validators[0].target_fields, vec!["name".to_string()]);

    assert_eq!(m.validators[1].name, "check_all");
    assert_eq!(m.validators[1].validator_type, "model");
    assert!(m.validators[1].target_fields.is_empty());
  }

  #[test]
  fn extracts_pytorch_submodules_correctly() {
    let source = r#"
import torch.nn as nn

class MyModel(nn.Module):
    def __init__(self):
        super(MyModel, self).__init__()
        self.conv1 = nn.Conv2d(1, 32, 3, 1)
        self.conv2 = nn.Conv2d(32, 64, 3, 1)
        self.fc = nn.Linear(9216, 128)

    def forward(self, x):
        x = self.conv1(x)
        x = self.conv2(x)
        x = self.fc(x)
        return x
"#;

    let symbols = extract_symbols_from_source(source, "src/models.py")
      .expect("symbols extraction should succeed");

    let submodules: Vec<_> = symbols
      .iter()
      .filter(|s| s.kind == SymbolKind::Unknown)
      .collect();

    assert_eq!(submodules.len(), 3);
    assert_eq!(submodules[0].name, "conv1");
    assert_eq!(submodules[0].qualified_name, "src.models::MyModel.conv1");
    assert_eq!(submodules[1].name, "conv2");
    assert_eq!(submodules[1].qualified_name, "src.models::MyModel.conv2");
    assert_eq!(submodules[2].name, "fc");
    assert_eq!(submodules[2].qualified_name, "src.models::MyModel.fc");

    let type_refs = extract_type_references_from_source(source, "src/models.py")
      .expect("type reference extraction should succeed");

    let pytorch_refs: Vec<_> = type_refs
      .iter()
      .filter(|tr| {
        tr.source_symbol_qualified_name
          .starts_with("src.models::MyModel.")
      })
      .collect();

    assert_eq!(pytorch_refs.len(), 3);
    assert_eq!(
      pytorch_refs[0].source_symbol_qualified_name,
      "src.models::MyModel.conv1"
    );
    assert_eq!(pytorch_refs[0].target_type_name, "nn.Conv2d");
    assert_eq!(
      pytorch_refs[1].source_symbol_qualified_name,
      "src.models::MyModel.conv2"
    );
    assert_eq!(pytorch_refs[1].target_type_name, "nn.Conv2d");
    assert_eq!(
      pytorch_refs[2].source_symbol_qualified_name,
      "src.models::MyModel.fc"
    );
    assert_eq!(pytorch_refs[2].target_type_name, "nn.Linear");
  }

  #[test]
  fn extracts_dataframe_column_references_correctly() {
    let source = r#"
import polars as pl

def process_data(df):
    result = df.filter(pl.col("age") > 30, pl.col("country") == "US").select(
        pl.col("name"),
        "city",
        'salary'
      )
    return result
"#;

    let type_refs = extract_type_references_from_source(source, "src/pipeline.py")
      .expect("type reference extraction should succeed");

    let col_refs: Vec<_> = type_refs
      .iter()
      .filter(|tr| tr.target_type_name.starts_with("col::"))
      .collect();

    assert_eq!(col_refs.len(), 5);
    assert_eq!(
      col_refs[0].source_symbol_qualified_name,
      "src.pipeline::process_data"
    );
    assert_eq!(col_refs[0].target_type_name, "col::name");
    assert_eq!(
      col_refs[1].source_symbol_qualified_name,
      "src.pipeline::process_data"
    );
    assert_eq!(col_refs[1].target_type_name, "col::city");
    assert_eq!(
      col_refs[2].source_symbol_qualified_name,
      "src.pipeline::process_data"
    );
    assert_eq!(col_refs[2].target_type_name, "col::salary");
    assert_eq!(
      col_refs[3].source_symbol_qualified_name,
      "src.pipeline::process_data"
    );
    assert_eq!(col_refs[3].target_type_name, "col::age");
    assert_eq!(
      col_refs[4].source_symbol_qualified_name,
      "src.pipeline::process_data"
    );
    assert_eq!(col_refs[4].target_type_name, "col::country");

    // Ensure that string literal constants like "US" are NOT extracted as columns
    let has_us_col = col_refs.iter().any(|tr| tr.target_type_name == "col::US");
    assert!(!has_us_col);
  }

  #[test]
  fn extracts_rop_and_monads_correctly() {
    let source = r#"
def step1(x):
  return x + 1

def step2(x):
  return x * 2

def pipeline(val):
  # 1. Monadic chain
  res = val.bind(step1).then(step2)
  # 2. Operator chaining
  flow = step1 >> step2 >>= step1
  # 3. Pipe function
  out = pipe(val, step1, step2)
  # 4. Higher-order functions
  mapped = map(step1, [1, 2, 3])
  filtered = filter(step2, [1, 2, 3])
  # 5. Partial application
  part = partial(step1, 5)
  part_long = functools.partial(step2, 10)
  # 6. Compose function
  comp = compose(step1, step2)
  return out
"#;

    let calls =
      extract_calls_from_source(source, "src/rop.py").expect("calls extraction should succeed");

    let pipeline_calls: Vec<_> = calls
      .iter()
      .filter(|c| c.source_symbol_qualified_name == "src.rop::pipeline")
      .collect();

    // Verify monadic bind/then targets
    assert!(pipeline_calls.iter().any(|c| c.target_name == "step1"));
    assert!(pipeline_calls.iter().any(|c| c.target_name == "step2"));

    // Verify >> and >>= operator targets
    assert!(pipeline_calls.iter().any(|c| c.target_name == "step1"));
    assert!(pipeline_calls.iter().any(|c| c.target_name == "step2"));

    // Verify pipe calls and that the first argument 'val' was skipped as an input value rather than a callable stage
    let pipe_calls: Vec<_> = pipeline_calls
      .iter()
      .filter(|c| c.target_name == "pipe")
      .collect();
    assert_eq!(pipe_calls.len(), 1);

    let has_val_as_call = pipeline_calls.iter().any(|c| c.target_name == "val");
    assert!(!has_val_as_call); // Should be skipped!

    // Verify compose captures ALL stages (step1 and step2)
    let compose_calls: Vec<_> = pipeline_calls
      .iter()
      .filter(|c| c.target_name == "compose")
      .collect();
    assert_eq!(compose_calls.len(), 1);

    // Both step1 and step2 should be extracted as calls
    assert!(pipeline_calls.iter().any(|c| c.target_name == "step1"));
    assert!(pipeline_calls.iter().any(|c| c.target_name == "step2"));

    // Verify map and filter target functions
    assert!(pipeline_calls.iter().any(|c| c.target_name == "map"));
    assert!(pipeline_calls.iter().any(|c| c.target_name == "filter"));

    // Verify partial applications are captured
    assert!(pipeline_calls.iter().any(|c| c.target_name == "partial"));
    assert!(
      pipeline_calls
        .iter()
        .any(|c| c.target_name == "functools.partial")
    );
  }

  #[test]
  fn extracts_currying_and_library_placeholders_correctly() {
    let source = r#"
from toolz import curry
from pymonad.tools import curry as pm_curry
from returns.pipeline import flow

@curry
def add(x, y, z):
  return x + y + z

@pm_curry
def multiply(a, b):
  return a * b

@curry
def step1(x):
  return x + 1

def step2(x, y):
  return x + y

def pipeline(val):
  # 1. toolz.curry partial application
  f1 = add(5) # Expect placeholders: y, z
  f2 = add(5, 10) # Expect placeholders: z
  f3 = add(5, y=10) # Expect placeholders: z

  # 2. functools.partial application
  f4 = partial(step2, 10) # Expect placeholders: y

  # 3. Monadic chain / flow stages with placeholders
  res = flow(val, add(1, 2), step1, multiply(10))
  # For add(1, 2): N=3, M=2 + 1 (flow) = 3. Expect placeholders: None (or fully applied)
  # For step1: N=1, M=0 + 1 (flow) = 1. Expect placeholders: None
  # For multiply(10): N=2, M=1 + 1 (flow) = 2. Expect placeholders: None

  return res
"#;

    let calls = extract_calls_from_source(source, "src/pipeline_depth.py")
      .expect("calls extraction should succeed");

    let pipeline_calls: Vec<_> = calls
      .iter()
      .filter(|c| c.source_symbol_qualified_name == "src.pipeline_depth::pipeline")
      .collect();

    // Let's assert ordering is assigned
    assert!(pipeline_calls.iter().all(|c| c.ordering.is_some()));

    // Find call to "add" (f1 = add(5))
    let call_add_f1 = pipeline_calls
      .iter()
      .find(|c| c.target_name == "add" && c.start_line == 23)
      .expect("should find first add call");
    assert_eq!(call_add_f1.placeholders.as_deref(), Some("y, z"));

    // Find call to "add" (f2 = add(5, 10))
    let call_add_f2 = pipeline_calls
      .iter()
      .find(|c| c.target_name == "add" && c.start_line == 24)
      .expect("should find second add call");
    assert_eq!(call_add_f2.placeholders.as_deref(), Some("z"));

    // Find call to "step2" inside partial (f4 = partial(step2, 10))
    let call_step2 = pipeline_calls
      .iter()
      .find(|c| c.target_name == "step2")
      .expect("should find step2 call");
    assert_eq!(call_step2.placeholders.as_deref(), Some("y"));
  }

  #[test]
  fn extracts_tensor_shapes_and_dataframe_lineage_correctly() {
    let source = r#"
def train(x: torch.Tensor[batch, 3, 224, 224]):
    # shape: (batch, channels, height, width)
    y = conv(x)
    
    batch, channels, height, width = y.shape
    
    df = df.with_columns(total_revenue = pl.col("revenue") * 1.1)
    df = df.select(pl.col("revenue").alias("rev"))
    df = df.rename({"revenue": "rev"})
    df["rev"] = df["revenue"]
"#;

    let type_refs = extract_type_references_from_source(source, "src/model.py")
      .expect("type reference extraction should succeed");

    // Let's assert shape hints are extracted

    let shape_refs: Vec<_> = type_refs
      .iter()
      .filter(|tr| tr.target_type_name.starts_with("shape::"))
      .collect();

    assert!(
      shape_refs
        .iter()
        .any(|tr| tr.target_type_name == "shape::x::[batch, 3, 224, 224]")
    );
    assert!(
      shape_refs
        .iter()
        .any(|tr| tr.target_type_name == "shape::y::[batch, channels, height, width]")
    );

    // Let's assert dataframe lineage is extracted
    let lineage_refs: Vec<_> = type_refs
      .iter()
      .filter(|tr| tr.target_type_name.contains(" <- "))
      .collect();

    assert!(
      lineage_refs
        .iter()
        .any(|tr| tr.target_type_name == "col::total_revenue <- with_columns(col::revenue)")
    );
    assert!(
      lineage_refs
        .iter()
        .any(|tr| tr.target_type_name == "col::rev <- select(col::revenue)")
    );
    assert!(
      lineage_refs
        .iter()
        .any(|tr| tr.target_type_name == "col::rev <- rename(col::revenue)")
    );
    assert!(
      lineage_refs
        .iter()
        .any(|tr| tr.target_type_name == "col::rev <- assign(col::revenue)")
    );
  }
}
