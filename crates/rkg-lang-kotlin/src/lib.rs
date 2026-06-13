#![allow(clippy::collapsible_if)]
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::path::Path;

use rkg_core::{Location, Symbol, SymbolKind};
use tree_sitter::{Node, Parser};

pub const CRATE_NAME: &str = "rkg-lang-kotlin";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KotlinParseError {
  UnsupportedLanguage(String),
  ParseCancelled,
}

impl Display for KotlinParseError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      KotlinParseError::UnsupportedLanguage(message) => {
        write!(f, "failed to initialize kotlin parser: {message}")
      }
      KotlinParseError::ParseCancelled => write!(f, "kotlin parsing was cancelled"),
    }
  }
}

impl std::error::Error for KotlinParseError {}

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
pub struct ExtractedAnnotation {
  pub target_symbol_qualified_name: String,
  pub annotation_name: String,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}

const KOTLIN_BUILTINS: &[&str] = &[
  "String",
  "Int",
  "Boolean",
  "Double",
  "Float",
  "Long",
  "Short",
  "Byte",
  "Char",
  "Unit",
  "Any",
  "Nothing",
  "List",
  "Set",
  "Map",
  "ArrayList",
  "HashMap",
  "HashSet",
  "Array",
  "Option",
  "Optional",
  "Result",
];

fn clean_type_name(t: &str) -> String {
  let cleaned = if let Some(idx) = t.find('<') {
    &t[..idx]
  } else {
    t
  };
  cleaned
    .trim()
    .trim_end_matches(['?', '!'])
    .trim()
    .to_string()
}

