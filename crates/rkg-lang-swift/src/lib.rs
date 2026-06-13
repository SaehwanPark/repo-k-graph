#![allow(
  clippy::collapsible_if,
  clippy::too_many_arguments,
  clippy::type_complexity,
  clippy::only_used_in_recursion
)]

use rkg_core::{Location, Symbol, SymbolKind};
use std::fmt::{Display, Formatter};
use std::path::Path;
use tree_sitter::{Node, Parser};

pub const CRATE_NAME: &str = "rkg-lang-swift";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwiftParseError {
  UnsupportedLanguage(String),
  ParseCancelled,
}

impl Display for SwiftParseError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      SwiftParseError::UnsupportedLanguage(message) => {
        write!(f, "failed to initialize swift parser: {message}")
      }
      SwiftParseError::ParseCancelled => write!(f, "swift parsing was cancelled"),
    }
  }
}

impl std::error::Error for SwiftParseError {}

fn find_name_child(node: Node<'_>) -> Option<Node<'_>> {
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    let kind = child.kind();
    if matches!(kind, "type_identifier" | "simple_identifier" | "identifier") {
      return Some(child);
    }
    if kind == "user_type" {
      let mut sub_cursor = child.walk();
      for sub_child in child.children(&mut sub_cursor) {
        let sub_kind = sub_child.kind();
        if matches!(
          sub_kind,
          "type_identifier" | "simple_identifier" | "identifier"
        ) {
          return Some(sub_child);
        }
      }
    }
  }
  None
}

fn find_identifier_recursive(node: Node<'_>, source: &str) -> Option<String> {
  let kind = node.kind();
  if matches!(kind, "simple_identifier" | "identifier" | "type_identifier") {
    let text = node
      .utf8_text(source.as_bytes())
      .unwrap_or("")
      .trim()
      .to_string();
    if !text.is_empty() {
      return Some(text);
    }
  }
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if let Some(name) = find_identifier_recursive(child, source) {
      return Some(name);
    }
  }
  None
}

fn extract_type_name(node: Node<'_>, source: &str) -> Option<String> {
  let kind = node.kind();
  if matches!(kind, "simple_identifier" | "identifier" | "type_identifier") {
    let text = node
      .utf8_text(source.as_bytes())
      .unwrap_or("")
      .trim()
      .to_string();
    if !text.is_empty() {
      return Some(text);
    }
  }
  if kind == "user_type" {
    let mut parts = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
      if child.kind() == "generic_argument_clause" {
        continue;
      }
      if let Some(part) = extract_type_name(child, source) {
        parts.push(part);
      }
    }
    if !parts.is_empty() {
      return Some(parts.join("."));
    }
  }
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if let Some(name) = extract_type_name(child, source) {
      return Some(name);
    }
  }
  None
}

fn find_property_names(node: Node<'_>, source: &str) -> Vec<String> {
  let mut names = Vec::new();
  fn recurse(n: Node<'_>, source: &str, names: &mut Vec<String>) {
    let kind = n.kind();
    if kind == "type_annotation" || kind == "initializer" {
      return; // Avoid picking up type names or default values
    }
    if kind == "pattern" {
      if let Some(name) = find_identifier_recursive(n, source) {
        names.push(name);
      }
      return;
    }
    if matches!(kind, "simple_identifier" | "identifier") {
      let text = n
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .trim()
        .to_string();
      if !text.is_empty() {
        names.push(text);
      }
      return;
    }
    let mut cursor = n.walk();
    for child in n.children(&mut cursor) {
      recurse(child, source, names);
    }
  }
  recurse(node, source, &mut names);
  names
}

fn build_scope_path(scope_stack: &[(String, SymbolKind)]) -> String {
  let mut parts = Vec::new();
  for (name, _) in scope_stack {
    parts.push(name.as_str());
  }
  parts.join(".")
}

/// Variant of `build_scope_path` for the 3-tuple scope stack used in `traverse_tests`.
fn build_test_scope_path(scope_stack: &[(String, SymbolKind, bool)]) -> String {
  scope_stack
    .iter()
    .map(|(name, _, _)| name.as_str())
    .collect::<Vec<_>>()
    .join(".")
}