fn extract_delegation_specifiers(
  node: Node<'_>,
  source: &str,
  scope_stack: &[(String, SymbolKind)],
  inheritance: &mut Vec<ExtractedInheritance>,
) {
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if child.kind() == "delegation_specifiers" {
      let mut sub_cursor = child.walk();
      for spec in child.children(&mut sub_cursor) {
        if spec.kind() == "delegation_specifier" {
          let mut super_name = String::new();
          let mut is_class = false;

          let mut spec_cursor = spec.walk();
          for spec_child in spec.children(&mut spec_cursor) {
            if spec_child.kind() == "constructor_invocation" {
              is_class = true;
              if let Some(ut) = spec_child.child_by_field_name("type") {
                if let Ok(t) = ut.utf8_text(source.as_bytes()) {
                  super_name = t.trim().to_string();
                }
              } else {
                let mut ci_cursor = spec_child.walk();
                if let Some(ut) = spec_child
                  .children(&mut ci_cursor)
                  .find(|c| c.kind() == "user_type")
                {
                  if let Ok(t) = ut.utf8_text(source.as_bytes()) {
                    super_name = t.trim().to_string();
                  }
                }
              }
            } else if spec_child.kind() == "user_type" {
              if let Ok(t) = spec_child.utf8_text(source.as_bytes()) {
                super_name = t.trim().to_string();
              }
            }
          }

          if !super_name.is_empty() {
            let super_name = clean_type_name(&super_name);
            let subclass_path = build_scope_path(scope_stack);
            let start = spec.start_position();
            let end = spec.end_position();
            inheritance.push(ExtractedInheritance {
              subclass_name: subclass_path,
              supertype_name: super_name,
              is_class_extends: is_class,
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
}

fn current_qualified_name(module_name: &str, scope_stack: &[(String, SymbolKind)]) -> String {
  if scope_stack.is_empty() {
    module_name.to_string()
  } else {
    let mut parts = Vec::new();
    for (name, _) in scope_stack {
      parts.push(name.as_str());
    }
    format!("{}::{}", module_name, parts.join("."))
  }
}

fn is_inside_import_or_package(node: Node<'_>) -> bool {
  let mut curr = node;
  while let Some(parent) = curr.parent() {
    let pkind = parent.kind();
    if pkind == "import" || pkind == "package_header" || pkind == "annotation" {
      return true;
    }
    curr = parent;
  }
  false
}

fn extract_node_annotations(
  node: Node<'_>,
  source: &str,
  target_symbol_qualified_name: &str,
  annotations: &mut Vec<ExtractedAnnotation>,
) {
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if child.kind() == "modifiers" {
      let mut mod_cursor = child.walk();
      for modifier in child.children(&mut mod_cursor) {
        if modifier.kind() == "annotation" {
          let mut ann_cursor = modifier.walk();
          let ut_node = modifier
            .children(&mut ann_cursor)
            .find(|c| c.kind() == "user_type")
            .or_else(|| {
              let mut ann_cursor2 = modifier.walk();
              modifier
                .children(&mut ann_cursor2)
                .find(|c| c.kind() == "constructor_invocation")
                .and_then(|ci| {
                  let mut ci_cursor = ci.walk();
                  ci.children(&mut ci_cursor)
                    .find(|c| c.kind() == "user_type")
                })
            });

          if let Some(ut) = ut_node {
            if let Ok(text) = ut.utf8_text(source.as_bytes()) {
              let name = text.trim().to_string();
              if !name.is_empty() {
                let start = modifier.start_position();
                let end = modifier.end_position();
                annotations.push(ExtractedAnnotation {
                  target_symbol_qualified_name: target_symbol_qualified_name.to_string(),
                  annotation_name: name,
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
    }
  }
}

fn handle_call(node: Node<'_>, source: &str, current_symbol: &str, calls: &mut Vec<ExtractedCall>) {
  if let Some(target_node) = node.child(0) {
    let start = node.start_position();
    let end = node.end_position();

    if target_node.kind() == "navigation_expression" {
      if let Ok(full_target) = target_node.utf8_text(source.as_bytes()) {
        let full_target = full_target.trim().to_string();
        let method_name = if let Some(last_child) = target_node.child(target_node.child_count() - 1)
        {
          if let Ok(name) = last_child.utf8_text(source.as_bytes()) {
            Some(name.trim().to_string())
          } else {
            None
          }
        } else {
          None
        };

        let is_self_call = full_target.starts_with("this.");
        let target_name = method_name.clone().unwrap_or(full_target);

        calls.push(ExtractedCall {
          source_symbol_qualified_name: current_symbol.to_string(),
          target_name,
          is_self_call,
          method_name,
          start_line: start.row + 1,
          start_column: start.column,
          end_line: end.row + 1,
          end_column: end.column,
        });
      }
    } else {
      if let Ok(func_name) = target_node.utf8_text(source.as_bytes()) {
        let func_name = func_name.trim().to_string();
        if !func_name.is_empty() {
          // Heuristic: uppercase first char = class/constructor (not self call)
          let is_self_call = if let Some(first_char) = func_name.chars().next() {
            !first_char.is_uppercase()
          } else {
            true
          };

          calls.push(ExtractedCall {
            source_symbol_qualified_name: current_symbol.to_string(),
            target_name: func_name,
            is_self_call,
            method_name: None,
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

fn extract_flow_helper_calls_from_function(
  node: Node<'_>,
  source: &str,
  current_symbol: &str,
) -> Vec<ExtractedCall> {
  if !source_imports_flow_api(source) {
    return Vec::new();
  }
  let Ok(text) = node.utf8_text(source.as_bytes()) else {
    return Vec::new();
  };
  if !has_supported_flow_producer_marker(text) && !has_supported_flow_consumer_marker(text) {
    return Vec::new();
  }

  let current_name = extract_simple_function_name(current_symbol);
  let start = node.start_position();
  let end = node.end_position();

  extract_call_names_from_text(text)
    .into_iter()
    .filter(|name| !is_builder_call(name) && name != &current_name)
    .collect::<HashSet<_>>()
    .into_iter()
    .map(|target_name| ExtractedCall {
      source_symbol_qualified_name: current_symbol.to_string(),
      target_name,
      is_self_call: false,
      method_name: None,
      start_line: start.row + 1,
      start_column: start.column,
      end_line: end.row + 1,
      end_column: end.column,
    })
    .collect()
}

#[allow(clippy::too_many_arguments)]
fn traverse_relations(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, SymbolKind)>,
  calls: &mut Vec<ExtractedCall>,
  type_refs: &mut Vec<ExtractedTypeReference>,
  inheritance: &mut Vec<ExtractedInheritance>,
  annotations: &mut Vec<ExtractedAnnotation>,
) {
  let kind = node.kind();
  let mut scope_pushed = false;

  match kind {
    "class_declaration" => {
      if let Some(name_node) = find_name_child(node) {
        if let Ok(class_name) = name_node.utf8_text(source.as_bytes()) {
          let class_name = class_name.trim().to_string();
          if !class_name.is_empty() {
            let is_interface = {
              let mut cursor = node.walk();
              node.children(&mut cursor).any(|c| c.kind() == "interface")
            };
            let is_enum = {
              let mut cursor = node.walk();
              node.children(&mut cursor).any(|c| {
                c.kind() == "enum"
                  || (c.kind() == "modifiers"
                    && c
                      .utf8_text(source.as_bytes())
                      .is_ok_and(|t| t.contains("enum")))
              })
            };

            let symbol_kind = if is_enum {
              SymbolKind::Enum
            } else if is_interface {
              SymbolKind::Interface
            } else {
              SymbolKind::Class
            };

            scope_stack.push((class_name, symbol_kind));
            scope_pushed = true;

            extract_delegation_specifiers(node, source, scope_stack, inheritance);

            let qname = current_qualified_name(module_name, scope_stack);
            extract_node_annotations(node, source, &qname, annotations);
          }
        }
      }
    }
    "object_declaration" => {
      if let Some(name_node) = find_name_child(node) {
        if let Ok(obj_name) = name_node.utf8_text(source.as_bytes()) {
          let obj_name = obj_name.trim().to_string();
          if !obj_name.is_empty() {
            scope_stack.push((obj_name, SymbolKind::Class));
            scope_pushed = true;

            extract_delegation_specifiers(node, source, scope_stack, inheritance);

            let qname = current_qualified_name(module_name, scope_stack);
            extract_node_annotations(node, source, &qname, annotations);
          }
        }
      }
    }
    "companion_object" => {
      let companion_name = if let Some(name_node) = find_name_child(node) {
        if let Ok(cname) = name_node.utf8_text(source.as_bytes()) {
          let trimmed = cname.trim().to_string();
          if trimmed.is_empty() {
            "Companion".to_string()
          } else {
            trimmed
          }
        } else {
          "Companion".to_string()
        }
      } else {
        "Companion".to_string()
      };

      scope_stack.push((companion_name, SymbolKind::Class));
      scope_pushed = true;

      extract_delegation_specifiers(node, source, scope_stack, inheritance);

      let qname = current_qualified_name(module_name, scope_stack);
      extract_node_annotations(node, source, &qname, annotations);
    }
    "function_declaration" => {
      if let Some(name_node) = find_name_child(node) {
        if let Ok(func_name) = name_node.utf8_text(source.as_bytes()) {
          let func_name = func_name.trim().to_string();
          if !func_name.is_empty() {
            let is_method = scope_stack
              .last()
              .map(|(_, kind)| {
                *kind == SymbolKind::Class
                  || *kind == SymbolKind::Interface
                  || *kind == SymbolKind::Enum
              })
              .unwrap_or(false);

            let symbol_kind = if is_method {
              SymbolKind::Method
            } else {
              SymbolKind::Function
            };

            let receiver = extract_receiver_type(node, source);
            let name_to_push = if let Some(rx) = receiver {
              format!("{rx}.{func_name}")
            } else {
              func_name
            };

            scope_stack.push((name_to_push, symbol_kind));
            scope_pushed = true;

            let qname = current_qualified_name(module_name, scope_stack);
            extract_node_annotations(node, source, &qname, annotations);
            calls.extend(extract_flow_helper_calls_from_function(
              node, source, &qname,
            ));
          }
        }
      }
    }
    "property_declaration" => {
      if let Some(prop_name) = find_property_name(node, source) {
        let receiver = extract_receiver_type(node, source);
        let name_to_push = if let Some(rx) = receiver {
          format!("{rx}.{prop_name}")
        } else {
          prop_name
        };

        scope_stack.push((name_to_push, SymbolKind::Unknown));
        scope_pushed = true;

        let qname = current_qualified_name(module_name, scope_stack);
        extract_node_annotations(node, source, &qname, annotations);
      }
    }
    "type_alias" => {
      if let Some(name_node) = find_name_child(node) {
        if let Ok(alias_name) = name_node.utf8_text(source.as_bytes()) {
          let alias_name = alias_name.trim().to_string();
          if !alias_name.is_empty() {
            scope_stack.push((alias_name.clone(), SymbolKind::TypeAlias));
            scope_pushed = true;

            let qname = current_qualified_name(module_name, scope_stack);
            extract_node_annotations(node, source, &qname, annotations);
          }
        }
      }
    }
    "call_expression" => {
      let current_symbol = current_qualified_name(module_name, scope_stack);
      handle_call(node, source, &current_symbol, calls);
    }
    "navigation_expression" => {
      if let Ok(text) = node.utf8_text(source.as_bytes()) {
        let text = text.trim();
        let parts: Vec<&str> = text.split('.').collect();
        let mut r_idx = None;
        for (idx, &part) in parts.iter().enumerate() {
          let part_trimmed = part.trim();
          if part_trimmed == "R" && idx + 2 < parts.len() {
            let res_type = parts[idx + 1].trim();
            if matches!(
              res_type,
              "layout" | "id" | "string" | "drawable" | "color" | "dimen"
            ) {
              r_idx = Some(idx);
              break;
            }
          }
        }

        if let Some(idx) = r_idx {
          if idx + 2 == parts.len() - 1 {
            let res_type = parts[idx + 1].trim();
            let res_name = parts[idx + 2].trim();
            let current_symbol = current_qualified_name(module_name, scope_stack);
            let start = node.start_position();
            let end = node.end_position();
            type_refs.push(ExtractedTypeReference {
              source_symbol_qualified_name: current_symbol,
              target_type_name: format!("R.{}.{}", res_type, res_name),
              start_line: start.row + 1,
              start_column: start.column,
              end_line: end.row + 1,
              end_column: end.column,
            });
          }
        }
      }
    }
    "user_type" if !is_inside_import_or_package(node) => {
      if let Ok(text) = node.utf8_text(source.as_bytes()) {
        let type_name = clean_type_name(text);
        if !type_name.is_empty() && !KOTLIN_BUILTINS.contains(&type_name.as_str()) {
          let current_symbol = current_qualified_name(module_name, scope_stack);
          let start = node.start_position();
          let end = node.end_position();
          type_refs.push(ExtractedTypeReference {
            source_symbol_qualified_name: current_symbol,
            target_type_name: type_name,
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
    traverse_relations(
      child,
      source,
      module_name,
      scope_stack,
      calls,
      type_refs,
      inheritance,
      annotations,
    );
  }

  if scope_pushed {
    scope_stack.pop();
  }
}

fn traverse_imports(node: Node<'_>, source: &str, imports: &mut Vec<ExtractedImport>) {
  if node.kind() == "import" {
    if let Ok(text) = node.utf8_text(source.as_bytes()) {
      let trimmed = text.trim();
      if let Some(stripped) = trimmed.strip_prefix("import") {
        let clean = stripped.trim().trim_end_matches(';').trim();
        let parts: Vec<&str> = clean.split_whitespace().collect();
        if !parts.is_empty() {
          let target = parts[0].to_string();
          let alias_name = if parts.len() >= 3 && parts[1] == "as" {
            Some(parts[2].to_string())
          } else {
            None
          };
          let start = node.start_position();
          let end = node.end_position();
          imports.push(ExtractedImport {
            target_qualified_name: target,
            alias_name,
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
    traverse_imports(child, source, imports);
  }
}

#[allow(clippy::type_complexity)]
fn run_traverse_relations(
  source: &str,
  file_path: &str,
) -> Result<
  (
    Vec<ExtractedCall>,
    Vec<ExtractedTypeReference>,
    Vec<ExtractedInheritance>,
    Vec<ExtractedAnnotation>,
  ),
  KotlinParseError,
> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let module_name = if let Some(pkg) = extract_package_name(root, source) {
    pkg
  } else {
    module_name_from_path(file_path)
  };

  let mut scope_stack = Vec::new();
  let mut calls = Vec::new();
  let mut type_refs = Vec::new();
  let mut inheritance = Vec::new();
  let mut annotations = Vec::new();

  traverse_relations(
    root,
    source,
    &module_name,
    &mut scope_stack,
    &mut calls,
    &mut type_refs,
    &mut inheritance,
    &mut annotations,
  );

  let mut seen_calls = HashSet::new();
  calls.retain(|call| {
    seen_calls.insert((
      call.source_symbol_qualified_name.clone(),
      call.target_name.clone(),
      call.method_name.clone(),
      call.start_line,
      call.end_line,
    ))
  });

  Ok((calls, type_refs, inheritance, annotations))
}

pub fn extract_imports_from_source(
  source: &str,
  _file_path: &str,
) -> Result<Vec<ExtractedImport>, KotlinParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();
  let mut imports = Vec::new();
  traverse_imports(root, source, &mut imports);
  Ok(imports)
}

pub fn extract_calls_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedCall>, KotlinParseError> {
  let (calls, _, _, _) = run_traverse_relations(source, file_path)?;
  Ok(calls)
}

pub fn extract_type_references_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedTypeReference>, KotlinParseError> {
  let (_, type_refs, _, _) = run_traverse_relations(source, file_path)?;
  Ok(type_refs)
}

pub fn extract_inheritance_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedInheritance>, KotlinParseError> {
  let (_, _, inheritance, _) = run_traverse_relations(source, file_path)?;
  Ok(inheritance)
}

pub fn extract_annotations_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedAnnotation>, KotlinParseError> {
  let (_, _, _, annotations) = run_traverse_relations(source, file_path)?;
  Ok(annotations)
}

fn parse_to_tree(source: &str) -> Result<tree_sitter::Tree, KotlinParseError> {
  let mut parser = Parser::new();
  parser
    .set_language(&tree_sitter_kotlin_ng::LANGUAGE.into())
    .map_err(|e| KotlinParseError::UnsupportedLanguage(e.to_string()))?;
  parser
    .parse(source, None)
    .ok_or(KotlinParseError::ParseCancelled)
}

fn module_name_from_path(file_path: &str) -> String {
  let path = Path::new(file_path);
  let mut components = Vec::new();
  for component in path.components() {
    let comp_str = component.as_os_str().to_string_lossy();
    if comp_str != "." && comp_str != ".." {
      components.push(comp_str.to_string());
    }
  }

  let normalized_path = components.join("/");
  let path_without_ext = normalized_path
    .strip_suffix(".kt")
    .or_else(|| normalized_path.strip_suffix(".kts"))
    .unwrap_or(&normalized_path);

  path_without_ext.replace(['/', '\\'], ".")
}

fn extract_package_name(root: Node<'_>, source: &str) -> Option<String> {
  let mut cursor = root.walk();
  for child in root.children(&mut cursor) {
    if child.kind() == "package_header" {
      // Attempt to find clean identifier or user_type child
      let mut sub_cursor = child.walk();
      for sub_child in child.children(&mut sub_cursor) {
        if sub_child.kind() == "user_type"
          || sub_child.kind() == "identifier"
          || sub_child.kind() == "simple_identifier"
        {
          if let Ok(text) = sub_child.utf8_text(source.as_bytes()) {
            let name = text.trim().to_string();
            if !name.is_empty() {
              return Some(name);
            }
          }
        }
      }
      // Fallback to string slicing
      if let Ok(text) = child.utf8_text(source.as_bytes()) {
        let trimmed = text.trim();
        if let Some(stripped) = trimmed.strip_prefix("package") {
          let name = stripped.trim().trim_end_matches(';').trim().to_string();
          if !name.is_empty() {
            return Some(name);
          }
        }
      }
    }
  }
  None
}

fn find_name_child(node: Node<'_>) -> Option<Node<'_>> {
  if let Some(name) = node.child_by_field_name("name") {
    return Some(name);
  }
  let mut cursor = node.walk();
  node.children(&mut cursor).find(|child| {
    child.kind() == "type_identifier"
      || child.kind() == "simple_identifier"
      || child.kind() == "identifier"
  })
}

fn find_property_name(node: Node<'_>, source: &str) -> Option<String> {
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if child.kind() == "variable_declaration" {
      if let Some(name_child) = find_name_child(child) {
        if let Ok(t) = name_child.utf8_text(source.as_bytes()) {
          return Some(t.trim().to_string());
        }
      }
    }
    if child.kind() == "simple_identifier" || child.kind() == "identifier" {
      if let Ok(t) = child.utf8_text(source.as_bytes()) {
        return Some(t.trim().to_string());
      }
    }
  }
  None
}

fn extract_receiver_type(node: Node<'_>, source: &str) -> Option<String> {
  let mut receiver_node = None;
  let mut has_dot = false;
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if child.kind() == "." {
      has_dot = true;
      break;
    }
    if child.kind() == "user_type"
      || child.kind() == "nullable_type"
      || child.kind() == "parenthesized_type"
      || child.kind().contains("type")
    {
      receiver_node = Some(child);
    }
  }

  if has_dot {
    if let Some(rx) = receiver_node {
      if let Ok(rx_text) = rx.utf8_text(source.as_bytes()) {
        let clean_rx = clean_type_name(rx_text.trim());
        if !clean_rx.is_empty() {
          return Some(clean_rx);
        }
      }
    }
  }
  None
}

fn build_scope_path(scope_stack: &[(String, SymbolKind)]) -> String {
  let mut path = String::new();
  for (i, (n, _)) in scope_stack.iter().enumerate() {
    if i > 0 {
      path.push('.');
    }
    path.push_str(n);
  }
  path
}

pub fn extract_symbols_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<Symbol>, KotlinParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let line_count = source.lines().count().max(1);
  let mut symbols = Vec::new();

  // Determine base module name
  let module_name = if let Some(pkg) = extract_package_name(root, source) {
    pkg
  } else {
    module_name_from_path(file_path)
  };

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

fn traverse_symbols(
  node: Node<'_>,
  source: &str,
  file_path: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, SymbolKind)>,
  symbols: &mut Vec<Symbol>,
) {
  let kind = node.kind();

  // Performance & Correctness Optimization: Do not descend into function bodies, constructor bodies, or initializer blocks.
  // This completely eliminates local variable/function pollution and avoids redundant AST walks.
  if kind == "function_body" || kind == "initializer_block" || kind == "constructor_body" {
    return;
  }

  let mut scope_pushed = false;

  match kind {
    "class_declaration" => {
      if let Some(name_node) = find_name_child(node) {
        if let Ok(class_name) = name_node.utf8_text(source.as_bytes()) {
          let class_name = class_name.trim().to_string();
          if !class_name.is_empty() {
            let qname = if scope_stack.is_empty() {
              format!("{}::{}", module_name, class_name)
            } else {
              format!(
                "{}::{}.{}",
                module_name,
                build_scope_path(scope_stack),
                class_name
              )
            };

            let start = node.start_position();
            let end = node.end_position();

            let is_interface = {
              let mut cursor = node.walk();
              node.children(&mut cursor).any(|c| c.kind() == "interface")
            };
            let is_enum = {
              let mut cursor = node.walk();
              node.children(&mut cursor).any(|c| {
                c.kind() == "enum"
                  || (c.kind() == "modifiers"
                    && c
                      .utf8_text(source.as_bytes())
                      .is_ok_and(|t| t.contains("enum")))
              })
            };

            let symbol_kind = if is_enum {
              SymbolKind::Enum
            } else if is_interface {
              SymbolKind::Interface
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
            scope_pushed = true;
          }
        }
      }
    }
    "object_declaration" => {
      if let Some(name_node) = find_name_child(node) {
        if let Ok(obj_name) = name_node.utf8_text(source.as_bytes()) {
          let obj_name = obj_name.trim().to_string();
          if !obj_name.is_empty() {
            let qname = if scope_stack.is_empty() {
              format!("{}::{}", module_name, obj_name)
            } else {
              format!(
                "{}::{}.{}",
                module_name,
                build_scope_path(scope_stack),
                obj_name
              )
            };

            let start = node.start_position();
            let end = node.end_position();

            symbols.push(Symbol {
              name: obj_name.clone(),
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

            scope_stack.push((obj_name, SymbolKind::Class));
            scope_pushed = true;
          }
        }
      }
    }
    "companion_object" => {
      let companion_name = if let Some(name_node) = find_name_child(node) {
        if let Ok(cname) = name_node.utf8_text(source.as_bytes()) {
          let trimmed = cname.trim().to_string();
          if trimmed.is_empty() {
            "Companion".to_string()
          } else {
            trimmed
          }
        } else {
          "Companion".to_string()
        }
      } else {
        "Companion".to_string()
      };

      let qname = if scope_stack.is_empty() {
        format!("{}::{}", module_name, companion_name)
      } else {
        format!(
          "{}::{}.{}",
          module_name,
          build_scope_path(scope_stack),
          companion_name
        )
      };

      let start = node.start_position();
      let end = node.end_position();

      symbols.push(Symbol {
        name: companion_name.clone(),
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

      scope_stack.push((companion_name, SymbolKind::Class));
      scope_pushed = true;
    }
    "function_declaration" => {
      if let Some(name_node) = find_name_child(node) {
        if let Ok(func_name) = name_node.utf8_text(source.as_bytes()) {
          let func_name = func_name.trim().to_string();
          if !func_name.is_empty() {
            let is_method = scope_stack
              .last()
              .map(|(_, kind)| {
                *kind == SymbolKind::Class
                  || *kind == SymbolKind::Interface
                  || *kind == SymbolKind::Enum
              })
              .unwrap_or(false);

            let symbol_kind = if is_method {
              SymbolKind::Method
            } else {
              SymbolKind::Function
            };

            // Parse extension receiver type
            let receiver = extract_receiver_type(node, source);

            let qname = if scope_stack.is_empty() {
              if let Some(ref rx) = receiver {
                format!("{}::{}.{}", module_name, rx, func_name)
              } else {
                format!("{}::{}", module_name, func_name)
              }
            } else {
              let scope_path = build_scope_path(scope_stack);
              if let Some(ref rx) = receiver {
                format!("{}::{}.{}.{}", module_name, scope_path, rx, func_name)
              } else {
                format!("{}::{}.{}", module_name, scope_path, func_name)
              }
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
    }
    "property_declaration" => {
      if let Some(prop_name) = find_property_name(node, source) {
        // Parse extension receiver type for properties
        let receiver = extract_receiver_type(node, source);

        let qname = if scope_stack.is_empty() {
          if let Some(ref rx) = receiver {
            format!("{}::{}.{}", module_name, rx, prop_name)
          } else {
            format!("{}::{}", module_name, prop_name)
          }
        } else {
          let scope_path = build_scope_path(scope_stack);
          if let Some(ref rx) = receiver {
            format!("{}::{}.{}.{}", module_name, scope_path, rx, prop_name)
          } else {
            format!("{}::{}.{}", module_name, scope_path, prop_name)
          }
        };

        let start = node.start_position();
        let end = node.end_position();

        symbols.push(Symbol {
          name: prop_name.clone(),
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
    "type_alias" => {
      if let Some(name_node) = find_name_child(node) {
        if let Ok(alias_name) = name_node.utf8_text(source.as_bytes()) {
          let alias_name = alias_name.trim().to_string();
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
              name: alias_name.clone(),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedKotlinRoute {
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
pub struct ExtractedChannelUsage {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedConcurrencyEdge {
  pub source_symbol_qualified_name: String,
  pub target_name: String,
  pub kind: String,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}

pub type ExtractedConcurrencyReport = (
  Vec<ExtractedConcurrencyEdge>,
  Vec<ExtractedConcurrencySpawn>,
  Vec<ExtractedChannelUsage>,
  Vec<ExtractedSelectBlock>,
);

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChannelOperation {
  source_symbol_qualified_name: String,
  channel_name: String,
  start_line: usize,
  start_column: usize,
  end_line: usize,
  end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FunctionChannelSignature {
  qualified_name: String,
  parameter_names: Vec<String>,
  channel_parameters: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SpawnTargetCall {
  target_name: String,
  argument_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FlowDerivedChannel {
  source_symbol_qualified_name: String,
  channel_name: String,
  upstream_flow_references: Vec<String>,
  start_line: usize,
  start_column: usize,
  end_line: usize,
  end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FlowConsumerEdge {
  upstream_flow_reference: String,
  target_symbol_qualified_name: String,
  start_line: usize,
  start_column: usize,
  end_line: usize,
  end_column: usize,
}

fn extract_navigation_receiver(call_node: Node<'_>, source: &str) -> Option<String> {
  let target_node = call_node.child(0)?;
  if target_node.kind() != "navigation_expression" {
    return None;
  }

  let text = target_node
    .utf8_text(source.as_bytes())
    .ok()?
    .trim()
    .to_string();
  let last_dot = text.rfind('.')?;
  let receiver = text[..last_dot].trim();
  if receiver.is_empty() {
    None
  } else {
    Some(receiver.to_string())
  }
}

fn is_builder_call(name: &str) -> bool {
  matches!(
    name,
    "launch"
      | "async"
      | "select"
      | "println"
      | "send"
      | "trySend"
      | "receive"
      | "receiveCatching"
      | "onReceive"
      | "onReceiveCatching"
      | "Channel"
      | "flow"
      | "channelFlow"
      | "callbackFlow"
      | "map"
      | "flatMapLatest"
      | "catch"
      | "collect"
      | "collectLatest"
      | "collectIndexed"
      | "launchIn"
      | "combine"
      | "zip"
      | "shareIn"
      | "stateIn"
      | "emit"
  )
}

fn extract_simple_function_name(qualified_name: &str) -> String {
  qualified_name
    .rsplit([':', '.'])
    .next()
    .unwrap_or(qualified_name)
    .to_string()
}

fn source_imports_flow_api(source: &str) -> bool {
  source.contains("kotlinx.coroutines.flow")
}

fn has_supported_flow_producer_marker(text: &str) -> bool {
  text.contains("flow {")
    || text.contains("flow{")
    || text.contains("channelFlow")
    || text.contains("callbackFlow")
    || text.contains(".map")
    || text.contains(".flatMapLatest")
    || text.contains(".catch")
    || text.contains(".shareIn")
    || text.contains(".stateIn")
    || text.contains(".combine")
    || text.contains(".zip")
    || text.contains("combine(")
    || text.contains("zip(")
}

fn has_supported_flow_consumer_marker(text: &str) -> bool {
  text.contains(".collect")
    || text.contains(".collectLatest")
    || text.contains(".collectIndexed")
    || text.contains(".launchIn")
}

fn extract_call_names_from_text(text: &str) -> Vec<String> {
  let chars: Vec<char> = text.chars().collect();
  let mut names = Vec::new();
  let mut idx = 0;

  while idx < chars.len() {
    if chars[idx].is_ascii_alphabetic() || chars[idx] == '_' {
      let start = idx;
      idx += 1;
      while idx < chars.len()
        && (chars[idx].is_ascii_alphanumeric() || chars[idx] == '_' || chars[idx] == '.')
      {
        idx += 1;
      }

      let end = idx;
      let mut lookahead = idx;
      while lookahead < chars.len() && chars[lookahead].is_whitespace() {
        lookahead += 1;
      }

      if lookahead < chars.len() && chars[lookahead] == '(' {
        let token: String = chars[start..end].iter().collect();
        let name = token
          .split('.')
          .next_back()
          .unwrap_or(token.as_str())
          .trim()
          .to_string();
        if !name.is_empty() {
          names.push(name);
        }
      }
      idx = end;
    } else {
      idx += 1;
    }
  }

  names
}

fn extract_expression_after_equals(text: &str) -> Option<String> {
  let (_, expression) = text.split_once('=')?;
  let expression = expression.trim();
  if expression.is_empty() {
    None
  } else {
    Some(expression.to_string())
  }
}

fn find_matching_closing_paren(text: &str) -> Option<usize> {
  let mut depth = 1;
  let mut in_string = false;
  let mut quote_char = ' ';
  for (idx, c) in text.char_indices() {
    if in_string {
      if c == quote_char {
        in_string = false;
      }
    } else if c == '"' || c == '\'' {
      in_string = true;
      quote_char = c;
    } else if c == '(' {
      depth += 1;
    } else if c == ')' {
      depth -= 1;
      if depth == 0 {
        return Some(idx);
      }
    }
  }
  None
}

fn split_top_level_arguments(text: &str) -> Vec<String> {
  let mut args = Vec::new();
  let mut current = String::new();
  let mut paren_depth: usize = 0;
  let mut bracket_depth: usize = 0;
  let mut brace_depth: usize = 0;
  let mut in_string = false;
  let mut quote_char = ' ';
  for c in text.chars() {
    if in_string {
      current.push(c);
      if c == quote_char {
        in_string = false;
      }
    } else if c == '"' || c == '\'' {
      in_string = true;
      quote_char = c;
      current.push(c);
    } else if c == '(' {
      paren_depth += 1;
      current.push(c);
    } else if c == ')' {
      paren_depth = paren_depth.saturating_sub(1);
      current.push(c);
    } else if c == '[' {
      bracket_depth += 1;
      current.push(c);
    } else if c == ']' {
      bracket_depth = bracket_depth.saturating_sub(1);
      current.push(c);
    } else if c == '{' {
      brace_depth += 1;
      current.push(c);
    } else if c == '}' {
      brace_depth = brace_depth.saturating_sub(1);
      current.push(c);
    } else if c == ',' && paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
      args.push(current.trim().to_string());
      current.clear();
    } else {
      current.push(c);
    }
  }
  let final_arg = current.trim();
  if !final_arg.is_empty() {
    args.push(final_arg.to_string());
  }
  args
}

fn extract_reference_from_expression_prefix(prefix: &str) -> Option<String> {
  let trimmed = prefix.trim();
  if trimmed.is_empty() {
    return None;
  }
  if trimmed.contains('(') || trimmed.contains('{') {
    let mut end = trimmed.len();
    for (idx, c) in trimmed.char_indices() {
      if c == '.' || c == '(' || c == '{' || c.is_whitespace() {
        end = idx;
        break;
      }
    }
    let base = trimmed[..end].trim().to_string();
    if !base.is_empty() { Some(base) } else { None }
  } else {
    trimmed
      .split('.')
      .next_back()
      .map(str::trim)
      .filter(|v| !v.is_empty())
      .map(ToString::to_string)
  }
}

fn extract_flow_upstream_references_from_expression(expression: &str) -> Vec<String> {
  let trimmed = expression.trim();
  let mut upstreams = Vec::new();

  // 1. Check for combine/zip operators or calls
  if trimmed.contains(".combine(") || trimmed.contains(".zip(") {
    if let Some(idx) = trimmed.find(".combine(").or_else(|| trimmed.find(".zip(")) {
      let prefix = trimmed[..idx].trim();
      if let Some(ref_prefix) = extract_reference_from_expression_prefix(prefix) {
        upstreams.push(ref_prefix);
      }
      let start_args = idx + ".combine(".len();
      let args_slice = &trimmed[start_args..];
      if let Some(end_args) = find_matching_closing_paren(args_slice) {
        let args_str = &args_slice[..end_args];
        let args = split_top_level_arguments(args_str);
        for arg in args {
          let arg_cleaned = arg.trim();
          if let Some(ref_arg) = extract_reference_from_expression_prefix(arg_cleaned) {
            upstreams.push(ref_arg);
          }
        }
      }
    }
  } else if trimmed.starts_with("combine(") || trimmed.starts_with("zip(") {
    let start_args = if trimmed.starts_with("combine(") {
      "combine(".len()
    } else {
      "zip(".len()
    };
    let args_slice = &trimmed[start_args..];
    if let Some(end_args) = find_matching_closing_paren(args_slice) {
      let args_str = &args_slice[..end_args];
      let args = split_top_level_arguments(args_str);
      for arg in args {
        let arg_cleaned = arg.trim();
        if let Some(ref_arg) = extract_reference_from_expression_prefix(arg_cleaned) {
          upstreams.push(ref_arg);
        }
      }
    }
  } else {
    // 2. Fallback to standard single upstream operators
    if trimmed.starts_with("flow")
      || trimmed.starts_with("channelFlow")
      || trimmed.starts_with("callbackFlow")
    {
      return upstreams;
    }

    for operator in ["map", "flatMapLatest", "catch", "shareIn", "stateIn"] {
      let marker = format!(".{operator}");
      if let Some(idx) = trimmed.find(&marker) {
        let prefix = trimmed[..idx].trim();
        if let Some(ref_prefix) = extract_reference_from_expression_prefix(prefix) {
          upstreams.push(ref_prefix);
        }
        break;
      }
    }
  }

  upstreams.retain(|name| !name.is_empty());
  upstreams
}

fn extract_collect_receiver_reference(call_node: Node<'_>, source: &str) -> Option<String> {
  let receiver = extract_navigation_receiver(call_node, source)?;
  extract_call_names_from_text(&receiver)
    .into_iter()
    .next()
    .or_else(|| {
      receiver
        .split('.')
        .next_back()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
    })
}

fn extract_flow_channel_from_property(
  node: Node<'_>,
  source: &str,
  current_symbol: &str,
) -> Option<FlowDerivedChannel> {
  if !source_imports_flow_api(source) {
    return None;
  }
  let prop_name = find_property_name(node, source)?;
  let text = node.utf8_text(source.as_bytes()).ok()?;
  let expression = extract_expression_after_equals(text)?;
  if !has_supported_flow_producer_marker(&expression) {
    return None;
  }

  let start = node.start_position();
  let end = node.end_position();
  Some(FlowDerivedChannel {
    source_symbol_qualified_name: current_symbol.to_string(),
    channel_name: prop_name,
    upstream_flow_references: extract_flow_upstream_references_from_expression(&expression),
    start_line: start.row + 1,
    start_column: start.column,
    end_line: end.row + 1,
    end_column: end.column,
  })
}

fn extract_flow_channel_from_function(
  node: Node<'_>,
  source: &str,
  current_symbol: &str,
) -> Option<FlowDerivedChannel> {
  if !source_imports_flow_api(source) {
    return None;
  }
  let text = node.utf8_text(source.as_bytes()).ok()?;
  let expression = extract_expression_after_equals(text)?;
  if !has_supported_flow_producer_marker(&expression) {
    return None;
  }

  let start = node.start_position();
  let end = node.end_position();
  Some(FlowDerivedChannel {
    source_symbol_qualified_name: current_symbol.to_string(),
    channel_name: extract_simple_function_name(current_symbol),
    upstream_flow_references: extract_flow_upstream_references_from_expression(&expression),
    start_line: start.row + 1,
    start_column: start.column,
    end_line: end.row + 1,
    end_column: end.column,
  })
}

fn extract_parameter_name(parameter: &str) -> Option<String> {
  let before_type = parameter.split(':').next()?.trim();
  let token = before_type
    .split_whitespace()
    .last()?
    .trim()
    .trim_matches(',');
  if token.is_empty() {
    None
  } else {
    Some(token.to_string())
  }
}

fn extract_function_channel_signature(
  node: Node<'_>,
  source: &str,
  qualified_name: String,
) -> Option<FunctionChannelSignature> {
  let text = node.utf8_text(source.as_bytes()).ok()?;
  let start = text.find('(')?;
  let end = text[start..].find(')')? + start;
  let params_text = &text[start + 1..end];
  let mut parameter_names = Vec::new();
  let mut channel_parameters = HashSet::new();

  for raw_parameter in params_text.split(',') {
    let parameter = raw_parameter.trim();
    if parameter.is_empty() {
      continue;
    }
    let Some(parameter_name) = extract_parameter_name(parameter) else {
      continue;
    };
    if parameter.contains("Channel<") || parameter.contains("Channel(") {
      channel_parameters.insert(parameter_name.clone());
    }
    parameter_names.push(parameter_name);
  }

  Some(FunctionChannelSignature {
    qualified_name,
    parameter_names,
    channel_parameters,
  })
}

fn extract_simple_argument_names(call_node: Node<'_>, source: &str) -> Vec<String> {
  let Ok(text) = call_node.utf8_text(source.as_bytes()) else {
    return Vec::new();
  };
  let Some(start) = text.find('(') else {
    return Vec::new();
  };
  let Some(end) = text.rfind(')') else {
    return Vec::new();
  };
  if end <= start {
    return Vec::new();
  }

  text[start + 1..end]
    .split(',')
    .filter_map(|argument| {
      let value = argument
        .split('=')
        .next_back()?
        .trim()
        .trim_matches('{')
        .trim_matches('}')
        .trim();
      if value.is_empty()
        || value.contains('(')
        || value.contains(')')
        || value.contains('{')
        || value.contains('}')
        || value.contains(' ')
      {
        None
      } else {
        Some(value.to_string())
      }
    })
    .collect()
}

fn collect_spawn_target_calls(node: Node<'_>, source: &str, calls: &mut Vec<SpawnTargetCall>) {
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if child.kind() == "call_expression" {
      if let Some(callee) = find_callee_name(child, source) {
        let callee = callee.trim().to_string();
        if !is_builder_call(&callee) {
          calls.push(SpawnTargetCall {
            target_name: callee,
            argument_names: extract_simple_argument_names(child, source),
          });
        }
      }
    }
    collect_spawn_target_calls(child, source, calls);
  }
}

fn find_spawn_target_call(node: Node<'_>, source: &str) -> Option<SpawnTargetCall> {
  let mut calls = Vec::new();
  collect_spawn_target_calls(node, source, &mut calls);
  calls.pop()
}

#[allow(clippy::too_many_arguments)]
fn handle_concurrency_call(
  node: Node<'_>,
  source: &str,
  current_symbol: &str,
  spawns: &mut Vec<ExtractedConcurrencySpawn>,
  spawn_bindings: &mut Vec<(String, SpawnTargetCall)>,
  selects: &mut Vec<ExtractedSelectBlock>,
  sends: &mut Vec<ChannelOperation>,
  receives: &mut Vec<ChannelOperation>,
  flow_consumers: &mut Vec<FlowConsumerEdge>,
) {
  let Some(callee) = find_callee_name(node, source) else {
    return;
  };
  let callee = callee.trim().to_string();
  let start = node.start_position();
  let end = node.end_position();

  if callee == "launch" || callee == "async" {
    let target_call = find_spawn_target_call(node, source);
    spawns.push(ExtractedConcurrencySpawn {
      source_symbol_qualified_name: current_symbol.to_string(),
      spawn_kind: callee,
      target_name: target_call.as_ref().map(|call| call.target_name.clone()),
      start_line: start.row + 1,
      start_column: start.column,
      end_line: end.row + 1,
      end_column: end.column,
    });
    if let Some(target_call) = target_call {
      spawn_bindings.push((current_symbol.to_string(), target_call));
    }
    return;
  }

  if (callee == "collect"
    || callee == "collectLatest"
    || callee == "collectIndexed"
    || callee == "launchIn")
    && source_imports_flow_api(source)
  {
    if let Some(upstream_flow_reference) = extract_collect_receiver_reference(node, source) {
      flow_consumers.push(FlowConsumerEdge {
        upstream_flow_reference,
        target_symbol_qualified_name: current_symbol.to_string(),
        start_line: start.row + 1,
        start_column: start.column,
        end_line: end.row + 1,
        end_column: end.column,
      });
    }
    return;
  }

  if callee == "select" || callee.starts_with("select<") || callee.contains("select") {
    return;
  }

  let Some(channel_name) = extract_navigation_receiver(node, source) else {
    return;
  };

  let operation = ChannelOperation {
    source_symbol_qualified_name: current_symbol.to_string(),
    channel_name,
    start_line: start.row + 1,
    start_column: start.column,
    end_line: end.row + 1,
    end_column: end.column,
  };

  if matches!(callee.as_str(), "send" | "trySend") {
    sends.push(operation);
  } else if matches!(
    callee.as_str(),
    "receive" | "receiveCatching" | "onReceive" | "onReceiveCatching"
  ) {
    if matches!(callee.as_str(), "onReceive" | "onReceiveCatching") {
      selects.push(ExtractedSelectBlock {
        source_symbol_qualified_name: current_symbol.to_string(),
        start_line: start.row + 1,
        start_column: start.column,
        end_line: end.row + 1,
        end_column: end.column,
      });
    }
    receives.push(operation);
  }
}

#[allow(clippy::too_many_arguments)]
fn traverse_concurrency(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, SymbolKind)>,
  spawns: &mut Vec<ExtractedConcurrencySpawn>,
  function_signatures: &mut Vec<FunctionChannelSignature>,
  spawn_bindings: &mut Vec<(String, SpawnTargetCall)>,
  channels: &mut Vec<ExtractedChannelUsage>,
  selects: &mut Vec<ExtractedSelectBlock>,
  sends: &mut Vec<ChannelOperation>,
  receives: &mut Vec<ChannelOperation>,
  flow_channels: &mut Vec<FlowDerivedChannel>,
  flow_consumers: &mut Vec<FlowConsumerEdge>,
) {
  let kind = node.kind();
  let mut scope_pushed = false;

  match kind {
    "class_declaration" => {
      if let Some(name_node) = find_name_child(node) {
        if let Ok(class_name) = name_node.utf8_text(source.as_bytes()) {
          let class_name = class_name.trim().to_string();
          if !class_name.is_empty() {
            let is_interface = {
              let mut cursor = node.walk();
              node.children(&mut cursor).any(|c| c.kind() == "interface")
            };
            let is_enum = {
              let mut cursor = node.walk();
              node.children(&mut cursor).any(|c| {
                c.kind() == "enum"
                  || (c.kind() == "modifiers"
                    && c
                      .utf8_text(source.as_bytes())
                      .is_ok_and(|t| t.contains("enum")))
              })
            };
            let symbol_kind = if is_enum {
              SymbolKind::Enum
            } else if is_interface {
              SymbolKind::Interface
            } else {
              SymbolKind::Class
            };
            scope_stack.push((class_name, symbol_kind));
            scope_pushed = true;
          }
        }
      }
    }
    "object_declaration" => {
      if let Some(name_node) = find_name_child(node) {
        if let Ok(obj_name) = name_node.utf8_text(source.as_bytes()) {
          let obj_name = obj_name.trim().to_string();
          if !obj_name.is_empty() {
            scope_stack.push((obj_name, SymbolKind::Class));
            scope_pushed = true;
          }
        }
      }
    }
    "companion_object" => {
      let companion_name = if let Some(name_node) = find_name_child(node) {
        if let Ok(cname) = name_node.utf8_text(source.as_bytes()) {
          let trimmed = cname.trim().to_string();
          if trimmed.is_empty() {
            "Companion".to_string()
          } else {
            trimmed
          }
        } else {
          "Companion".to_string()
        }
      } else {
        "Companion".to_string()
      };
      scope_stack.push((companion_name, SymbolKind::Class));
      scope_pushed = true;
    }
    "function_declaration" => {
      if let Some(name_node) = find_name_child(node) {
        if let Ok(func_name) = name_node.utf8_text(source.as_bytes()) {
          let func_name = func_name.trim().to_string();
          if !func_name.is_empty() {
            let is_method = scope_stack
              .last()
              .map(|(_, kind)| {
                *kind == SymbolKind::Class
                  || *kind == SymbolKind::Interface
                  || *kind == SymbolKind::Enum
              })
              .unwrap_or(false);
            let symbol_kind = if is_method {
              SymbolKind::Method
            } else {
              SymbolKind::Function
            };
            let receiver = extract_receiver_type(node, source);
            let name_to_push = if let Some(rx) = receiver {
              format!("{rx}.{func_name}")
            } else {
              func_name
            };
            scope_stack.push((name_to_push, symbol_kind));
            if let Some(signature) = extract_function_channel_signature(
              node,
              source,
              current_qualified_name(module_name, scope_stack),
            ) {
              function_signatures.push(signature);
            }
            let current_symbol = current_qualified_name(module_name, scope_stack);
            if let Some(flow_channel) =
              extract_flow_channel_from_function(node, source, &current_symbol)
            {
              flow_channels.push(flow_channel);
            }
            scope_pushed = true;
          }
        }
      }
    }
    "property_declaration" => {
      let current_symbol = current_qualified_name(module_name, scope_stack);
      if let Some(prop_name) = find_property_name(node, source) {
        let text = node.utf8_text(source.as_bytes()).unwrap_or("");
        if text.contains("Channel<") || text.contains("Channel(") {
          let start = node.start_position();
          let end = node.end_position();
          channels.push(ExtractedChannelUsage {
            source_symbol_qualified_name: current_symbol.clone(),
            channel_kind: "Channel".to_string(),
            tx_name: prop_name.clone(),
            rx_name: prop_name,
            start_line: start.row + 1,
            start_column: start.column,
            end_line: end.row + 1,
            end_column: end.column,
          });
        }
      }
      if let Some(flow_channel) = extract_flow_channel_from_property(node, source, &current_symbol)
      {
        flow_channels.push(flow_channel);
      }
    }
    "call_expression" => {
      let current_symbol = current_qualified_name(module_name, scope_stack);
      handle_concurrency_call(
        node,
        source,
        &current_symbol,
        spawns,
        spawn_bindings,
        selects,
        sends,
        receives,
        flow_consumers,
      );
    }
    _ => {}
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_concurrency(
      child,
      source,
      module_name,
      scope_stack,
      spawns,
      function_signatures,
      spawn_bindings,
      channels,
      selects,
      sends,
      receives,
      flow_channels,
      flow_consumers,
    );
  }

  if scope_pushed {
    scope_stack.pop();
  }
}

pub fn extract_concurrency_from_source(
  source: &str,
  file_path: &str,
) -> Result<ExtractedConcurrencyReport, KotlinParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();
  let module_name = if let Some(pkg) = extract_package_name(root, source) {
    pkg
  } else {
    module_name_from_path(file_path)
  };

  let mut scope_stack = Vec::new();
  let mut spawns = Vec::new();
  let mut function_signatures = Vec::new();
  let mut spawn_bindings = Vec::new();
  let mut channels = Vec::new();
  let mut selects = Vec::new();
  let mut sends = Vec::new();
  let mut receives = Vec::new();
  let mut flow_channels = Vec::new();
  let mut flow_consumers = Vec::new();

  traverse_concurrency(
    root,
    source,
    &module_name,
    &mut scope_stack,
    &mut spawns,
    &mut function_signatures,
    &mut spawn_bindings,
    &mut channels,
    &mut selects,
    &mut sends,
    &mut receives,
    &mut flow_channels,
    &mut flow_consumers,
  );

  let mut local_channel_identities: HashMap<(String, String), String> = HashMap::new();
  for channel in &channels {
    let identity = format!(
      "{}::channel::{}",
      channel.source_symbol_qualified_name, channel.tx_name
    );
    local_channel_identities.insert(
      (
        channel.source_symbol_qualified_name.clone(),
        channel.tx_name.clone(),
      ),
      identity,
    );
  }
  let mut simple_name_to_qnames: HashMap<String, Vec<String>> = HashMap::new();
  for signature in &function_signatures {
    let simple_name = signature
      .qualified_name
      .rsplit([':', '.'])
      .next()
      .unwrap_or(&signature.qualified_name)
      .to_string();
    simple_name_to_qnames
      .entry(simple_name)
      .or_default()
      .push(signature.qualified_name.clone());
  }

  let mut channel_bindings: HashMap<(String, String), String> = HashMap::new();
  for (source_symbol, target_call) in &spawn_bindings {
    let Some(target_candidates) = simple_name_to_qnames.get(&target_call.target_name) else {
      continue;
    };
    if target_candidates.len() != 1 {
      continue;
    }
    let target_qualified_name = &target_candidates[0];
    let Some(signature) = function_signatures
      .iter()
      .find(|sig| sig.qualified_name == *target_qualified_name)
    else {
      continue;
    };

    for (parameter_name, argument_name) in signature
      .parameter_names
      .iter()
      .zip(target_call.argument_names.iter())
    {
      if !signature.channel_parameters.contains(parameter_name) {
        continue;
      }
      let Some(identity) = local_channel_identities
        .get(&(source_symbol.clone(), argument_name.clone()))
        .or_else(|| channel_bindings.get(&(source_symbol.clone(), argument_name.clone())))
      else {
        continue;
      };
      channel_bindings.insert(
        (signature.qualified_name.clone(), parameter_name.clone()),
        identity.clone(),
      );
    }
  }

  let resolve_flow_identity = |channel: &FlowDerivedChannel,
                               local_channel_identities: &HashMap<(String, String), String>,
                               simple_name_to_qnames: &HashMap<String, Vec<String>>|
   -> Option<String> {
    if channel.upstream_flow_references.len() == 1 {
      let upstream_reference = &channel.upstream_flow_references[0];
      if let Some(identity) = local_channel_identities.get(&(
        channel.source_symbol_qualified_name.clone(),
        upstream_reference.clone(),
      )) {
        return Some(identity.clone());
      }

      if let Some(candidates) = simple_name_to_qnames.get(upstream_reference) {
        if candidates.len() == 1 {
          return Some(candidates[0].clone());
        }
      }
    }

    Some(format!(
      "{}::flow::{}",
      channel.source_symbol_qualified_name, channel.channel_name
    ))
  };

  for channel in &flow_channels {
    let Some(identity) =
      resolve_flow_identity(channel, &local_channel_identities, &simple_name_to_qnames)
    else {
      continue;
    };

    local_channel_identities.insert(
      (
        channel.source_symbol_qualified_name.clone(),
        channel.channel_name.clone(),
      ),
      identity.clone(),
    );
    channel_bindings.insert(
      (
        channel.source_symbol_qualified_name.clone(),
        channel.channel_name.clone(),
      ),
      identity,
    );
    channels.push(ExtractedChannelUsage {
      source_symbol_qualified_name: channel.source_symbol_qualified_name.clone(),
      channel_kind: "Flow".to_string(),
      tx_name: channel.channel_name.clone(),
      rx_name: channel.channel_name.clone(),
      start_line: channel.start_line,
      start_column: channel.start_column,
      end_line: channel.end_line,
      end_column: channel.end_column,
    });
  }

  let operation_channel_identity = |operation: &ChannelOperation| -> Option<String> {
    local_channel_identities
      .get(&(
        operation.source_symbol_qualified_name.clone(),
        operation.channel_name.clone(),
      ))
      .or_else(|| {
        channel_bindings.get(&(
          operation.source_symbol_qualified_name.clone(),
          operation.channel_name.clone(),
        ))
      })
      .cloned()
  };

  let mut edges = Vec::new();
  let mut seen_edges = HashSet::new();

  for spawn in &spawns {
    if let Some(target_name) = &spawn.target_name {
      let key = (
        spawn.source_symbol_qualified_name.clone(),
        target_name.clone(),
        "Spawns".to_string(),
      );
      if seen_edges.insert(key) {
        edges.push(ExtractedConcurrencyEdge {
          source_symbol_qualified_name: spawn.source_symbol_qualified_name.clone(),
          target_name: target_name.clone(),
          kind: "Spawns".to_string(),
          start_line: spawn.start_line,
          start_column: spawn.start_column,
          end_line: spawn.end_line,
          end_column: spawn.end_column,
        });
      }
    }
  }

  for send in &sends {
    let Some(send_identity) = operation_channel_identity(send) else {
      continue;
    };
    for receive in &receives {
      let Some(receive_identity) = operation_channel_identity(receive) else {
        continue;
      };
      if send_identity == receive_identity
        && send.source_symbol_qualified_name != receive.source_symbol_qualified_name
      {
        let key = (
          send.source_symbol_qualified_name.clone(),
          receive.source_symbol_qualified_name.clone(),
          "SendsTo".to_string(),
        );
        if seen_edges.insert(key) {
          edges.push(ExtractedConcurrencyEdge {
            source_symbol_qualified_name: send.source_symbol_qualified_name.clone(),
            target_name: receive.source_symbol_qualified_name.clone(),
            kind: "SendsTo".to_string(),
            start_line: send.start_line,
            start_column: send.start_column,
            end_line: receive.end_line,
            end_column: receive.end_column,
          });
        }
      }
    }
  }

  for flow_channel in &flow_channels {
    for upstream_reference in &flow_channel.upstream_flow_references {
      let upstream_qname = if let Some(identity) = local_channel_identities.get(&(
        flow_channel.source_symbol_qualified_name.clone(),
        upstream_reference.clone(),
      )) {
        identity.clone()
      } else {
        let Some(upstream_candidates) = simple_name_to_qnames.get(upstream_reference) else {
          continue;
        };
        if upstream_candidates.len() != 1 {
          continue;
        }
        upstream_candidates[0].clone()
      };
      if upstream_qname == flow_channel.source_symbol_qualified_name {
        continue;
      }
      let key = (
        upstream_qname.clone(),
        flow_channel.source_symbol_qualified_name.clone(),
        "SendsTo".to_string(),
      );
      if seen_edges.insert(key) {
        edges.push(ExtractedConcurrencyEdge {
          source_symbol_qualified_name: upstream_qname,
          target_name: flow_channel.source_symbol_qualified_name.clone(),
          kind: "SendsTo".to_string(),
          start_line: flow_channel.start_line,
          start_column: flow_channel.start_column,
          end_line: flow_channel.end_line,
          end_column: flow_channel.end_column,
        });
      }
    }
  }

  let extract_parent_scope = |qname: &str| -> String {
    if let Some(idx) = qname.rfind('.') {
      qname[..idx].to_string()
    } else if let Some(idx) = qname.rfind("::") {
      qname[..idx].to_string()
    } else {
      "".to_string()
    }
  };

  for flow_consumer in &flow_consumers {
    let mut upstream_qname = None;

    // 1. Try to resolve as a class/package property in the parent scope
    let parent_scope = extract_parent_scope(&flow_consumer.target_symbol_qualified_name);
    if !parent_scope.is_empty() {
      if let Some(identity) =
        local_channel_identities.get(&(parent_scope, flow_consumer.upstream_flow_reference.clone()))
      {
        upstream_qname = Some(identity.clone());
      }
    }

    // 2. Fall back to simple_name_to_qnames
    if upstream_qname.is_none() {
      if let Some(upstream_candidates) =
        simple_name_to_qnames.get(&flow_consumer.upstream_flow_reference)
      {
        if upstream_candidates.len() == 1 {
          upstream_qname = Some(upstream_candidates[0].clone());
        }
      }
    }

    let Some(up_qname) = upstream_qname else {
      continue;
    };

    let key = (
      up_qname.clone(),
      flow_consumer.target_symbol_qualified_name.clone(),
      "SendsTo".to_string(),
    );
    if seen_edges.insert(key) {
      edges.push(ExtractedConcurrencyEdge {
        source_symbol_qualified_name: up_qname,
        target_name: flow_consumer.target_symbol_qualified_name.clone(),
        kind: "SendsTo".to_string(),
        start_line: flow_consumer.start_line,
        start_column: flow_consumer.start_column,
        end_line: flow_consumer.end_line,
        end_column: flow_consumer.end_column,
      });
    }
  }

  Ok((edges, spawns, channels, selects))
}

pub fn extract_ktor_routes_from_source(
  source: &str,
  file_path: &str,
) -> Result<Vec<ExtractedKotlinRoute>, KotlinParseError> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let package_name = extract_package_name(root, source).unwrap_or_default();

  let mut routes = Vec::new();
  let mut path_stack = Vec::new();

  traverse_ktor_routes(
    root,
    source,
    file_path,
    &package_name,
    None,
    None,
    &mut path_stack,
    &mut routes,
  );

  Ok(routes)
}

#[allow(clippy::too_many_arguments, clippy::only_used_in_recursion)]
fn traverse_ktor_routes(
  node: Node<'_>,
  source: &str,
  file_path: &str,
  package_name: &str,
  enclosing_class: Option<&str>,
  enclosing_func: Option<&str>,
  path_stack: &mut Vec<String>,
  routes: &mut Vec<ExtractedKotlinRoute>,
) {
  let kind = node.kind();

  let mut current_class = enclosing_class;
  let mut current_func = enclosing_func;

  if kind == "class_declaration" {
    if let Some(name_node) = find_name_child(node) {
      if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
        current_class = Some(name.trim());
      }
    }
  } else if kind == "function_declaration" {
    if let Some(name_node) = find_name_child(node) {
      if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
        current_func = Some(name.trim());
      }
    }
  }

  let mut path_pushed = false;

  if kind == "call_expression" {
    let first_child = node.child(0);
    let is_outer_call = first_child
      .map(|c| c.kind() == "call_expression")
      .unwrap_or(false);

    if is_outer_call {
      let inner_call = first_child.unwrap();
      if let Some(callee) = find_callee_name(inner_call, source) {
        let callee = callee.trim();
        if callee == "route" {
          let path_arg = extract_path_arg(inner_call, source);
          let path_str = path_arg.unwrap_or_default();
          path_stack.push(path_str);
          path_pushed = true;
        }
      }
    } else {
      if let Some(callee) = find_callee_name(node, source) {
        let callee = callee.trim();
        let is_route = matches!(
          callee,
          "get" | "post" | "put" | "delete" | "patch" | "options" | "head" | "route" | "routing"
        );

        if is_route {
          let method = match callee {
            "get" => "GET",
            "post" => "POST",
            "put" => "PUT",
            "delete" => "DELETE",
            "patch" => "PATCH",
            "options" => "OPTIONS",
            "head" => "HEAD",
            "route" => extract_method_arg(node, source).unwrap_or(""),
            _ => "",
          };

          if !method.is_empty() {
            let path_arg = extract_path_arg(node, source);
            let path_str = path_arg.unwrap_or_default();

            let is_route_callee = callee == "route";
            if !is_route_callee {
              path_stack.push(path_str);
            }

            let full_path = format!(
              "/{}",
              path_stack
                .iter()
                .map(|s| s.trim_matches('/'))
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("/")
            );

            if !is_route_callee {
              path_stack.pop();
            }

            let clean_class = current_class.unwrap_or("MainKt");
            let clean_func = current_func.unwrap_or("routing");

            let qualified_name = if package_name.is_empty() {
              format!("{}::{}", clean_class, clean_func)
            } else {
              format!("{}::{}", package_name, clean_func)
            };

            let clean_path = full_path.replace('/', "_");
            let clean_path = clean_path.replace(|c: char| !c.is_alphanumeric(), "_");
            let mut clean_path = clean_path;
            while clean_path.contains("__") {
              clean_path = clean_path.replace("__", "_");
            }
            let clean_path = clean_path.trim_matches('_');
            let handler_name = format!("{}_{}", method.to_lowercase(), clean_path);

            let start = node.start_position();

            routes.push(ExtractedKotlinRoute {
              handler_name,
              qualified_name,
              method: method.to_string(),
              path: full_path,
              response_model: None,
              start_line: start.row + 1,
              start_column: start.column + 1,
              end_line: node.end_position().row + 1,
              end_column: node.end_position().column + 1,
            });
          }
        }
      }
    }
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_ktor_routes(
      child,
      source,
      file_path,
      package_name,
      current_class,
      current_func,
      path_stack,
      routes,
    );
  }

  if path_pushed {
    path_stack.pop();
  }
}

fn find_callee_name(call_node: Node<'_>, source: &str) -> Option<String> {
  if let Some(first_child) = call_node.child(0) {
    if let Ok(text) = first_child.utf8_text(source.as_bytes()) {
      let text = text.trim().to_string();
      if let Some(last_part) = text.split('.').next_back() {
        return Some(last_part.to_string());
      }
      return Some(text);
    }
  }
  None
}

fn find_value_arguments(call_node: Node<'_>) -> Option<Node<'_>> {
  let mut cursor = call_node.walk();
  for child in call_node.children(&mut cursor) {
    if child.kind() == "value_arguments" {
      return Some(child);
    }
    if child.kind() == "call_suffix" {
      let mut sub_cursor = child.walk();
      for sub_child in child.children(&mut sub_cursor) {
        if sub_child.kind() == "value_arguments" {
          return Some(sub_child);
        }
      }
    }
  }
  None
}

fn extract_path_arg(call_node: Node<'_>, source: &str) -> Option<String> {
  if let Some(args) = find_value_arguments(call_node) {
    return find_first_string_literal_in_node(args, source);
  }
  None
}

fn find_first_string_literal_in_node(node: Node<'_>, source: &str) -> Option<String> {
  let kind = node.kind();
  if kind == "string_literal" || kind.contains("string") {
    if let Ok(text) = node.utf8_text(source.as_bytes()) {
      return Some(text.trim_matches('"').trim_matches('\'').to_string());
    }
  }
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if let Some(res) = find_first_string_literal_in_node(child, source) {
      return Some(res);
    }
  }
  None
}

fn extract_method_arg(call_node: Node<'_>, source: &str) -> Option<&'static str> {
  if let Some(args) = find_value_arguments(call_node) {
    return find_http_method_in_arguments(args, source);
  }
  None
}

fn find_http_method_in_arguments(node: Node<'_>, source: &str) -> Option<&'static str> {
  if node.kind() == "simple_identifier" || node.kind() == "value_argument" {
    if let Ok(text) = node.utf8_text(source.as_bytes()) {
      let upper = text.to_uppercase();
      if upper.contains("POST") {
        return Some("POST");
      }
      if upper.contains("PUT") {
        return Some("PUT");
      }
      if upper.contains("DELETE") {
        return Some("DELETE");
      }
      if upper.contains("PATCH") {
        return Some("PATCH");
      }
      if upper.contains("OPTIONS") {
        return Some("OPTIONS");
      }
      if upper.contains("HEAD") {
        return Some("HEAD");
      }
      if upper.contains("GET") {
        return Some("GET");
      }
    }
  }
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if let Some(res) = find_http_method_in_arguments(child, source) {
      return Some(res);
    }
  }
  None
}

// ----------------------------------------
// Tests
// ----------------------------------------
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_basic_kotlin_symbols() {
    let source = r#"
      package com.example.patient

      class Patient(val id: String) {
        fun validate(): Boolean {
          val localProperty = "leak"
          return id.isNotEmpty()
        }

        companion object {
          fun createDefault(): Patient {
            return Patient("default")
          }
        }
      }

      enum class Status {
        ACTIVE, INACTIVE
      }

      typealias PatientId = String

      interface Validatable {
        fun isValid(): Boolean
      }

      object PatientRegistry {
        val count = 0
      }

      fun String.isAlpha(): Boolean {
        return this.all { it.isLetter() }
      }

      val Int.doubleVal: Int
        get() = this * 2
    "#;

    let symbols = extract_symbols_from_source(source, "src/Patient.kt").unwrap();

    // Verify module symbol
    let module_sym = symbols
      .iter()
      .find(|s| s.kind == SymbolKind::Module)
      .unwrap();
    assert_eq!(module_sym.qualified_name, "com.example.patient");

    // Verify class
    let class_sym = symbols
      .iter()
      .find(|s| s.name == "Patient" && s.kind == SymbolKind::Class)
      .unwrap();
    assert_eq!(class_sym.kind, SymbolKind::Class);
    assert_eq!(class_sym.qualified_name, "com.example.patient::Patient");

    // Verify method
    let method_sym = symbols.iter().find(|s| s.name == "validate").unwrap();
    assert_eq!(method_sym.kind, SymbolKind::Method);
    assert_eq!(
      method_sym.qualified_name,
      "com.example.patient::Patient.validate"
    );

    // Verify local variable count/localProperty is NOT indexed (prevents database pollution)
    assert!(symbols.iter().all(|s| s.name != "localProperty"));

    // Verify companion object
    let comp_sym = symbols.iter().find(|s| s.name == "Companion").unwrap();
    assert_eq!(comp_sym.kind, SymbolKind::Class);
    assert_eq!(
      comp_sym.qualified_name,
      "com.example.patient::Patient.Companion"
    );

    // Verify method inside companion
    let comp_method = symbols.iter().find(|s| s.name == "createDefault").unwrap();
    assert_eq!(comp_method.kind, SymbolKind::Method);
    assert_eq!(
      comp_method.qualified_name,
      "com.example.patient::Patient.Companion.createDefault"
    );

    // Verify enum class
    let enum_sym = symbols.iter().find(|s| s.name == "Status").unwrap();
    assert_eq!(enum_sym.kind, SymbolKind::Enum);
    assert_eq!(enum_sym.qualified_name, "com.example.patient::Status");

    // Verify actual typealias
    let alias_sym = symbols.iter().find(|s| s.name == "PatientId").unwrap();
    assert_eq!(alias_sym.kind, SymbolKind::TypeAlias);
    assert_eq!(alias_sym.qualified_name, "com.example.patient::PatientId");

    // Verify interface
    let interface_sym = symbols.iter().find(|s| s.name == "Validatable").unwrap();
    assert_eq!(interface_sym.kind, SymbolKind::Interface);
    assert_eq!(
      interface_sym.qualified_name,
      "com.example.patient::Validatable"
    );

    // Verify object
    let obj_sym = symbols
      .iter()
      .find(|s| s.name == "PatientRegistry")
      .unwrap();
    assert_eq!(obj_sym.kind, SymbolKind::Class);
    assert_eq!(
      obj_sym.qualified_name,
      "com.example.patient::PatientRegistry"
    );

    // Verify property mapped to Unknown
    let prop_sym = symbols.iter().find(|s| s.name == "count").unwrap();
    assert_eq!(prop_sym.kind, SymbolKind::Unknown);
    assert_eq!(
      prop_sym.qualified_name,
      "com.example.patient::PatientRegistry.count"
    );

    // Verify extension function
    let ext_fn = symbols.iter().find(|s| s.name == "isAlpha").unwrap();
    assert_eq!(ext_fn.kind, SymbolKind::Function);
    assert_eq!(ext_fn.qualified_name, "com.example.patient::String.isAlpha");

    // Verify extension property
    let ext_prop = symbols.iter().find(|s| s.name == "doubleVal").unwrap();
    assert_eq!(ext_prop.kind, SymbolKind::Unknown);
    assert_eq!(
      ext_prop.qualified_name,
      "com.example.patient::Int.doubleVal"
    );
  }

  #[test]
  fn test_kotlin_relationships() {
    let source = r#"
      package com.example.patient
      import com.example.db.Database
      import com.example.util.*
      import com.example.log.Logger as SimpleLogger

      @Inject
      class Patient(val id: String) : Person(), Validatable {
        @Test
        fun validate(db: Database): Boolean {
          val logger = SimpleLogger()
          logger.info(id)
          return id.isNotEmpty()
        }
      }
    "#;

    let imports = extract_imports_from_source(source, "src/Patient.kt").unwrap();
    assert_eq!(imports.len(), 3);
    assert!(
      imports
        .iter()
        .any(|i| i.target_qualified_name == "com.example.db.Database" && i.alias_name.is_none())
    );
    assert!(
      imports
        .iter()
        .any(|i| i.target_qualified_name == "com.example.util.*" && i.alias_name.is_none())
    );
    assert!(
      imports
        .iter()
        .any(|i| i.target_qualified_name == "com.example.log.Logger"
          && i.alias_name == Some("SimpleLogger".to_string()))
    );

    let calls = extract_calls_from_source(source, "src/Patient.kt").unwrap();
    // Expect: SimpleLogger() constructor call and logger.info(id) navigation call
    assert!(
      calls
        .iter()
        .any(|c| c.target_name == "SimpleLogger" && !c.is_self_call)
    );
    assert!(calls.iter().any(|c| c.target_name == "info"
      && !c.is_self_call
      && c.method_name == Some("info".to_string())));

    let type_refs = extract_type_references_from_source(source, "src/Patient.kt").unwrap();
    // Expect to reference Database (from db parameter) but NOT String or Boolean (filtered built-ins)
    assert!(type_refs.iter().any(|t| t.target_type_name == "Database"));
    assert!(!type_refs.iter().any(|t| t.target_type_name == "String"));
    assert!(!type_refs.iter().any(|t| t.target_type_name == "Boolean"));

    let inheritance = extract_inheritance_from_source(source, "src/Patient.kt").unwrap();
    assert_eq!(inheritance.len(), 2);
    let person_inh = inheritance
      .iter()
      .find(|i| i.supertype_name == "Person")
      .unwrap();
    let valid_inh = inheritance
      .iter()
      .find(|i| i.supertype_name == "Validatable")
      .unwrap();
    assert!(person_inh.is_class_extends); // class inheritance has parentheses
    assert!(!valid_inh.is_class_extends); // interface implementation does not

    let annotations = extract_annotations_from_source(source, "src/Patient.kt").unwrap();
    assert_eq!(annotations.len(), 2);
    assert!(annotations.iter().any(|a| a.annotation_name == "Inject"
      && a.target_symbol_qualified_name == "com.example.patient::Patient"));
    assert!(annotations.iter().any(|a| a.annotation_name == "Test"
      && a.target_symbol_qualified_name == "com.example.patient::Patient.validate"));
  }

  #[test]
  fn test_kotlin_flow_relationships_capture_pipeline_helpers() {
    let source = r#"
      package com.example.flow

      import kotlinx.coroutines.CoroutineScope
      import kotlinx.coroutines.flow.Flow
      import kotlinx.coroutines.flow.catch
      import kotlinx.coroutines.flow.collect
      import kotlinx.coroutines.flow.flow
      import kotlinx.coroutines.flow.map

      fun updatesFlow(): Flow<Int> = flow {
        emit(fetchUpdate())
      }

      fun sharedUpdates(scope: CoroutineScope): Flow<Int> {
        val mapped = updatesFlow()
          .map { normalize(it) }
          .catch { logFailure() }
        return mapped
      }

      suspend fun persistUpdates(scope: CoroutineScope) {
        sharedUpdates(scope).collect { persist(it) }
      }

      fun fetchUpdate(): Int = 1
      fun normalize(value: Int): Int = value
      fun logFailure() {}
      fun persist(value: Int) {}
    "#;

    let calls =
      extract_calls_from_source(source, "src/main/kotlin/com/example/flow/App.kt").unwrap();

    assert!(calls.iter().any(|call| {
      call.source_symbol_qualified_name == "com.example.flow::sharedUpdates"
        && call.target_name == "updatesFlow"
    }));
    assert!(calls.iter().any(|call| {
      call.source_symbol_qualified_name == "com.example.flow::sharedUpdates"
        && call.target_name == "normalize"
    }));
    assert!(calls.iter().any(|call| {
      call.source_symbol_qualified_name == "com.example.flow::sharedUpdates"
        && call.target_name == "logFailure"
    }));
    assert!(calls.iter().any(|call| {
      call.source_symbol_qualified_name == "com.example.flow::persistUpdates"
        && call.target_name == "persist"
    }));
  }

  #[test]
  fn test_extract_kotlin_routes() {
    let source = r#"
      package com.example.api

      fun Application.module() {
          routing {
              get("/users") {
                  call.respondText("users")
              }
              route("/api") {
                  post("/items") {
                      call.respondText("post item")
                  }
                  route("/v2", HttpMethod.Put) {
                      handle {
                          call.respondText("put item")
                      }
                  }
              }
          }
      }
    "#;

    let routes =
      extract_ktor_routes_from_source(source, "src/main/kotlin/com/example/api/App.kt").unwrap();
    assert_eq!(routes.len(), 3);

    let get_route = routes.iter().find(|r| r.path == "/users").unwrap();
    assert_eq!(get_route.method, "GET");
    assert_eq!(get_route.qualified_name, "com.example.api::module");
    assert_eq!(get_route.handler_name, "get_users");

    let post_route = routes.iter().find(|r| r.path == "/api/items").unwrap();
    assert_eq!(post_route.method, "POST");
    assert_eq!(post_route.qualified_name, "com.example.api::module");
    assert_eq!(post_route.handler_name, "post_api_items");

    let put_route = routes.iter().find(|r| r.path == "/api/v2").unwrap();
    assert_eq!(put_route.method, "PUT");
    assert_eq!(put_route.qualified_name, "com.example.api::module");
    assert_eq!(put_route.handler_name, "put_api_v2");
  }

  #[test]
  fn test_extract_kotlin_concurrency() {
    let source = r#"
      package com.example.concurrent

      import kotlinx.coroutines.CoroutineScope
      import kotlinx.coroutines.async
      import kotlinx.coroutines.channels.Channel
      import kotlinx.coroutines.launch
      import kotlinx.coroutines.selects.select

      fun orchestrate(scope: CoroutineScope) {
        val updates = Channel<Int>()

        scope.launch {
          publishUpdates(updates)
        }

        scope.async {
          readUpdates(updates)
        }

        select<Unit> {
          updates.onReceive { value ->
            println(value)
          }
        }
      }

      suspend fun publishUpdates(updates: Channel<Int>) {
        updates.send(42)
      }

      suspend fun readUpdates(updates: Channel<Int>) {
        updates.receive()
      }
    "#;

    let (edges, spawns, channels, selects) =
      extract_concurrency_from_source(source, "src/main/kotlin/com/example/concurrent/App.kt")
        .unwrap();

    assert!(spawns.iter().any(|s| {
      s.source_symbol_qualified_name == "com.example.concurrent::orchestrate"
        && s.spawn_kind == "launch"
        && s.target_name.as_deref() == Some("publishUpdates")
    }));
    assert!(spawns.iter().any(|s| {
      s.source_symbol_qualified_name == "com.example.concurrent::orchestrate"
        && s.spawn_kind == "async"
        && s.target_name.as_deref() == Some("readUpdates")
    }));

    assert!(channels.iter().any(|c| {
      c.source_symbol_qualified_name == "com.example.concurrent::orchestrate"
        && c.channel_kind == "Channel"
        && c.tx_name == "updates"
        && c.rx_name == "updates"
    }));

    assert_eq!(selects.len(), 1);
    assert!(
      selects
        .iter()
        .any(|s| { s.source_symbol_qualified_name == "com.example.concurrent::orchestrate" })
    );

    assert!(edges.iter().any(|e| {
      e.source_symbol_qualified_name == "com.example.concurrent::orchestrate"
        && e.target_name == "publishUpdates"
        && e.kind == "Spawns"
    }));
    assert!(edges.iter().any(|e| {
      e.source_symbol_qualified_name == "com.example.concurrent::orchestrate"
        && e.target_name == "readUpdates"
        && e.kind == "Spawns"
    }));
    assert!(edges.iter().any(|e| {
      e.source_symbol_qualified_name == "com.example.concurrent::publishUpdates"
        && e.target_name == "com.example.concurrent::readUpdates"
        && e.kind == "SendsTo"
    }));
  }

  #[test]
  fn test_extract_kotlin_concurrency_avoids_cross_pairing_same_named_channels() {
    let source = r#"
      package com.example.concurrent

      import kotlinx.coroutines.channels.Channel

      fun first() {
        val updates = Channel<Int>()
        sendFirst(updates)
      }

      fun second() {
        val updates = Channel<Int>()
        receiveSecond(updates)
      }

      suspend fun sendFirst(updates: Channel<Int>) {
        updates.send(1)
      }

      suspend fun receiveSecond(updates: Channel<Int>) {
        updates.receive()
      }
    "#;

    let (edges, _, _, selects) =
      extract_concurrency_from_source(source, "src/main/kotlin/com/example/concurrent/App.kt")
        .unwrap();

    assert!(selects.is_empty());
    assert!(!edges.iter().any(|e| {
      e.source_symbol_qualified_name == "com.example.concurrent::sendFirst"
        && e.target_name == "com.example.concurrent::receiveSecond"
        && e.kind == "SendsTo"
    }));
  }

  #[test]
  fn test_extract_kotlin_spawn_prefers_last_worker_call() {
    let source = r#"
      package com.example.concurrent

      import kotlinx.coroutines.CoroutineScope
      import kotlinx.coroutines.launch

      fun orchestrate(scope: CoroutineScope) {
        scope.launch {
          logStart()
          doWork()
        }
      }

      fun logStart() {}
      fun doWork() {}
    "#;

    let (_, spawns, _, _) =
      extract_concurrency_from_source(source, "src/main/kotlin/com/example/concurrent/App.kt")
        .unwrap();

    assert!(spawns.iter().any(|spawn| {
      spawn.source_symbol_qualified_name == "com.example.concurrent::orchestrate"
        && spawn.target_name.as_deref() == Some("doWork")
    }));
  }

  #[test]
  fn test_extract_kotlin_flow_topology() {
    let source = r#"
      package com.example.flow

      import kotlinx.coroutines.flow.Flow
      import kotlinx.coroutines.flow.flow
      import kotlinx.coroutines.flow.map
      import kotlinx.coroutines.flow.catch
      import kotlinx.coroutines.flow.shareIn
      import kotlinx.coroutines.flow.collect

      fun updatesFlow(): Flow<Int> = flow {
        emit(fetchUpdate())
      }

      fun sharedUpdates(scope: CoroutineScope): Flow<Int> {
        val mapped = updatesFlow()
          .map { normalize(it) }
          .catch { logFailure() }
        return mapped.shareIn(scope, SharingStarted.Eagerly, replay = 1)
      }

      suspend fun persistUpdates(scope: CoroutineScope) {
        sharedUpdates(scope).collect { persist(it) }
      }

      fun fetchUpdate(): Int = 1
      fun normalize(value: Int): Int = value
      fun logFailure() {}
      fun persist(value: Int) {}
    "#;

    let (edges, spawns, channels, selects) =
      extract_concurrency_from_source(source, "src/main/kotlin/com/example/flow/App.kt").unwrap();

    assert!(spawns.is_empty());
    assert!(selects.is_empty());

    assert!(channels.iter().any(|channel| {
      channel.source_symbol_qualified_name == "com.example.flow::updatesFlow"
        && channel.channel_kind == "Flow"
        && channel.tx_name == "updatesFlow"
        && channel.rx_name == "updatesFlow"
    }));
    assert!(channels.iter().any(|channel| {
      channel.source_symbol_qualified_name == "com.example.flow::sharedUpdates"
        && channel.channel_kind == "Flow"
        && channel.tx_name == "mapped"
        && channel.rx_name == "mapped"
    }));

    assert!(edges.iter().any(|edge| {
      edge.source_symbol_qualified_name == "com.example.flow::updatesFlow"
        && edge.target_name == "com.example.flow::sharedUpdates"
        && edge.kind == "SendsTo"
    }));
    assert!(!edges.iter().any(|edge| {
      edge.source_symbol_qualified_name == "com.example.flow::fetchUpdate"
        && edge.target_name == "com.example.flow::updatesFlow"
        && edge.kind == "SendsTo"
    }));
    assert!(edges.iter().any(|edge| {
      edge.source_symbol_qualified_name == "com.example.flow::sharedUpdates"
        && edge.target_name == "com.example.flow::persistUpdates"
        && edge.kind == "SendsTo"
    }));
  }

  #[test]
  fn test_extract_kotlin_direct_return_flow_transform() {
    let source = r#"
      package com.example.flow

      import kotlinx.coroutines.flow.Flow
      import kotlinx.coroutines.flow.flow
      import kotlinx.coroutines.flow.map

      fun updatesFlow(): Flow<Int> = flow {
        emit(fetchUpdate())
      }

      fun mappedFlow(): Flow<Int> = updatesFlow().map { normalize(it) }

      fun fetchUpdate(): Int = 1
      fun normalize(value: Int): Int = value
    "#;

    let (edges, _, channels, _) =
      extract_concurrency_from_source(source, "src/main/kotlin/com/example/flow/App.kt").unwrap();

    assert!(channels.iter().any(|channel| {
      channel.source_symbol_qualified_name == "com.example.flow::mappedFlow"
        && channel.channel_kind == "Flow"
        && channel.tx_name == "mappedFlow"
    }));
    assert!(edges.iter().any(|edge| {
      edge.source_symbol_qualified_name == "com.example.flow::updatesFlow"
        && edge.target_name == "com.example.flow::mappedFlow"
        && edge.kind == "SendsTo"
    }));
  }

  #[test]
  fn test_extract_kotlin_multi_source_flow_operators_are_supported() {
    let source = r#"
      package com.example.flow

      import kotlinx.coroutines.flow.Flow
      import kotlinx.coroutines.flow.combine
      import kotlinx.coroutines.flow.flowOf

      fun left(): Flow<Int> = flowOf(1)
      fun right(): Flow<Int> = flowOf(2)
      fun combined(): Flow<Int> = left().combine(right()) { a, b -> a + b }
    "#;

    let (edges, _, channels, _) =
      extract_concurrency_from_source(source, "src/main/kotlin/com/example/flow/App.kt").unwrap();

    assert!(channels.iter().any(|channel| {
      channel.source_symbol_qualified_name == "com.example.flow::combined"
        && channel.channel_kind == "Flow"
    }));

    // Verify that we extract SendsTo edges from both left and right to combined
    assert!(edges.iter().any(|edge| {
      edge.source_symbol_qualified_name == "com.example.flow::left"
        && edge.target_name == "com.example.flow::combined"
        && edge.kind == "SendsTo"
    }));
    assert!(edges.iter().any(|edge| {
      edge.source_symbol_qualified_name == "com.example.flow::right"
        && edge.target_name == "com.example.flow::combined"
        && edge.kind == "SendsTo"
    }));
  }

  #[test]
  fn test_extract_kotlin_flow_collection_methods() {
    let source = r#"
      package com.example.flow

      import kotlinx.coroutines.flow.Flow
      import kotlinx.coroutines.flow.flowOf
      import kotlinx.coroutines.flow.collectLatest
      import kotlinx.coroutines.flow.collectIndexed
      import kotlinx.coroutines.flow.launchIn

      fun srcFlow(): Flow<Int> = flowOf(1)

      suspend fun runLatest() {
        srcFlow().collectLatest { println(it) }
      }

      suspend fun runIndexed() {
        srcFlow().collectIndexed { idx, it -> println(it) }
      }

      fun runLaunch(scope: CoroutineScope) {
        srcFlow().launchIn(scope)
      }
    "#;

    let (edges, _, _, _) =
      extract_concurrency_from_source(source, "src/main/kotlin/com/example/flow/App.kt").unwrap();

    assert!(edges.iter().any(|edge| {
      edge.source_symbol_qualified_name == "com.example.flow::srcFlow"
        && edge.target_name == "com.example.flow::runLatest"
        && edge.kind == "SendsTo"
    }));
    assert!(edges.iter().any(|edge| {
      edge.source_symbol_qualified_name == "com.example.flow::srcFlow"
        && edge.target_name == "com.example.flow::runIndexed"
        && edge.kind == "SendsTo"
    }));
    assert!(edges.iter().any(|edge| {
      edge.source_symbol_qualified_name == "com.example.flow::srcFlow"
        && edge.target_name == "com.example.flow::runLaunch"
        && edge.kind == "SendsTo"
    }));
  }

  #[test]
  fn test_extract_kotlin_android_resource_references() {
    let source = r#"
      package com.example.app

      import android.os.Bundle
      import androidx.appcompat.app.AppCompatActivity

      class MainActivity : AppCompatActivity() {
          override fun onCreate(savedInstanceState: Bundle?) {
              super.onCreate(savedInstanceState)
              setContentView(R  .  layout  .  activity_main)
              val button = findViewById<Button>(R.id.btn_submit)
              val appName = getString(R.string.app_name).toString()
          }
      }
    "#;

    let type_refs = extract_type_references_from_source(source, "src/MainActivity.kt").unwrap();

    // Verify space-separated extraction works
    assert!(type_refs.iter().any(|tr| tr.source_symbol_qualified_name
      == "com.example.app::MainActivity.onCreate"
      && tr.target_type_name == "R.layout.activity_main"));

    // Verify standard extraction works
    assert!(type_refs.iter().any(|tr| tr.source_symbol_qualified_name
      == "com.example.app::MainActivity.onCreate.button"
      && tr.target_type_name == "R.id.btn_submit"));

    // Verify chained extraction extracts R.string.app_name and avoids duplicates
    let app_name_refs: Vec<_> = type_refs
      .iter()
      .filter(|tr| tr.target_type_name == "R.string.app_name")
      .collect();
    assert_eq!(app_name_refs.len(), 1);

    // Verify we do not extract R.string.app_name.toString
    assert!(
      !type_refs
        .iter()
        .any(|tr| tr.target_type_name.contains("toString"))
    );
  }
}