fn module_name_from_path(file_path: &str) -> String {
  let path = Path::new(file_path);
  let mut components = Vec::new();
  for component in path.components() {
    if let std::path::Component::Normal(comp) = component {
      components.push(comp.to_string_lossy().to_string());
    }
  }

  let normalized_path = components.join("/");
  let path_without_ext = normalized_path
    .strip_suffix(".swift")
    .unwrap_or(&normalized_path);

  path_without_ext.replace(['/', '\\'], ".")
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

  if kind == "function_body" {
    return;
  }

  let mut scope_pushed = false;

  match kind {
    "class_declaration"
    | "struct_declaration"
    | "enum_declaration"
    | "protocol_declaration"
    | "extension_declaration"
    | "actor_declaration" => {
      if let Some(name_node) = find_name_child(node) {
        let text = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string();
        if !text.is_empty() {
          let is_extension = kind == "extension_declaration" || {
            let mut cursor = node.walk();
            node.children(&mut cursor).any(|c| c.kind() == "extension")
          };

          if is_extension {
            scope_stack.push((text, SymbolKind::Class));
            scope_pushed = true;
          } else {
            let is_struct = kind == "struct_declaration" || {
              let mut cursor = node.walk();
              node.children(&mut cursor).any(|c| c.kind() == "struct")
            };
            let is_enum = kind == "enum_declaration" || {
              let mut cursor = node.walk();
              node.children(&mut cursor).any(|c| c.kind() == "enum")
            };
            let is_interface = kind == "protocol_declaration" || {
              let mut cursor = node.walk();
              node.children(&mut cursor).any(|c| c.kind() == "protocol")
            };

            let symbol_kind = if is_struct {
              SymbolKind::Struct
            } else if is_enum {
              SymbolKind::Enum
            } else if is_interface {
              SymbolKind::Interface
            } else {
              SymbolKind::Class
            };

            let qname = if scope_stack.is_empty() {
              format!("{}::{}", module_name, text)
            } else {
              format!(
                "{}::{}.{}",
                module_name,
                build_scope_path(scope_stack),
                text
              )
            };

            let start = node.start_position();
            let end = node.end_position();

            symbols.push(Symbol {
              name: text.clone(),
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

            scope_stack.push((text, symbol_kind));
            scope_pushed = true;
          }
        }
      }
    }
    "function_declaration" | "protocol_function_declaration" => {
      if let Some(name_node) = find_name_child(node) {
        let func_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string();
        if !func_name.is_empty() {
          let is_method = scope_stack
            .last()
            .map(|(_, k)| {
              *k == SymbolKind::Class
                || *k == SymbolKind::Struct
                || *k == SymbolKind::Enum
                || *k == SymbolKind::Interface
            })
            .unwrap_or(false);

          let symbol_kind = if is_method {
            SymbolKind::Method
          } else {
            SymbolKind::Function
          };

          let qname = if scope_stack.is_empty() {
            format!("{}::{}", module_name, func_name)
          } else {
            format!(
              "{}::{}.{}",
              module_name,
              build_scope_path(scope_stack),
              func_name
            )
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
          scope_pushed = true;
        }
      }
    }
    "init_declaration"
    | "initializer_declaration"
    | "deinit_declaration"
    | "deinitializer_declaration" => {
      let func_name = if kind.starts_with("init") {
        "init".to_string()
      } else {
        "deinit".to_string()
      };

      let symbol_kind = SymbolKind::Method;

      let qname = if scope_stack.is_empty() {
        format!("{}::{}", module_name, func_name)
      } else {
        format!(
          "{}::{}.{}",
          module_name,
          build_scope_path(scope_stack),
          func_name
        )
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
      scope_pushed = true;
    }
    "property_declaration" => {
      for prop_name in find_property_names(node, source) {
        let qname = if scope_stack.is_empty() {
          format!("{}::{}", module_name, prop_name)
        } else {
          format!(
            "{}::{}.{}",
            module_name,
            build_scope_path(scope_stack),
            prop_name
          )
        };

        let start = node.start_position();
        let end = node.end_position();

        symbols.push(Symbol {
          name: prop_name,
          qualified_name: qname,
          kind: SymbolKind::Unknown,
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
    "typealias_declaration" => {
      if let Some(name_node) = find_name_child(node) {
        let alias_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string();
        if !alias_name.is_empty() {
          let qname = if scope_stack.is_empty() {
            format!("{}::{}", module_name, alias_name)
          } else {
            format!(
              "{}::{}.{}",
              module_name,
              build_scope_path(scope_stack),
              alias_name
            )
          };

          let start = node.start_position();
          let end = node.end_position();

          symbols.push(Symbol {
            name: alias_name,
            qualified_name: qname,
            kind: SymbolKind::TypeAlias,
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

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_symbols(child, source, file_path, module_name, scope_stack, symbols);
  }

  if scope_pushed {
    scope_stack.pop();
  }
}

pub fn extract_symbols_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<Symbol>, SwiftParseError> {
  let mut parser = Parser::new();
  parser
    .set_language(&tree_sitter_swift::LANGUAGE.into())
    .map_err(|e| SwiftParseError::UnsupportedLanguage(e.to_string()))?;

  let tree = parser
    .parse(source, None)
    .ok_or(SwiftParseError::ParseCancelled)?;
  let root = tree.root_node();

  let line_count = source.lines().count().max(1);
  let mut symbols = Vec::new();

  let module_name = module_name_from_path(file_path);

  // Add the file module symbol
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedImport {
  pub target_qualified_name: String,
  pub alias_name: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedInheritance {
  pub subclass_name: String,
  pub supertype_name: String,
  pub is_class_extends: bool,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedAttribute {
  pub target_symbol_qualified_name: String,
  pub attribute_name: String,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}

const SWIFT_PRIMITIVES: &[&str] = &[
  "String",
  "Int",
  "Double",
  "Float",
  "Bool",
  "Void",
  "Character",
  "Any",
  "AnyObject",
  "Self",
  "Array",
  "Dictionary",
  "Set",
  "Optional",
  "Int8",
  "Int16",
  "Int32",
  "Int64",
  "UInt",
  "UInt8",
  "UInt16",
  "UInt32",
  "UInt64",
  "Float32",
  "Float64",
];

fn extract_import_target(node: Node<'_>, source: &str) -> Option<String> {
  const SWIFT_IMPORT_KINDS: &[&str] =
    &["class", "struct", "protocol", "enum", "func", "var", "let"];
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    let kind = child.kind();
    if matches!(
      kind,
      "navigation_expression"
        | "simple_identifier"
        | "type_identifier"
        | "path_identifier"
        | "identifier"
    ) {
      let text = child
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .trim()
        .to_string();
      if !SWIFT_IMPORT_KINDS.contains(&text.as_str()) {
        return Some(text);
      }
    }
  }
  None
}

fn extract_attribute_name(node: Node<'_>, source: &str) -> Option<String> {
  let text = node.utf8_text(source.as_bytes()).unwrap_or("").trim();
  if let Some(clean) = text.strip_prefix('@') {
    let name = if let Some(idx) = clean.find('(') {
      clean[..idx].trim().to_string()
    } else {
      clean.trim().to_string()
    };
    if !name.is_empty() {
      return Some(name);
    }
  }
  None
}

fn extract_string_arg(node: Node<'_>, target_label: &str, source: &str) -> Option<String> {
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if child.kind() == "call_suffix" {
      let mut sub_cursor = child.walk();
      for sub_child in child.children(&mut sub_cursor) {
        if sub_child.kind() == "value_arguments" {
          let mut arg_cursor = sub_child.walk();
          for arg in sub_child.children(&mut arg_cursor) {
            if arg.kind() == "value_argument" {
              let mut label_matches = target_label.is_empty();
              let mut string_val = None;
              let mut label_cursor = arg.walk();
              for arg_child in arg.children(&mut label_cursor) {
                if arg_child.kind() == "value_argument_label" {
                  let label_text = arg_child.utf8_text(source.as_bytes()).unwrap_or("").trim();
                  label_matches = label_text == target_label;
                } else if arg_child.kind() == "line_string_literal" {
                  let raw_val = arg_child.utf8_text(source.as_bytes()).unwrap_or("");
                  let clean = raw_val
                    .trim()
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .unwrap_or(raw_val.trim());
                  string_val = Some(clean.to_string());
                }
              }
              if label_matches && string_val.is_some() {
                return string_val;
              }
            }
          }
        }
      }
    }
  }
  None
}

fn extract_call_target(node: Node<'_>, source: &str) -> Option<(String, bool, Option<String>)> {
  let first_child = node.child(0)?;
  let kind = first_child.kind();
  if kind == "navigation_expression" {
    let mut is_self = false;
    if let Some(obj_node) = first_child.child(0) {
      if obj_node.kind() == "simple_identifier" {
        let obj_text = obj_node.utf8_text(source.as_bytes()).unwrap_or("");
        if obj_text == "self" {
          is_self = true;
        }
      }
    }
    let mut method_name = None;
    let mut cursor = first_child.walk();
    for child in first_child.children(&mut cursor) {
      if child.kind() == "navigation_suffix" {
        if let Some(ident) = find_name_child(child) {
          method_name = Some(
            ident
              .utf8_text(source.as_bytes())
              .unwrap_or("")
              .trim()
              .to_string(),
          );
        }
      }
    }
    if let Some(mname) = method_name {
      return Some((mname.clone(), is_self, Some(mname)));
    }
  } else if matches!(kind, "simple_identifier" | "type_identifier") {
    let name = first_child
      .utf8_text(source.as_bytes())
      .unwrap_or("")
      .trim()
      .to_string();
    return Some((name, false, None));
  }
  None
}

fn find_user_attributes(
  node: Node<'_>,
  source: &str,
  attributes: &mut Vec<ExtractedAttribute>,
  target_qname: &str,
) {
  let kind = node.kind();
  if kind == "class_body"
    || kind == "struct_body"
    || kind == "enum_body"
    || kind == "protocol_body"
    || kind == "actor_body"
    || kind == "extension_body"
    || kind == "function_body"
  {
    return;
  }
  if kind == "attribute" || kind == "user_attribute" {
    if let Some(attr_name) = extract_attribute_name(node, source) {
      let start = node.start_position();
      let end = node.end_position();
      attributes.push(ExtractedAttribute {
        target_symbol_qualified_name: target_qname.to_string(),
        attribute_name: attr_name,
        start_line: start.row + 1,
        start_column: start.column,
        end_line: end.row + 1,
        end_column: end.column,
      });
    }
  }
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    find_user_attributes(child, source, attributes, target_qname);
  }
}

fn traverse_relations(
  node: Node<'_>,
  source: &str,
  file_path: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, SymbolKind)>,
  imports: &mut Vec<ExtractedImport>,
  calls: &mut Vec<ExtractedCall>,
  type_refs: &mut Vec<ExtractedTypeReference>,
  inheritance: &mut Vec<ExtractedInheritance>,
  attributes: &mut Vec<ExtractedAttribute>,
) {
  let kind = node.kind();
  let mut scope_pushed = false;

  match kind {
    "import_declaration" => {
      if let Some(target) = extract_import_target(node, source) {
        let start = node.start_position();
        let end = node.end_position();
        imports.push(ExtractedImport {
          target_qualified_name: target,
          alias_name: None,
          start_line: start.row + 1,
          start_column: start.column,
          end_line: end.row + 1,
          end_column: end.column,
        });
      }
      return;
    }
    "class_declaration"
    | "struct_declaration"
    | "enum_declaration"
    | "protocol_declaration"
    | "extension_declaration"
    | "actor_declaration" => {
      if let Some(name_node) = find_name_child(node) {
        let text = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string();
        if !text.is_empty() {
          let is_extension = kind == "extension_declaration" || {
            let mut cursor = node.walk();
            node.children(&mut cursor).any(|c| c.kind() == "extension")
          };
          let is_struct = kind == "struct_declaration" || {
            let mut cursor = node.walk();
            node.children(&mut cursor).any(|c| c.kind() == "struct")
          };
          let is_enum = kind == "enum_declaration" || {
            let mut cursor = node.walk();
            node.children(&mut cursor).any(|c| c.kind() == "enum")
          };
          let is_interface = kind == "protocol_declaration" || {
            let mut cursor = node.walk();
            node.children(&mut cursor).any(|c| c.kind() == "protocol")
          };

          let symbol_kind = if is_struct {
            SymbolKind::Struct
          } else if is_enum {
            SymbolKind::Enum
          } else if is_interface {
            SymbolKind::Interface
          } else {
            SymbolKind::Class
          };

          let qname = if scope_stack.is_empty() {
            format!("{}::{}", module_name, text)
          } else {
            format!(
              "{}::{}.{}",
              module_name,
              build_scope_path(scope_stack),
              text
            )
          };

          let mut is_first_inheritance = true;
          let mut cursor = node.walk();
          for child in node.children(&mut cursor) {
            if child.kind() == "inheritance_specifier" {
              if let Some(super_name) = extract_type_name(child, source) {
                let start = child.start_position();
                let end = child.end_position();
                let is_class_extends =
                  (symbol_kind == SymbolKind::Class) && !is_extension && is_first_inheritance;
                inheritance.push(ExtractedInheritance {
                  subclass_name: qname.clone(),
                  supertype_name: super_name,
                  is_class_extends,
                  start_line: start.row + 1,
                  start_column: start.column,
                  end_line: end.row + 1,
                  end_column: end.column,
                });
                is_first_inheritance = false;
              }
            }
          }

          find_user_attributes(node, source, attributes, &qname);

          scope_stack.push((text, symbol_kind));
          scope_pushed = true;
        }
      }
    }
    "function_declaration"
    | "protocol_function_declaration"
    | "init_declaration"
    | "initializer_declaration"
    | "deinit_declaration"
    | "deinitializer_declaration" => {
      let func_name = if kind.starts_with("init") {
        "init".to_string()
      } else if kind.starts_with("deinit") {
        "deinit".to_string()
      } else if let Some(name_node) = find_name_child(node) {
        name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string()
      } else {
        "".to_string()
      };

      if !func_name.is_empty() {
        let is_method = scope_stack
          .last()
          .map(|(_, k)| {
            *k == SymbolKind::Class
              || *k == SymbolKind::Struct
              || *k == SymbolKind::Enum
              || *k == SymbolKind::Interface
          })
          .unwrap_or(false);

        let symbol_kind = if is_method {
          SymbolKind::Method
        } else {
          SymbolKind::Function
        };

        let qname = if scope_stack.is_empty() {
          format!("{}::{}", module_name, func_name)
        } else {
          format!(
            "{}::{}.{}",
            module_name,
            build_scope_path(scope_stack),
            func_name
          )
        };

        find_user_attributes(node, source, attributes, &qname);

        scope_stack.push((func_name, symbol_kind));
        scope_pushed = true;
      }
    }
    "property_declaration" => {
      let prop_names = find_property_names(node, source);
      for prop_name in &prop_names {
        let qname = if scope_stack.is_empty() {
          format!("{}::{}", module_name, prop_name)
        } else {
          format!(
            "{}::{}.{}",
            module_name,
            build_scope_path(scope_stack),
            prop_name
          )
        };
        find_user_attributes(node, source, attributes, &qname);
      }

      let is_member_property = if let Some(parent) = node.parent() {
        let pkind = parent.kind();
        matches!(
          pkind,
          "class_body"
            | "struct_body"
            | "enum_body"
            | "protocol_body"
            | "actor_body"
            | "extension_body"
        )
      } else {
        false
      };

      let mut scope_pushed = false;
      if is_member_property {
        if let Some(prop_name) = prop_names.first() {
          scope_stack.push((prop_name.clone(), SymbolKind::Unknown));
          scope_pushed = true;
        }
      }

      let mut cursor = node.walk();
      for child in node.children(&mut cursor) {
        traverse_relations(
          child,
          source,
          file_path,
          module_name,
          scope_stack,
          imports,
          calls,
          type_refs,
          inheritance,
          attributes,
        );
      }

      if scope_pushed {
        scope_stack.pop();
      }
      return;
    }
    "call_expression" => {
      if let Some((target_name, is_self_call, method_name)) = extract_call_target(node, source) {
        let current_scope = if scope_stack.is_empty() {
          module_name.to_string()
        } else {
          format!("{}::{}", module_name, build_scope_path(scope_stack))
        };
        let start = node.start_position();
        let end = node.end_position();

        // Check for Storyboard/Nib/ViewController/Controller instantiations:
        if target_name == "UIStoryboard" || target_name == "NSStoryboard" {
          if let Some(sb_name) = extract_string_arg(node, "name", source) {
            calls.push(ExtractedCall {
              source_symbol_qualified_name: current_scope.clone(),
              target_name: format!("storyboard::{}", sb_name),
              is_self_call: false,
              method_name: None,
              start_line: start.row + 1,
              start_column: start.column,
              end_line: end.row + 1,
              end_column: end.column,
            });
          }
        } else if target_name == "UINib" || target_name == "NSNib" {
          let nib_name = extract_string_arg(node, "nibName", source)
            .or_else(|| extract_string_arg(node, "nibNamed", source))
            .or_else(|| extract_string_arg(node, "", source));
          if let Some(name) = nib_name {
            calls.push(ExtractedCall {
              source_symbol_qualified_name: current_scope.clone(),
              target_name: format!("nib::{}", name),
              is_self_call: false,
              method_name: None,
              start_line: start.row + 1,
              start_column: start.column,
              end_line: end.row + 1,
              end_column: end.column,
            });
          }
        } else if target_name == "instantiateViewController"
          || method_name.as_deref() == Some("instantiateViewController")
          || target_name == "instantiateController"
          || method_name.as_deref() == Some("instantiateController")
        {
          let vc_id = extract_string_arg(node, "withIdentifier", source)
            .or_else(|| extract_string_arg(node, "identifier", source));
          if let Some(id) = vc_id {
            calls.push(ExtractedCall {
              source_symbol_qualified_name: current_scope.clone(),
              target_name: format!("viewcontroller::{}", id),
              is_self_call: false,
              method_name: None,
              start_line: start.row + 1,
              start_column: start.column,
              end_line: end.row + 1,
              end_column: end.column,
            });
          }
        }

        calls.push(ExtractedCall {
          source_symbol_qualified_name: current_scope,
          target_name,
          is_self_call,
          method_name,
          start_line: start.row + 1,
          start_column: start.column,
          end_line: end.row + 1,
          end_column: end.column,
        });
      }
    }
    "macro_invocation" => {
      let mut macro_name = None;
      let mut cursor = node.walk();
      for child in node.children(&mut cursor) {
        if child.kind() == "simple_identifier" {
          let name = child.utf8_text(source.as_bytes()).unwrap_or("Preview");
          macro_name = Some(format!("#{}", name));
          break;
        }
      }

      if let Some(macro_name) = macro_name {
        let current_scope = if scope_stack.is_empty() {
          module_name.to_string()
        } else {
          format!("{}::{}", module_name, build_scope_path(scope_stack))
        };
        let start = node.start_position();
        let end = node.end_position();

        calls.push(ExtractedCall {
          source_symbol_qualified_name: current_scope,
          target_name: macro_name,
          is_self_call: false,
          method_name: None,
          start_line: start.row + 1,
          start_column: start.column,
          end_line: end.row + 1,
          end_column: end.column,
        });
      }
    }
    "user_type" => {
      let mut is_decl_name = false;
      if let Some(parent) = node.parent() {
        let pkind = parent.kind();
        if matches!(
          pkind,
          "class_declaration"
            | "struct_declaration"
            | "enum_declaration"
            | "protocol_declaration"
            | "extension_declaration"
            | "actor_declaration"
            | "typealias_declaration"
        ) {
          if let Some(name_node) = find_name_child(parent) {
            if name_node == node || name_node.parent() == Some(node) {
              is_decl_name = true;
            }
          }
        }
      }

      if !is_decl_name {
        if let Some(type_name) = extract_type_name(node, source) {
          if !SWIFT_PRIMITIVES.contains(&type_name.as_str()) {
            let mut parent_prop = None;
            let mut curr = node;
            while let Some(p) = curr.parent() {
              if p.kind() == "property_declaration" {
                parent_prop = Some(p);
                break;
              }
              curr = p;
            }

            let start = node.start_position();
            let end = node.end_position();

            if let Some(prop_node) = parent_prop {
              for prop_name in find_property_names(prop_node, source) {
                let source_qname = if scope_stack.is_empty() {
                  format!("{}::{}", module_name, prop_name)
                } else {
                  let mut temp_stack = scope_stack.clone();
                  if temp_stack.last().map(|(n, _)| n) == Some(&prop_name) {
                    temp_stack.pop();
                  }
                  format!(
                    "{}::{}.{}",
                    module_name,
                    build_scope_path(&temp_stack),
                    prop_name
                  )
                };
                type_refs.push(ExtractedTypeReference {
                  source_symbol_qualified_name: source_qname,
                  target_type_name: type_name.clone(),
                  start_line: start.row + 1,
                  start_column: start.column,
                  end_line: end.row + 1,
                  end_column: end.column,
                });
              }
            } else {
              let current_scope = if scope_stack.is_empty() {
                module_name.to_string()
              } else {
                format!("{}::{}", module_name, build_scope_path(scope_stack))
              };
              type_refs.push(ExtractedTypeReference {
                source_symbol_qualified_name: current_scope,
                target_type_name: type_name,
                start_line: start.row + 1,
                start_column: start.column,
                end_line: end.row + 1,
                end_column: end.column,
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
    traverse_relations(
      child,
      source,
      file_path,
      module_name,
      scope_stack,
      imports,
      calls,
      type_refs,
      inheritance,
      attributes,
    );
  }

  if scope_pushed {
    scope_stack.pop();
  }
}

fn run_traverse_relations(
  source: &str,
  file_path: &str,
) -> Result<
  (
    Vec<ExtractedImport>,
    Vec<ExtractedCall>,
    Vec<ExtractedTypeReference>,
    Vec<ExtractedInheritance>,
    Vec<ExtractedAttribute>,
  ),
  SwiftParseError,
> {
  let mut parser = Parser::new();
  parser
    .set_language(&tree_sitter_swift::LANGUAGE.into())
    .map_err(|e| SwiftParseError::UnsupportedLanguage(e.to_string()))?;

  let tree = parser
    .parse(source, None)
    .ok_or(SwiftParseError::ParseCancelled)?;
  let root = tree.root_node();

  let module_name = module_name_from_path(file_path);

  let mut scope_stack = Vec::new();
  let mut imports = Vec::new();
  let mut calls = Vec::new();
  let mut type_refs = Vec::new();
  let mut inheritance = Vec::new();
  let mut attributes = Vec::new();

  traverse_relations(
    root,
    source,
    file_path,
    &module_name,
    &mut scope_stack,
    &mut imports,
    &mut calls,
    &mut type_refs,
    &mut inheritance,
    &mut attributes,
  );

  Ok((imports, calls, type_refs, inheritance, attributes))
}

pub fn extract_imports_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedImport>, SwiftParseError> {
  let (imports, _, _, _, _) = run_traverse_relations(source, file_path)?;
  Ok(imports)
}

pub fn extract_calls_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedCall>, SwiftParseError> {
  let (_, calls, _, _, _) = run_traverse_relations(source, file_path)?;
  Ok(calls)
}

pub fn extract_type_references_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedTypeReference>, SwiftParseError> {
  let (_, _, type_refs, _, _) = run_traverse_relations(source, file_path)?;
  Ok(type_refs)
}

pub fn extract_inheritance_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedInheritance>, SwiftParseError> {
  let (_, _, _, inheritance, _) = run_traverse_relations(source, file_path)?;
  Ok(inheritance)
}

pub fn extract_attributes_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedAttribute>, SwiftParseError> {
  let (_, _, _, _, attributes) = run_traverse_relations(source, file_path)?;
  Ok(attributes)
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
pub struct ExtractedConcurrencySpawn {
  pub source_symbol_qualified_name: String,
  pub spawn_kind: String,
  pub target_name: Option<String>,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedChannelPair {
  pub source_symbol_qualified_name: String,
  pub channel_kind: String,
  pub tx_name: String,
  pub rx_name: String,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedSelectBlock {
  pub source_symbol_qualified_name: String,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}

pub fn extract_tests_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedTest>, SwiftParseError> {
  let mut parser = Parser::new();
  parser
    .set_language(&tree_sitter_swift::LANGUAGE.into())
    .map_err(|e| SwiftParseError::UnsupportedLanguage(e.to_string()))?;

  let tree = parser
    .parse(source, None)
    .ok_or(SwiftParseError::ParseCancelled)?;
  let root = tree.root_node();

  let module_name = module_name_from_path(file_path);
  let mut tests = Vec::new();
  let mut scope_stack: Vec<(String, SymbolKind, bool)> = Vec::new();
  traverse_tests(
    root,
    source,
    file_path,
    &module_name,
    &mut scope_stack,
    &mut tests,
  );

  Ok(tests)
}

fn has_test_attribute(node: Node<'_>, source: &str) -> Option<(bool, bool)> {
  let kind = node.kind();
  if kind == "attribute" || kind == "user_attribute" {
    let text = node.utf8_text(source.as_bytes()).unwrap_or("").trim();
    if text.starts_with("@Test") {
      let is_parametrized = text.contains("arguments:");
      return Some((true, is_parametrized));
    }
  }
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if let Some(res) = has_test_attribute(child, source) {
      return Some(res);
    }
  }
  None
}

fn traverse_tests(
  node: Node<'_>,
  source: &str,
  file_path: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, SymbolKind, bool)>,
  tests: &mut Vec<ExtractedTest>,
) {
  let kind = node.kind();
  let mut scope_pushed = false;

  match kind {
    "class_declaration" => {
      if let Some(name_node) = find_name_child(node) {
        let class_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string();
        if !class_name.is_empty() {
          let mut inherits_xctest = false;
          let mut cursor = node.walk();
          for child in node.children(&mut cursor) {
            if child.kind() == "inheritance_specifier" {
              if let Some(super_name) = extract_type_name(child, source) {
                if super_name == "XCTestCase" {
                  inherits_xctest = true;
                  break;
                }
              }
            }
          }

          let qname = if scope_stack.is_empty() {
            format!("{}::{}", module_name, class_name)
          } else {
            format!(
              "{}::{}.{}",
              module_name,
              build_test_scope_path(scope_stack),
              class_name
            )
          };

          if inherits_xctest {
            let start = node.start_position();
            let end = node.end_position();
            tests.push(ExtractedTest {
              name: class_name.clone(),
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

          scope_stack.push((class_name, SymbolKind::Class, inherits_xctest));
          scope_pushed = true;
        }
      }
    }
    "function_declaration" => {
      if let Some(name_node) = find_name_child(node) {
        let func_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string();
        if !func_name.is_empty() {
          let qname = if scope_stack.is_empty() {
            format!("{}::{}", module_name, func_name)
          } else {
            format!(
              "{}::{}.{}",
              module_name,
              build_test_scope_path(scope_stack),
              func_name
            )
          };

          let mut is_test = false;
          let mut is_parametrized = false;

          let in_xctest_class = scope_stack
            .last()
            .map(|(_, k, is_xctest)| *k == SymbolKind::Class && *is_xctest)
            .unwrap_or(false)
            && func_name.starts_with("test");

          if in_xctest_class {
            is_test = true;
          }

          if let Some((has_test, is_param)) = has_test_attribute(node, source) {
            if has_test {
              is_test = true;
              is_parametrized = is_param;
            }
          }

          if is_test {
            let start = node.start_position();
            let end = node.end_position();
            tests.push(ExtractedTest {
              name: func_name.clone(),
              qualified_name: qname,
              kind: ExtractedTestKind::Function,
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
    }
    _ => {}
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_tests(child, source, file_path, module_name, scope_stack, tests);
  }

  if scope_pushed {
    scope_stack.pop();
  }
}

pub fn extract_concurrency_from_source(
  source: &str,
  file_path: &str,
) -> Result<
  (
    Vec<ExtractedConcurrencySpawn>,
    Vec<ExtractedChannelPair>,
    Vec<ExtractedSelectBlock>,
  ),
  SwiftParseError,
> {
  let mut parser = Parser::new();
  parser
    .set_language(&tree_sitter_swift::LANGUAGE.into())
    .map_err(|e| SwiftParseError::UnsupportedLanguage(e.to_string()))?;

  let tree = parser
    .parse(source, None)
    .ok_or(SwiftParseError::ParseCancelled)?;
  let root = tree.root_node();

  let module_name = module_name_from_path(file_path);
  let mut spawns = Vec::new();
  let mut channels = Vec::new();
  let mut selects = Vec::new();
  let mut scope_stack = Vec::new();

  traverse_concurrency(
    root,
    source,
    &module_name,
    &mut scope_stack,
    &mut spawns,
    &mut channels,
    &mut selects,
  );

  Ok((spawns, channels, selects))
}

fn traverse_concurrency(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, SymbolKind)>,
  spawns: &mut Vec<ExtractedConcurrencySpawn>,
  channels: &mut Vec<ExtractedChannelPair>,
  selects: &mut Vec<ExtractedSelectBlock>,
) {
  let kind = node.kind();
  let mut scope_pushed = false;

  match kind {
    "class_declaration"
    | "struct_declaration"
    | "enum_declaration"
    | "protocol_declaration"
    | "extension_declaration"
    | "actor_declaration" => {
      if let Some(name_node) = find_name_child(node) {
        let text = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string();
        if !text.is_empty() {
          let symbol_kind = if kind == "struct_declaration" {
            SymbolKind::Struct
          } else if kind == "enum_declaration" {
            SymbolKind::Enum
          } else if kind == "protocol_declaration" {
            SymbolKind::Interface
          } else {
            SymbolKind::Class
          };
          scope_stack.push((text, symbol_kind));
          scope_pushed = true;
        }
      }
    }
    "function_declaration" | "init_declaration" | "deinit_declaration" => {
      let func_name = if kind.starts_with("init") {
        "init".to_string()
      } else if kind.starts_with("deinit") {
        "deinit".to_string()
      } else if let Some(name_node) = find_name_child(node) {
        name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string()
      } else {
        "".to_string()
      };
      if !func_name.is_empty() {
        scope_stack.push((func_name, SymbolKind::Function));
        scope_pushed = true;
      }
    }
    "call_expression" => {
      if let Some((target_name, _, method_name)) = extract_call_target(node, source) {
        let is_spawn = target_name == "Task"
          || method_name.as_deref() == Some("detached")
          || method_name.as_deref() == Some("addTask");

        if is_spawn {
          let spawn_kind = if method_name.as_deref() == Some("detached") {
            "Task.detached".to_string()
          } else if method_name.as_deref() == Some("addTask") {
            "addTask".to_string()
          } else {
            "Task".to_string()
          };

          let current_scope = if scope_stack.is_empty() {
            module_name.to_string()
          } else {
            format!("{}::{}", module_name, build_scope_path(scope_stack))
          };

          let start = node.start_position();
          let end = node.end_position();

          spawns.push(ExtractedConcurrencySpawn {
            source_symbol_qualified_name: current_scope,
            spawn_kind,
            target_name: None,
            start_line: start.row + 1,
            start_column: start.column,
            end_line: end.row + 1,
            end_column: end.column,
          });
        }
      }
    }
    "for_statement" => {
      let text = node.utf8_text(source.as_bytes()).unwrap_or("");
      if text.contains("await") {
        let current_scope = if scope_stack.is_empty() {
          module_name.to_string()
        } else {
          format!("{}::{}", module_name, build_scope_path(scope_stack))
        };
        let start = node.start_position();
        let end = node.end_position();
        selects.push(ExtractedSelectBlock {
          source_symbol_qualified_name: current_scope,
          start_line: start.row + 1,
          start_column: start.column,
          end_line: end.row + 1,
          end_column: end.column,
        });
      }
    }
    _ => {
      if kind == "property_declaration" || kind == "pattern_binding" || kind == "local_declaration"
      {
        let text = node.utf8_text(source.as_bytes()).unwrap_or("");
        if text.contains("AsyncStream") || text.contains("AsyncChannel") {
          let kind_str = if text.contains("AsyncChannel") {
            "AsyncChannel"
          } else {
            "AsyncStream"
          };
          let current_scope = if scope_stack.is_empty() {
            module_name.to_string()
          } else {
            format!("{}::{}", module_name, build_scope_path(scope_stack))
          };
          let start = node.start_position();
          let end = node.end_position();

          let mut tx_name = "continuation".to_string();
          let mut rx_name = "stream".to_string();

          if let Some(eq_idx) = text.find('=') {
            let lhs = &text[..eq_idx].trim();
            if lhs.contains('(') && lhs.contains(')') {
              if let Some(start_paren) = lhs.find('(') {
                if let Some(end_paren) = lhs.find(')') {
                  let vars: Vec<&str> = lhs[start_paren + 1..end_paren]
                    .split(',')
                    .map(|s| s.trim())
                    .collect();
                  if vars.len() >= 2 {
                    rx_name = vars[0].to_string();
                    tx_name = vars[1].to_string();
                  }
                }
              }
            } else {
              rx_name = lhs.replace("let", "").replace("var", "").trim().to_string();
            }
          }

          channels.push(ExtractedChannelPair {
            source_symbol_qualified_name: current_scope,
            channel_kind: kind_str.to_string(),
            tx_name,
            rx_name,
            start_line: start.row + 1,
            start_column: start.column,
            end_line: end.row + 1,
            end_column: end.column,
          });
        }
      }
    }
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_concurrency(
      child,
      source,
      module_name,
      scope_stack,
      spawns,
      channels,
      selects,
    );
  }

  if scope_pushed {
    scope_stack.pop();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_tdd_extract_swift_symbols() {
    let source = r#"
    import Foundation

    struct Patient {
      var name: String
      func displayInfo() {
        print("Patient: \(name)")
      }
    }
    "#;

    let symbols = extract_symbols_from_source(source, "src/Patient.swift").unwrap();

    assert!(
      symbols.len() >= 2,
      "Expected at least 2 symbols, got {}",
      symbols.len()
    );
    let struct_sym = symbols
      .iter()
      .find(|s| s.name == "Patient" && s.kind == SymbolKind::Struct)
      .unwrap();
    assert_eq!(struct_sym.qualified_name, "src.Patient::Patient");

    let method_sym = symbols
      .iter()
      .find(|s| s.name == "displayInfo" && s.kind == SymbolKind::Method)
      .unwrap();
    assert_eq!(
      method_sym.qualified_name,
      "src.Patient::Patient.displayInfo"
    );

    let prop_sym = symbols
      .iter()
      .find(|s| s.name == "name" && s.kind == SymbolKind::Unknown)
      .unwrap();
    assert_eq!(prop_sym.qualified_name, "src.Patient::Patient.name");
  }

  #[test]
  fn test_swift_protocols_extensions_and_classes() {
    let source = r#"
    protocol Describable {
      func getDescription() -> String
    }

    class Document: Describable {
      var title: String
      init(title: String) {
        self.title = title
      }
      func getDescription() -> String {
        return "Document: \(title)"
      }
    }

    extension Document {
      func printTitle() {
        print(title)
      }
    }
    "#;

    let symbols = extract_symbols_from_source(source, "src/Document.swift").unwrap();

    let proto_sym = symbols
      .iter()
      .find(|s| s.name == "Describable" && s.kind == SymbolKind::Interface)
      .unwrap();
    assert_eq!(proto_sym.qualified_name, "src.Document::Describable");

    let class_sym = symbols
      .iter()
      .find(|s| s.name == "Document" && s.kind == SymbolKind::Class)
      .unwrap();
    assert_eq!(class_sym.qualified_name, "src.Document::Document");

    let init_sym = symbols
      .iter()
      .find(|s| s.name == "init" && s.kind == SymbolKind::Method)
      .unwrap();
    assert_eq!(init_sym.qualified_name, "src.Document::Document.init");

    // Extension itself must not be registered as a duplicate symbol
    let ext_sym = symbols
      .iter()
      .find(|s| s.name == "Document" && s.kind == SymbolKind::Class && s.location.start_line == 17);
    assert!(
      ext_sym.is_none(),
      "Extension block itself must not be pushed as duplicate symbol!"
    );

    let ext_method = symbols
      .iter()
      .find(|s| s.name == "printTitle" && s.kind == SymbolKind::Method)
      .unwrap();
    assert_eq!(
      ext_method.qualified_name,
      "src.Document::Document.printTitle"
    );
  }

  #[test]
  fn test_swift_actors_and_multi_property_declarations() {
    let source = r#"
    actor BankAccount {
      var balance: Double
      let owner, id: String
      init(owner: String, id: String) {
        self.owner = owner
        self.id = id
        self.balance = 0
      }
      func getBalance() -> Double {
        return balance
      }
    }
    "#;

    let symbols = extract_symbols_from_source(source, "BankAccount.swift").unwrap();

    let actor_sym = symbols
      .iter()
      .find(|s| s.name == "BankAccount" && s.kind == SymbolKind::Class)
      .unwrap();
    assert_eq!(actor_sym.qualified_name, "BankAccount::BankAccount");

    // owner, id, balance properties
    let owner_prop = symbols
      .iter()
      .find(|s| s.name == "owner" && s.kind == SymbolKind::Unknown)
      .unwrap();
    assert_eq!(owner_prop.qualified_name, "BankAccount::BankAccount.owner");

    let id_prop = symbols
      .iter()
      .find(|s| s.name == "id" && s.kind == SymbolKind::Unknown)
      .unwrap();
    assert_eq!(id_prop.qualified_name, "BankAccount::BankAccount.id");

    let balance_prop = symbols
      .iter()
      .find(|s| s.name == "balance" && s.kind == SymbolKind::Unknown)
      .unwrap();
    assert_eq!(
      balance_prop.qualified_name,
      "BankAccount::BankAccount.balance"
    );

    let get_bal = symbols
      .iter()
      .find(|s| s.name == "getBalance" && s.kind == SymbolKind::Method)
      .unwrap();
    assert_eq!(
      get_bal.qualified_name,
      "BankAccount::BankAccount.getBalance"
    );
  }

  #[test]
  fn test_absolute_path_module_name_resolution() {
    let name = module_name_from_path("/home/user/project/src/Utility.swift");
    assert_eq!(name, "home.user.project.src.Utility");
  }

  fn print_node_tree(node: Node<'_>, source: &str, depth: usize) {
    let indent = "  ".repeat(depth);
    let text = node.utf8_text(source.as_bytes()).unwrap_or("").trim();
    let text_cropped = if text.len() > 60 { &text[..60] } else { text };
    println!(
      "{}{:?} kind: {:?} text: {:?}",
      indent,
      node,
      node.kind(),
      text_cropped
    );
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
      print_node_tree(child, source, depth + 1);
    }
  }

  #[test]
  fn test_extract_swift_relationships_tdd() {
    let source = r#"
    import Foundation
    import class UIKit.UIView

    @objc class PatientManager: NSObject {
      var patient: Patient
      var title: String = ""
      var records: [MedicalRecord] = []

      @discardableResult
      func updatePatient(id: String, age: Int, record: MedicalRecord) -> Bool {
        let result = record.verify()
        patient.displayInfo()
        return result
      }
    }

    struct MedicalRecord: Describable {
      func getDescription() -> String {
        return "Record"
      }
    }

    extension PatientManager: Describable {
      func reset() {}
      func getDescription() -> String {
        return "Manager"
      }
    }
    "#;

    let path = "src/Patient.swift";

    let mut parser = Parser::new();
    parser
      .set_language(&tree_sitter_swift::LANGUAGE.into())
      .unwrap();
    let tree = parser.parse(source, None).unwrap();
    print_node_tree(tree.root_node(), source, 0);

    // 1. Imports
    let imports = extract_imports_from_source(source, path).unwrap();
    assert!(
      imports
        .iter()
        .any(|i| i.target_qualified_name == "Foundation")
    );
    assert!(
      imports
        .iter()
        .any(|i| i.target_qualified_name == "UIKit.UIView")
    );

    // 2. Attributes
    let attrs = extract_attributes_from_source(source, path).unwrap();
    println!("EXTRACTED ATTRIBUTES: {:?}", attrs);
    assert!(attrs.iter().any(|a| a.attribute_name == "objc"
      && a.target_symbol_qualified_name == "src.Patient::PatientManager"));
    assert!(attrs.iter().any(|a| a.attribute_name == "discardableResult"
      && a.target_symbol_qualified_name == "src.Patient::PatientManager.updatePatient"));

    // 3. Inheritance / Extensions
    let inheritance = extract_inheritance_from_source(source, path).unwrap();
    // PatientManager inherits NSObject (class inheritance)
    let pm_ns = inheritance
      .iter()
      .find(|i| i.subclass_name == "src.Patient::PatientManager" && i.supertype_name == "NSObject")
      .unwrap();
    assert!(pm_ns.is_class_extends);

    // MedicalRecord inherits Describable (protocol implementation)
    let mr_desc = inheritance
      .iter()
      .find(|i| {
        i.subclass_name == "src.Patient::MedicalRecord" && i.supertype_name == "Describable"
      })
      .unwrap();
    assert!(!mr_desc.is_class_extends);

    // Extension on PatientManager implements Describable
    let ext_pm = inheritance
      .iter()
      .find(|i| {
        i.subclass_name == "src.Patient::PatientManager" && i.supertype_name == "Describable"
      })
      .unwrap();
    assert!(!ext_pm.is_class_extends);

    // 4. Type References
    let type_refs = extract_type_references_from_source(source, path).unwrap();
    // patient: Patient is a type reference
    assert!(type_refs.iter().any(|t| t.source_symbol_qualified_name
      == "src.Patient::PatientManager.patient"
      && t.target_type_name == "Patient"));
    // records: [MedicalRecord] should refer to MedicalRecord
    assert!(type_refs.iter().any(|t| t.source_symbol_qualified_name
      == "src.Patient::PatientManager.records"
      && t.target_type_name == "MedicalRecord"));
    // title: String should be filtered out because String is a primitive
    assert!(!type_refs.iter().any(|t| t.target_type_name == "String"));
    // age: Int should be filtered out (primitive)
    assert!(!type_refs.iter().any(|t| t.target_type_name == "Int"));
    // record: MedicalRecord is a type reference
    assert!(type_refs.iter().any(|t| t.source_symbol_qualified_name
      == "src.Patient::PatientManager.updatePatient"
      && t.target_type_name == "MedicalRecord"));
    // -> Bool should be filtered out (primitive)
    assert!(!type_refs.iter().any(|t| t.target_type_name == "Bool"));

    // 5. Calls
    let calls = extract_calls_from_source(source, path).unwrap();
    // record.verify() -> call target verify or record.verify
    assert!(calls.iter().any(|c| c.source_symbol_qualified_name
      == "src.Patient::PatientManager.updatePatient"
      && c.target_name == "verify"));
    // patient.displayInfo() -> call target displayInfo
    assert!(calls.iter().any(|c| c.source_symbol_qualified_name
      == "src.Patient::PatientManager.updatePatient"
      && c.target_name == "displayInfo"));
  }

  #[test]
  fn test_extract_swift_tests() {
    let source = r#"
    import XCTest

    class PatientTests: XCTestCase {
      func testValidation() {}
      func testPerformance() {}
    }

    @Test func testModernSwift() {}
    @Test(arguments: 1...5) func testParametrized(arg: Int) {}
    "#;

    let tests = extract_tests_from_source(source, "src/PatientTests.swift").unwrap();
    assert_eq!(tests.len(), 5);

    let class_test = tests
      .iter()
      .find(|t| t.kind == ExtractedTestKind::Class)
      .unwrap();
    assert_eq!(class_test.name, "PatientTests");
    assert_eq!(class_test.qualified_name, "src.PatientTests::PatientTests");

    let func_test1 = tests.iter().find(|t| t.name == "testValidation").unwrap();
    assert_eq!(func_test1.kind, ExtractedTestKind::Function);
    assert_eq!(
      func_test1.qualified_name,
      "src.PatientTests::PatientTests.testValidation"
    );

    let func_test2 = tests.iter().find(|t| t.name == "testModernSwift").unwrap();
    assert_eq!(func_test2.kind, ExtractedTestKind::Function);
    assert!(!func_test2.is_parametrized);

    let func_test3 = tests.iter().find(|t| t.name == "testParametrized").unwrap();
    assert_eq!(func_test3.kind, ExtractedTestKind::Function);
    assert!(func_test3.is_parametrized);
  }

  #[test]
  fn test_extract_swift_concurrency() {
    let source = r#"
    func process() {
      Task {
        print("Spawn unstructured task")
      }
      Task.detached {
        print("Spawn detached task")
      }
      let (stream, continuation) = AsyncStream.makeStream(of: Int.self)
      for await item in stream {
        print(item)
      }
    }
    "#;

    let (spawns, channels, selects) =
      extract_concurrency_from_source(source, "src/Concurrency.swift").unwrap();

    assert_eq!(spawns.len(), 2);
    assert_eq!(spawns[0].spawn_kind, "Task");
    assert_eq!(spawns[1].spawn_kind, "Task.detached");

    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0].channel_kind, "AsyncStream");
    assert_eq!(channels[0].tx_name, "continuation");
    assert_eq!(channels[0].rx_name, "stream");

    assert_eq!(selects.len(), 1);
    assert!(selects[0].source_symbol_qualified_name.contains("process"));
  }

  #[test]
  fn test_extract_swift_ui_and_uikit_depth() {
    let source = r#"
    struct MyView: View {
      @State var count: Int = 0
      @Binding var isActive: Bool
      @ObservedObject var model: MyModel
      @EnvironmentObject var settings: Settings

      var body: some View {
        VStack {
          Text("Count: \(count)")
          NavigationLink(destination: OtherView()) {
            Text("Go")
          }
          Button("Increment") {
            count += 1
          }
        }
      }
    }

    #Preview {
      MyView()
    }

    class MyViewController: UIViewController, UITableViewDelegate {
      @IBOutlet var myButton: UIButton!

      override func viewDidLoad() {
        super.viewDidLoad()
        let storyboard = UIStoryboard(name: "Main", bundle: nil)
        let vc = storyboard.instantiateViewController(identifier: "DetailVC")
        let nib = UINib(nibName: "CustomCell", bundle: nil)

        let nsStoryboard = NSStoryboard(name: "MacMain", bundle: nil)
        let nsVC = nsStoryboard.instantiateController(withIdentifier: "MacVC")
        let nsNib = NSNib(nibNamed: "MacCell", bundle: nil)
      }

      @IBAction func tapped(_ sender: Any) {}
    }
    "#;

    let path = "src/MyView.swift";

    // 1. Check Attributes on properties and functions
    let attributes = extract_attributes_from_source(source, path).unwrap();

    // @State count
    assert!(attributes.iter().any(|a| a.attribute_name == "State"
      && a.target_symbol_qualified_name == "src.MyView::MyView.count"));

    // @Binding isActive
    assert!(attributes.iter().any(|a| a.attribute_name == "Binding"
      && a.target_symbol_qualified_name == "src.MyView::MyView.isActive"));

    // @ObservedObject model
    assert!(
      attributes
        .iter()
        .any(|a| a.attribute_name == "ObservedObject"
          && a.target_symbol_qualified_name == "src.MyView::MyView.model")
    );

    // @EnvironmentObject settings
    assert!(
      attributes
        .iter()
        .any(|a| a.attribute_name == "EnvironmentObject"
          && a.target_symbol_qualified_name == "src.MyView::MyView.settings")
    );

    // @IBOutlet myButton
    assert!(attributes.iter().any(|a| a.attribute_name == "IBOutlet"
      && a.target_symbol_qualified_name == "src.MyView::MyViewController.myButton"));

    // @IBAction tapped
    assert!(attributes.iter().any(|a| a.attribute_name == "IBAction"
      && a.target_symbol_qualified_name == "src.MyView::MyViewController.tapped"));

    // 2. Check Calls (SwiftUI View composition and Storyboard/Nib/ViewController references)
    let calls = extract_calls_from_source(source, path).unwrap();

    // Nested view calls inside computed property 'body'
    assert!(
      calls.iter().any(|c| c.target_name == "VStack"
        && c.source_symbol_qualified_name == "src.MyView::MyView.body")
    );
    assert!(calls.iter().any(
      |c| c.target_name == "Text" && c.source_symbol_qualified_name == "src.MyView::MyView.body"
    ));
    assert!(calls.iter().any(|c| c.target_name == "NavigationLink"
      && c.source_symbol_qualified_name == "src.MyView::MyView.body"));
    assert!(calls.iter().any(|c| c.target_name == "OtherView"
      && c.source_symbol_qualified_name == "src.MyView::MyView.body"));
    assert!(
      calls.iter().any(|c| c.target_name == "Button"
        && c.source_symbol_qualified_name == "src.MyView::MyView.body")
    );
    assert!(calls.iter().any(|c| c.target_name == "MyView"));

    // Freestanding macro Preview call
    assert!(calls.iter().any(|c| c.target_name == "#Preview"));

    // Storyboard/Nib/ViewController virtual target calls (UIKit)
    assert!(calls.iter().any(|c| c.target_name == "storyboard::Main"
      && c.source_symbol_qualified_name == "src.MyView::MyViewController.viewDidLoad"));
    assert!(
      calls
        .iter()
        .any(|c| c.target_name == "viewcontroller::DetailVC"
          && c.source_symbol_qualified_name == "src.MyView::MyViewController.viewDidLoad")
    );
    assert!(calls.iter().any(|c| c.target_name == "nib::CustomCell"
      && c.source_symbol_qualified_name == "src.MyView::MyViewController.viewDidLoad"));

    // Storyboard/Nib/ViewController virtual target calls (AppKit)
    assert!(calls.iter().any(|c| c.target_name == "storyboard::MacMain"
      && c.source_symbol_qualified_name == "src.MyView::MyViewController.viewDidLoad"));
    assert!(
      calls
        .iter()
        .any(|c| c.target_name == "viewcontroller::MacVC"
          && c.source_symbol_qualified_name == "src.MyView::MyViewController.viewDidLoad")
    );
    assert!(calls.iter().any(|c| c.target_name == "nib::MacCell"
      && c.source_symbol_qualified_name == "src.MyView::MyViewController.viewDidLoad"));

    // 3. Check Protocol Conformance
    let inheritance = extract_inheritance_from_source(source, path).unwrap();
    assert!(
      inheritance
        .iter()
        .any(|i| i.subclass_name == "src.MyView::MyViewController"
          && i.supertype_name == "UITableViewDelegate")
    );
  }
}
