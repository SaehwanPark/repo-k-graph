#![allow(clippy::collapsible_if)]
use rkg_core::{Location, Symbol, SymbolKind};
use std::path::Path;
use tree_sitter::{Node, Parser};

pub const CRATE_NAME: &str = "rkg-lang-rust";

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedImplementation {
  pub struct_name: String,
  pub trait_name: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedRustRoute {
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

#[derive(Debug, Clone)]
pub struct LocalFuncInfo {
  pub qualified_name: String,
  pub response_model: Option<String>,
  pub state_dependencies: Vec<String>,
}

fn extract_state_type(type_text: &str) -> Option<String> {
  if type_text.contains("State<") || type_text.contains("Data<") {
    if let Some(start) = type_text.find('<') {
      if let Some(end) = type_text.rfind('>') {
        let mut inner = type_text[start + 1..end].trim();
        while inner.starts_with("Arc<")
          || inner.starts_with("Mutex<")
          || inner.starts_with("RwLock<")
          || inner.starts_with("Option<")
        {
          if let Some(inner_start) = inner.find('<') {
            if let Some(inner_end) = inner.rfind('>') {
              inner = inner[inner_start + 1..inner_end].trim();
            } else {
              break;
            }
          } else {
            break;
          }
        }
        if !inner.is_empty() {
          return Some(inner.to_string());
        }
      }
    }
  }
  None
}

fn extract_json_payload_type(type_text: &str) -> Option<String> {
  if type_text.contains("Json<") {
    if let Some(start) = type_text.find('<') {
      if let Some(end) = type_text.rfind('>') {
        let inner = type_text[start + 1..end].trim();
        if !inner.is_empty() {
          return Some(inner.to_string());
        }
      }
    }
  }
  None
}

fn extract_path_payload_type(type_text: &str) -> Option<String> {
  if type_text.contains("Path<") {
    if let Some(start) = type_text.find('<') {
      if let Some(end) = type_text.rfind('>') {
        let mut inner = type_text[start + 1..end].trim();
        while inner.starts_with("Option<") || inner.starts_with("Result<") {
          if let Some(inner_start) = inner.find('<') {
            if let Some(inner_end) = inner.rfind('>') {
              inner = inner[inner_start + 1..inner_end].trim();
            } else {
              break;
            }
          } else {
            break;
          }
        }
        if should_track_path_payload_dependency(inner) {
          return Some(inner.to_string());
        }
      }
    }
  }
  None
}

fn should_track_path_payload_dependency(inner: &str) -> bool {
  let inner = inner.trim().trim_start_matches('&').trim();
  if inner.is_empty() || inner.starts_with('(') || inner.starts_with('[') {
    return false;
  }

  let simple_name = inner.rsplit("::").next().unwrap_or(inner);
  !is_primitive_type(simple_name)
}

fn extract_route_dependency_type(type_text: &str) -> Option<String> {
  extract_state_type(type_text)
    .or_else(|| extract_json_payload_type(type_text))
    .or_else(|| extract_path_payload_type(type_text))
}

fn extract_response_model_from_return_type(ret_text: &str) -> Option<String> {
  if let Some(pos) = ret_text.find("Json<") {
    let rest = &ret_text[pos + 5..];
    if let Some(end) = rest.rfind('>') {
      let inner = rest[..end].trim();
      if !inner.is_empty() {
        return Some(inner.to_string());
      }
    }
  }
  None
}

fn parse_actix_attribute(attr_text: &str) -> Option<(String, String)> {
  let mut clean = attr_text.trim();
  if clean.starts_with("#[") && clean.ends_with(']') {
    clean = clean[2..clean.len() - 1].trim();
  }
  if let Some(pos) = clean.find('(') {
    let macro_name = clean[..pos].trim();
    let lower_name = macro_name.to_lowercase();
    let method = if lower_name.ends_with("get") {
      Some("GET".to_string())
    } else if lower_name.ends_with("post") {
      Some("POST".to_string())
    } else if lower_name.ends_with("put") {
      Some("PUT".to_string())
    } else if lower_name.ends_with("delete") {
      Some("DELETE".to_string())
    } else if lower_name.ends_with("patch") {
      Some("PATCH".to_string())
    } else if lower_name.ends_with("options") {
      Some("OPTIONS".to_string())
    } else if lower_name.ends_with("head") {
      Some("HEAD".to_string())
    } else {
      None
    };

    if let Some(m) = method {
      let args = &clean[pos + 1..clean.len() - 1];
      if let Some(start_quote) = args.find('"') {
        if let Some(end_quote) = args[start_quote + 1..].find('"') {
          let path = args[start_quote + 1..start_quote + 1 + end_quote].to_string();
          return Some((m, path));
        }
      }
    }
  }
  None
}

fn extract_first_identifier(node: Node<'_>, source: &str) -> Option<String> {
  let kind = node.kind();
  if kind == "identifier" || kind == "scoped_identifier" || kind == "path_expression" {
    return Some(
      node
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .trim()
        .to_string(),
    );
  }
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
      if let Some(found) = extract_first_identifier(child, source) {
        return Some(found);
      }
    }
  }
  None
}

fn extract_actix_method_from_receiver(node: Node<'_>, source: &str) -> String {
  let text = node
    .utf8_text(source.as_bytes())
    .unwrap_or("")
    .trim()
    .to_lowercase();
  if text.contains("get") {
    "GET".to_string()
  } else if text.contains("post") {
    "POST".to_string()
  } else if text.contains("put") {
    "PUT".to_string()
  } else if text.contains("delete") {
    "DELETE".to_string()
  } else if text.contains("patch") {
    "PATCH".to_string()
  } else if text.contains("options") {
    "OPTIONS".to_string()
  } else if text.contains("head") {
    "HEAD".to_string()
  } else {
    "GET".to_string()
  }
}

fn find_best_local_function(
  handler_name: &str,
  enclosing_scope: &str,
  local_functions: &std::collections::HashMap<String, LocalFuncInfo>,
) -> Option<LocalFuncInfo> {
  if let Some(info) = local_functions.get(handler_name) {
    return Some(info.clone());
  }

  let mut lookup_name = handler_name.to_string();
  if lookup_name.starts_with("self::") {
    lookup_name = lookup_name[6..].to_string();
  } else if lookup_name.starts_with("super::") {
    lookup_name = lookup_name[7..].to_string();
  } else if lookup_name.starts_with("crate::") {
    if let Some(first_segment) = enclosing_scope.split("::").next() {
      lookup_name = format!("{}::{}", first_segment, &lookup_name[7..]);
    } else {
      lookup_name = lookup_name[7..].to_string();
    }
  }

  if let Some(info) = local_functions.get(&lookup_name) {
    return Some(info.clone());
  }

  let mut matches = Vec::new();
  let target_suffix = format!("::{}", lookup_name);
  for (qname, info) in local_functions {
    if qname == &lookup_name || qname.ends_with(&target_suffix) {
      matches.push(info.clone());
    }
  }

  if matches.is_empty() {
    return None;
  }

  if matches.len() == 1 {
    return Some(matches[0].clone());
  }

  let mut best_match = None;
  let mut longest_common_prefix_len = 0;
  for m in matches {
    let mut common_len = 0;
    let m_parts: Vec<&str> = m.qualified_name.split("::").collect();
    let scope_parts: Vec<&str> = enclosing_scope.split("::").collect();
    for (p1, p2) in m_parts.iter().zip(scope_parts.iter()) {
      if p1 == p2 {
        common_len += 1;
      } else {
        break;
      }
    }
    if common_len >= longest_common_prefix_len {
      longest_common_prefix_len = common_len;
      best_match = Some(m);
    }
  }

  best_match
}

#[allow(clippy::too_many_arguments)]
fn add_route_for_handler(
  handler_name: &str,
  method: &str,
  path: &str,
  routes: &mut Vec<ExtractedRustRoute>,
  module_name: &str,
  enclosing_scope: &str,
  local_functions: &std::collections::HashMap<String, LocalFuncInfo>,
  start_pos: tree_sitter::Point,
  end_pos: tree_sitter::Point,
) {
  let clean_handler = if let Some(pos) = handler_name.rfind("::") {
    &handler_name[pos + 2..]
  } else {
    handler_name
  };

  let (qname, response_model) =
    if let Some(info) = find_best_local_function(handler_name, enclosing_scope, local_functions) {
      (info.qualified_name.clone(), info.response_model.clone())
    } else {
      (format!("{module_name}::{handler_name}"), None)
    };

  routes.push(ExtractedRustRoute {
    handler_name: clean_handler.to_string(),
    qualified_name: qname,
    method: method.to_string(),
    path: path.to_string(),
    response_model,
    start_line: start_pos.row + 1,
    start_column: start_pos.column,
    end_line: end_pos.row + 1,
    end_column: end_pos.column,
  });
}

#[allow(clippy::too_many_arguments)]
fn parse_axum_handler_expr(
  node: Node<'_>,
  source: &str,
  path: &str,
  routes: &mut Vec<ExtractedRustRoute>,
  module_name: &str,
  enclosing_scope: &str,
  local_functions: &std::collections::HashMap<String, LocalFuncInfo>,
  start_pos: tree_sitter::Point,
  end_pos: tree_sitter::Point,
) {
  let kind = node.kind();
  if kind == "call_expression" {
    if let Some(func_node) = node.child_by_field_name("function") {
      if func_node.kind() == "field_expression" {
        let field_name = func_node
          .child_by_field_name("field")
          .map(|f| f.utf8_text(source.as_bytes()).unwrap_or("").trim())
          .unwrap_or("");

        let method = field_name.to_uppercase();
        if matches!(
          method.as_str(),
          "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "OPTIONS" | "HEAD" | "ROUTE"
        ) {
          if let Some(args_node) = node.child_by_field_name("arguments") {
            if let Some(handler_name) = extract_first_identifier(args_node, source) {
              add_route_for_handler(
                &handler_name,
                &method,
                path,
                routes,
                module_name,
                enclosing_scope,
                local_functions,
                start_pos,
                end_pos,
              );
            }
          }
        }

        if field_name == "to" {
          if let Some(args_node) = node.child_by_field_name("arguments") {
            if let Some(handler_name) = extract_first_identifier(args_node, source) {
              let receiver = func_node.child_by_field_name("value");
              let method = receiver
                .map(|r| extract_actix_method_from_receiver(r, source))
                .unwrap_or_else(|| "GET".to_string());

              add_route_for_handler(
                &handler_name,
                &method,
                path,
                routes,
                module_name,
                enclosing_scope,
                local_functions,
                start_pos,
                end_pos,
              );
            }
          }
        }

        if let Some(receiver) = func_node.child_by_field_name("value") {
          parse_axum_handler_expr(
            receiver,
            source,
            path,
            routes,
            module_name,
            enclosing_scope,
            local_functions,
            start_pos,
            end_pos,
          );
        }
      } else if func_node.kind() == "identifier" {
        let method = func_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_uppercase();
        if matches!(
          method.as_str(),
          "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "OPTIONS" | "HEAD" | "ROUTE"
        ) {
          if let Some(args_node) = node.child_by_field_name("arguments") {
            if let Some(handler_name) = extract_first_identifier(args_node, source) {
              add_route_for_handler(
                &handler_name,
                &method,
                path,
                routes,
                module_name,
                enclosing_scope,
                local_functions,
                start_pos,
                end_pos,
              );
            }
          }
        }
      }
    }
  } else {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
      parse_axum_handler_expr(
        child,
        source,
        path,
        routes,
        module_name,
        enclosing_scope,
        local_functions,
        start_pos,
        end_pos,
      );
    }
  }
}

fn traverse_first_pass(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, bool)>,
  local_functions: &mut std::collections::HashMap<String, LocalFuncInfo>,
  routes: &mut Vec<ExtractedRustRoute>,
  dependencies: &mut Vec<(String, String)>,
) {
  let kind = node.kind();
  let mut scope_pushed = None;

  match kind {
    "mod_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let mod_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !mod_name.is_empty() {
          scope_pushed = Some((mod_name, false));
        }
      }
    }
    "struct_item" | "enum_item" | "trait_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !name.is_empty() {
          scope_pushed = Some((name, false));
        }
      }
    }
    "impl_item" => {
      if let Some(type_node) = node.child_by_field_name("type") {
        let type_name = get_type_name_text(type_node, source);
        if !type_name.is_empty() {
          if let Some(trait_node) = node.child_by_field_name("trait") {
            let trait_name = get_type_name_text(trait_node, source);
            let scope_name = format!("<{} as {}>", type_name, trait_name);
            scope_pushed = Some((scope_name, true));
          } else {
            scope_pushed = Some((type_name, true));
          }
        }
      }
    }
    "function_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let func_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !func_name.is_empty() {
          let qname = build_qualified_name(module_name, scope_stack, &func_name);

          let response_model = if let Some(ret_node) = node.child_by_field_name("return_type") {
            let ret_text = ret_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
            extract_response_model_from_return_type(ret_text)
          } else {
            None
          };

          let mut state_dependencies = Vec::new();
          if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
              if child.kind() == "parameter" {
                if let Some(type_node) = child.child_by_field_name("type") {
                  let type_text = type_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
                  if let Some(dependency_type) = extract_route_dependency_type(type_text) {
                    if extract_state_type(type_text).is_some() {
                      state_dependencies.push(dependency_type.clone());
                    }
                    dependencies.push((qname.clone(), dependency_type));
                  }
                }
              }
            }
          }

          local_functions.insert(
            qname.clone(),
            LocalFuncInfo {
              qualified_name: qname.clone(),
              response_model: response_model.clone(),
              state_dependencies,
            },
          );

          let attrs = get_attributes_for_node(node, source);
          for attr in attrs {
            if let Some((method, path)) = parse_actix_attribute(&attr) {
              let start = node.start_position();
              let end = node.end_position();
              routes.push(ExtractedRustRoute {
                handler_name: func_name.clone(),
                qualified_name: qname.clone(),
                method,
                path,
                response_model: response_model.clone(),
                start_line: start.row + 1,
                start_column: start.column,
                end_line: end.row + 1,
                end_column: end.column,
              });
            }
          }

          scope_pushed = Some((func_name, false));
        }
      }
    }
    _ => {}
  }

  if let Some(pushed) = scope_pushed.clone() {
    scope_stack.push(pushed);
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_first_pass(
      child,
      source,
      module_name,
      scope_stack,
      local_functions,
      routes,
      dependencies,
    );
  }

  if scope_pushed.is_some() {
    scope_stack.pop();
  }
}

fn traverse_second_pass(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, bool)>,
  local_functions: &std::collections::HashMap<String, LocalFuncInfo>,
  routes: &mut Vec<ExtractedRustRoute>,
) {
  let kind = node.kind();
  let mut scope_pushed = None;

  match kind {
    "mod_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let mod_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !mod_name.is_empty() {
          scope_pushed = Some((mod_name, false));
        }
      }
    }
    "struct_item" | "enum_item" | "trait_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !name.is_empty() {
          scope_pushed = Some((name, false));
        }
      }
    }
    "impl_item" => {
      if let Some(type_node) = node.child_by_field_name("type") {
        let type_name = get_type_name_text(type_node, source);
        if !type_name.is_empty() {
          if let Some(trait_node) = node.child_by_field_name("trait") {
            let trait_name = get_type_name_text(trait_node, source);
            let scope_name = format!("<{} as {}>", type_name, trait_name);
            scope_pushed = Some((scope_name, true));
          } else {
            scope_pushed = Some((type_name, true));
          }
        }
      }
    }
    "function_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let func_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !func_name.is_empty() {
          scope_pushed = Some((func_name, false));
        }
      }
    }
    "call_expression" => {
      if let Some(func_node) = node.child_by_field_name("function") {
        if func_node.kind() == "field_expression" {
          let field_name = func_node
            .child_by_field_name("field")
            .map(|f| f.utf8_text(source.as_bytes()).unwrap_or("").trim())
            .unwrap_or("");

          if field_name == "route" {
            if let Some(args_node) = node.child_by_field_name("arguments") {
              let mut cursor = args_node.walk();
              let mut args = Vec::new();
              for child in args_node.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
                  args.push(child);
                }
              }

              if args.len() >= 2 {
                let path_text = args[0].utf8_text(source.as_bytes()).unwrap_or("").trim();
                let clean_path = if (path_text.starts_with('"') && path_text.ends_with('"'))
                  || (path_text.starts_with('\'') && path_text.ends_with('\''))
                {
                  if path_text.len() >= 2 {
                    path_text[1..path_text.len() - 1].to_string()
                  } else {
                    path_text.to_string()
                  }
                } else {
                  path_text.to_string()
                };

                let handler_node = args[1];
                let start = node.start_position();
                let end = node.end_position();
                let enclosing_scope = get_current_enclosing_symbol(module_name, scope_stack)
                  .unwrap_or_else(|| module_name.to_string());
                parse_axum_handler_expr(
                  handler_node,
                  source,
                  &clean_path,
                  routes,
                  module_name,
                  &enclosing_scope,
                  local_functions,
                  start,
                  end,
                );
              }
            }
          }
        }
      }
    }
    _ => {}
  }

  if let Some(pushed) = scope_pushed.clone() {
    scope_stack.push(pushed);
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_second_pass(
      child,
      source,
      module_name,
      scope_stack,
      local_functions,
      routes,
    );
  }

  if scope_pushed.is_some() {
    scope_stack.pop();
  }
}

#[allow(clippy::type_complexity)]
pub fn extract_routes_and_dependencies_from_source(
  source: &str,
  file_path: &str,
  _active_features: &[String],
) -> Result<(Vec<ExtractedRustRoute>, Vec<(String, String)>), String> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let module_name = module_name_from_path(file_path);
  let mut routes = Vec::new();
  let mut dependencies = Vec::new();
  let mut scope_stack = Vec::new();
  let mut local_functions = std::collections::HashMap::new();

  traverse_first_pass(
    root,
    source,
    &module_name,
    &mut scope_stack,
    &mut local_functions,
    &mut routes,
    &mut dependencies,
  );

  let mut scope_stack2 = Vec::new();
  traverse_second_pass(
    root,
    source,
    &module_name,
    &mut scope_stack2,
    &local_functions,
    &mut routes,
  );

  Ok((routes, dependencies))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcurrencyTopology {
  pub spawns: Vec<ExtractedConcurrencySpawn>,
  pub channels: Vec<ExtractedChannelPair>,
  pub selects: Vec<ExtractedSelectBlock>,
}

pub fn extract_concurrency_topology_from_source(
  source: &str,
  file_path: &str,
  active_features: &[String],
) -> Result<ConcurrencyTopology, String> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();
  let module_name = module_name_from_path(file_path);
  let mut scope_stack = Vec::new();

  let mut spawns = Vec::new();
  let mut channels = Vec::new();
  let mut selects = Vec::new();

  traverse_concurrency(
    root,
    source,
    &module_name,
    &mut scope_stack,
    &mut spawns,
    &mut channels,
    &mut selects,
    active_features,
  );

  Ok(ConcurrencyTopology {
    spawns,
    channels,
    selects,
  })
}

fn find_first_call_in_node(node: Node<'_>, source: &str) -> Option<String> {
  if node.kind() == "call_expression" {
    if let Some(func_node) = node.child_by_field_name("function") {
      let mut func_name = func_node
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .trim()
        .to_string();
      if let Some(pos) = func_name.rfind("::") {
        func_name = func_name[pos + 2..].to_string();
      }
      if let Some(pos) = func_name.rfind('.') {
        func_name = func_name[pos + 1..].to_string();
      }
      return Some(func_name);
    }
  }
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if let Some(found) = find_first_call_in_node(child, source) {
      return Some(found);
    }
  }
  None
}

#[allow(clippy::too_many_arguments)]
fn traverse_concurrency(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, bool)>,
  spawns: &mut Vec<ExtractedConcurrencySpawn>,
  channels: &mut Vec<ExtractedChannelPair>,
  selects: &mut Vec<ExtractedSelectBlock>,
  active_features: &[String],
) {
  if !check_node_cfg(node, source, active_features) {
    return;
  }
  let kind = node.kind();
  let mut scope_pushed = None;

  match kind {
    "mod_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let mod_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !mod_name.is_empty() {
          scope_pushed = Some((mod_name, false));
        }
      }
    }
    "struct_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let struct_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !struct_name.is_empty() {
          scope_pushed = Some((struct_name, false));
        }
      }
    }
    "enum_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let enum_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !enum_name.is_empty() {
          scope_pushed = Some((enum_name, false));
        }
      }
    }
    "trait_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let trait_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !trait_name.is_empty() {
          scope_pushed = Some((trait_name, true));
        }
      }
    }
    "impl_item" => {
      if let Some(type_node) = node.child_by_field_name("type") {
        let type_name = get_type_name_text(type_node, source);
        if !type_name.is_empty() {
          if let Some(trait_node) = node.child_by_field_name("trait") {
            let trait_name = get_type_name_text(trait_node, source);
            let scope_name = format!("<{} as {}>", type_name, trait_name);
            scope_pushed = Some((scope_name, true));
          } else {
            scope_pushed = Some((type_name, true));
          }
        }
      }
    }
    "function_item" | "function_signature_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let func_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !func_name.is_empty() {
          scope_pushed = Some((func_name, false));
        }
      }
    }
    "call_expression" => {
      if let Some(caller_qname) = get_current_enclosing_symbol(module_name, scope_stack) {
        if let Some(func_node) = node.child_by_field_name("function") {
          let func_text = func_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
          if func_text == "tokio::spawn"
            || func_text == "tokio::task::spawn"
            || func_text == "thread::spawn"
            || func_text == "std::thread::spawn"
          {
            let mut target_name = None;
            if let Some(args_node) = node.child_by_field_name("arguments") {
              let mut cursor = args_node.walk();
              for arg in args_node.children(&mut cursor) {
                if let Some(found) = find_first_call_in_node(arg, source) {
                  target_name = Some(found);
                  break;
                }
              }
            }

            let start = node.start_position();
            let end = node.end_position();
            spawns.push(ExtractedConcurrencySpawn {
              source_symbol_qualified_name: caller_qname.clone(),
              spawn_kind: func_text.to_string(),
              target_name,
              start_line: start.row + 1,
              start_column: start.column,
              end_line: end.row + 1,
              end_column: end.column,
            });
          }
        }
      }
    }
    "let_declaration" => {
      if let Some(caller_qname) = get_current_enclosing_symbol(module_name, scope_stack) {
        if let Some(value_node) = node.child_by_field_name("value") {
          let value_text = value_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
          let channel_kind = if value_text.contains("std::sync::mpsc::channel") {
            Some("std_mpsc")
          } else if value_text.contains("mpsc::channel") {
            Some("mpsc")
          } else if value_text.contains("oneshot::channel") {
            Some("oneshot")
          } else if value_text.contains("broadcast::channel") {
            Some("broadcast")
          } else if value_text.contains("watch::channel") {
            Some("watch")
          } else if value_text.starts_with("channel(") || value_text.contains("::channel(") {
            Some("mpsc")
          } else {
            None
          };

          if let Some(kind_str) = channel_kind {
            if let Some(pattern_node) = node.child_by_field_name("pattern") {
              let pattern_kind = pattern_node.kind();
              if pattern_kind == "tuple_pattern" {
                let mut vars = Vec::new();
                let mut cursor = pattern_node.walk();
                for child in pattern_node.children(&mut cursor) {
                  if child.kind() == "identifier" {
                    let var_name = child
                      .utf8_text(source.as_bytes())
                      .unwrap_or("")
                      .trim()
                      .to_string();
                    if !var_name.is_empty() {
                      vars.push(var_name);
                    }
                  } else if child.kind() == "mut_pattern" {
                    if let Some(ident_node) = child.child_by_field_name("pattern") {
                      let var_name = ident_node
                        .utf8_text(source.as_bytes())
                        .unwrap_or("")
                        .trim()
                        .to_string();
                      if !var_name.is_empty() {
                        vars.push(var_name);
                      }
                    } else {
                      let var_name = child
                        .utf8_text(source.as_bytes())
                        .unwrap_or("")
                        .trim()
                        .to_string();
                      let clean = var_name
                        .strip_prefix("mut ")
                        .unwrap_or(&var_name)
                        .trim()
                        .to_string();
                      vars.push(clean);
                    }
                  }
                }
                if vars.len() == 2 {
                  let start = node.start_position();
                  let end = node.end_position();
                  channels.push(ExtractedChannelPair {
                    source_symbol_qualified_name: caller_qname.clone(),
                    channel_kind: kind_str.to_string(),
                    tx_name: vars[0].clone(),
                    rx_name: vars[1].clone(),
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
    "macro_invocation" => {
      if let Some(caller_qname) = get_current_enclosing_symbol(module_name, scope_stack) {
        let is_select = if let Some(macro_node) = node.child_by_field_name("macro") {
          let macro_name = macro_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
          macro_name == "select" || macro_name == "tokio::select" || macro_name.contains("select")
        } else if let Some(first_child) = node.child(0) {
          let macro_name = first_child
            .utf8_text(source.as_bytes())
            .unwrap_or("")
            .trim();
          macro_name == "select" || macro_name == "tokio::select" || macro_name.contains("select")
        } else {
          false
        };

        if is_select {
          let start = node.start_position();
          let end = node.end_position();
          selects.push(ExtractedSelectBlock {
            source_symbol_qualified_name: caller_qname.clone(),
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

  if let Some(pushed) = scope_pushed.clone() {
    scope_stack.push(pushed);
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
      active_features,
    );
  }

  if scope_pushed.is_some() {
    scope_stack.pop();
  }
}

fn parse_to_tree(source: &str) -> Result<tree_sitter::Tree, String> {
  let mut parser = Parser::new();
  parser
    .set_language(&tree_sitter_rust::LANGUAGE.into())
    .map_err(|e| format!("failed to initialize rust parser: {e}"))?;
  parser
    .parse(source, None)
    .ok_or_else(|| "rust parsing was cancelled".to_string())
}

fn module_name_from_path(file_path: &str) -> String {
  let normalized = file_path.replace('\\', "/");
  let path_without_mod = if normalized.ends_with("/mod.rs") {
    &normalized[..normalized.len() - 7]
  } else if normalized == "mod.rs" {
    ""
  } else {
    normalized.strip_suffix(".rs").unwrap_or(&normalized)
  };
  path_without_mod.replace('/', "::")
}

fn get_type_name_text(node: Node<'_>, source: &str) -> String {
  let raw = node.utf8_text(source.as_bytes()).unwrap_or("").trim();
  let mut clean = raw;
  if clean.starts_with("->") {
    clean = clean[2..].trim();
  }
  while clean.starts_with('&') || clean.starts_with('*') {
    clean = clean[1..].trim();
  }
  if clean.starts_with("mut ") {
    clean = clean[4..].trim();
  }
  if clean.starts_with("const ") {
    clean = clean[6..].trim();
  }
  if let Some(pos) = clean.find('<') {
    clean = clean[..pos].trim();
  }
  if let Some(pos) = clean.rfind("::") {
    clean = clean[pos + 2..].trim();
  }
  clean.to_string()
}

fn build_qualified_name(
  module_name: &str,
  scope_stack: &[(String, bool)],
  symbol_name: &str,
) -> String {
  let mut parts = vec![module_name.to_string()];
  for (n, _) in scope_stack {
    parts.push(n.clone());
  }
  parts.push(symbol_name.to_string());
  parts.join("::")
}

fn get_current_enclosing_symbol(
  module_name: &str,
  scope_stack: &[(String, bool)],
) -> Option<String> {
  if scope_stack.is_empty() {
    None
  } else {
    let (last_name, _) = &scope_stack[scope_stack.len() - 1];
    Some(build_qualified_name(
      module_name,
      &scope_stack[..scope_stack.len() - 1],
      last_name,
    ))
  }
}

fn is_primitive_type(t: &str) -> bool {
  matches!(
    t,
    "i8"
      | "i16"
      | "i32"
      | "i64"
      | "i128"
      | "isize"
      | "u8"
      | "u16"
      | "u32"
      | "u64"
      | "u128"
      | "usize"
      | "f32"
      | "f64"
      | "str"
      | "String"
      | "char"
      | "bool"
      | "Self"
      | "self"
      | "Option"
      | "Result"
      | "Vec"
      | "HashMap"
      | "HashSet"
      | "BTreeMap"
      | "BTreeSet"
      | "Box"
      | "Rc"
      | "Arc"
      | "Cell"
      | "RefCell"
      | "std"
      | "core"
      | "alloc"
      | "dyn"
      | "impl"
      | "static"
      | "const"
      | "mut"
      | "ref"
      | "as"
      | "where"
      | "for"
      | "fn"
      | "trait"
      | "type"
      | "struct"
      | "enum"
      | "union"
      | "crate"
      | "super"
  )
}

fn extract_type_names_from_string(type_str: &str) -> Vec<String> {
  let mut clean_str = type_str.to_string();
  if clean_str.starts_with("->") {
    clean_str = clean_str[2..].to_string();
  }

  let mut words = Vec::new();
  let mut current_word = String::new();
  let mut in_lifetime = false;

  for c in clean_str.chars() {
    if in_lifetime {
      if c.is_alphanumeric() || c == '_' {
        continue;
      } else {
        in_lifetime = false;
      }
    }

    if c == '\'' {
      in_lifetime = true;
      if !current_word.is_empty() {
        words.push(current_word.clone());
        current_word.clear();
      }
      continue;
    }

    if c.is_alphanumeric() || c == '_' {
      current_word.push(c);
    } else {
      if !current_word.is_empty() {
        words.push(current_word.clone());
        current_word.clear();
      }
    }
  }
  if !current_word.is_empty() {
    words.push(current_word);
  }

  let mut result = Vec::new();
  for w in words {
    if !is_primitive_type(&w) && !result.contains(&w) {
      result.push(w);
    }
  }
  result
}

fn check_node_cfg(node: Node<'_>, source: &str, active_features: &[String]) -> bool {
  let attrs = get_attributes_for_node(node, source);
  for attr in attrs {
    let mut clean = attr.trim();
    if clean.starts_with("#[") && clean.ends_with(']') {
      clean = clean[2..clean.len() - 1].trim();
    }
    if clean.starts_with("cfg(") && clean.ends_with(')') {
      let inner = &clean[4..clean.len() - 1].trim();
      if inner.starts_with("feature = \"") && inner.ends_with('"') {
        let feature_name = &inner[11..inner.len() - 1];
        if !active_features.iter().any(|f| f == feature_name) {
          return false;
        }
      } else if inner.starts_with("not(feature = \"") && inner.ends_with("\")") {
        let feature_name = &inner[15..inner.len() - 2];
        if active_features.iter().any(|f| f == feature_name) {
          return false;
        }
      }
    }
  }
  true
}

pub fn extract_symbols_from_source(
  source: &str,
  file_path: &str,
  active_features: &[String],
) -> Result<Vec<Symbol>, String> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();

  let module_name = module_name_from_path(file_path);
  let mut symbols = Vec::new();

  let line_count = source.lines().count().max(1);
  let path = Path::new(file_path);
  let is_mod_rs = path
    .file_name()
    .and_then(|n| n.to_str())
    .is_some_and(|name| name == "mod.rs");

  let module_symbol_name = if is_mod_rs {
    path
      .parent()
      .and_then(|p| p.file_name())
      .and_then(|s| s.to_str())
      .unwrap_or("mod")
      .to_string()
  } else {
    path
      .file_stem()
      .and_then(|s| s.to_str())
      .unwrap_or(file_path)
      .to_string()
  };

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
    active_features,
  );

  Ok(symbols)
}

fn traverse(
  node: Node<'_>,
  source: &str,
  file_path: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, bool)>,
  symbols: &mut Vec<Symbol>,
  active_features: &[String],
) {
  if !check_node_cfg(node, source, active_features) {
    return;
  }
  let kind = node.kind();
  let mut scope_pushed = None;

  match kind {
    "mod_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let mod_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !mod_name.is_empty() {
          let qname = build_qualified_name(module_name, scope_stack, &mod_name);
          let start = node.start_position();
          let end = node.end_position();

          symbols.push(Symbol {
            name: mod_name.clone(),
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
          scope_pushed = Some((mod_name, false));
        }
      }
    }
    "struct_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let struct_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !struct_name.is_empty() {
          let qname = build_qualified_name(module_name, scope_stack, &struct_name);
          let start = node.start_position();
          let end = node.end_position();

          symbols.push(Symbol {
            name: struct_name.clone(),
            qualified_name: qname,
            kind: SymbolKind::Struct,
            location: Location {
              file_path: file_path.to_string(),
              start_line: start.row + 1,
              end_line: end.row + 1,
              start_column: Some(start.column),
              end_column: Some(end.column),
            },
          });
          scope_pushed = Some((struct_name, false));
        }
      }
    }
    "enum_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let enum_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !enum_name.is_empty() {
          let qname = build_qualified_name(module_name, scope_stack, &enum_name);
          let start = node.start_position();
          let end = node.end_position();

          symbols.push(Symbol {
            name: enum_name.clone(),
            qualified_name: qname,
            kind: SymbolKind::Enum,
            location: Location {
              file_path: file_path.to_string(),
              start_line: start.row + 1,
              end_line: end.row + 1,
              start_column: Some(start.column),
              end_column: Some(end.column),
            },
          });
          scope_pushed = Some((enum_name, false));
        }
      }
    }
    "trait_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let trait_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !trait_name.is_empty() {
          let qname = build_qualified_name(module_name, scope_stack, &trait_name);
          let start = node.start_position();
          let end = node.end_position();

          symbols.push(Symbol {
            name: trait_name.clone(),
            qualified_name: qname,
            kind: SymbolKind::Trait,
            location: Location {
              file_path: file_path.to_string(),
              start_line: start.row + 1,
              end_line: end.row + 1,
              start_column: Some(start.column),
              end_column: Some(end.column),
            },
          });
          scope_pushed = Some((trait_name, true));
        }
      }
    }
    "impl_item" => {
      if let Some(type_node) = node.child_by_field_name("type") {
        let type_name = get_type_name_text(type_node, source);
        if !type_name.is_empty() {
          if let Some(trait_node) = node.child_by_field_name("trait") {
            let trait_name = get_type_name_text(trait_node, source);
            let scope_name = format!("<{} as {}>", type_name, trait_name);
            scope_pushed = Some((scope_name, true));
          } else {
            scope_pushed = Some((type_name, true));
          }
        }
      }
    }
    "function_item" | "function_signature_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let func_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !func_name.is_empty() {
          let is_method = scope_stack
            .last()
            .map(|(_, is_container)| *is_container)
            .unwrap_or(false);
          let kind = if is_method {
            SymbolKind::Method
          } else {
            SymbolKind::Function
          };
          let qname = build_qualified_name(module_name, scope_stack, &func_name);
          let start = node.start_position();
          let end = node.end_position();

          symbols.push(Symbol {
            name: func_name.clone(),
            qualified_name: qname,
            kind,
            location: Location {
              file_path: file_path.to_string(),
              start_line: start.row + 1,
              end_line: end.row + 1,
              start_column: Some(start.column),
              end_column: Some(end.column),
            },
          });
          scope_pushed = Some((func_name, false));
        }
      }
    }
    _ => {}
  }

  if let Some(pushed) = scope_pushed.clone() {
    scope_stack.push(pushed);
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse(
      child,
      source,
      file_path,
      module_name,
      scope_stack,
      symbols,
      active_features,
    );
  }

  if scope_pushed.is_some() {
    scope_stack.pop();
  }
}

pub fn extract_imports_from_source(
  source: &str,
  _file_path: &str,
  active_features: &[String],
) -> Result<Vec<ExtractedImport>, String> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();
  let mut imports = Vec::new();
  find_use_declarations(root, source, &mut imports, active_features);
  Ok(imports)
}

fn find_use_declarations(
  node: Node<'_>,
  source: &str,
  imports: &mut Vec<ExtractedImport>,
  active_features: &[String],
) {
  if !check_node_cfg(node, source, active_features) {
    return;
  }
  if node.kind() == "use_declaration" {
    traverse_use_node(node, source, "", imports);
  } else {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
      find_use_declarations(child, source, imports, active_features);
    }
  }
}

fn traverse_use_node(
  node: Node<'_>,
  source: &str,
  prefix: &str,
  imports: &mut Vec<ExtractedImport>,
) {
  let kind = node.kind();
  match kind {
    "use_declaration" => {
      let mut cursor = node.walk();
      for child in node.children(&mut cursor) {
        if child.kind() == "use_clause"
          || child.kind() == "scoped_identifier"
          || child.kind() == "identifier"
          || child.kind() == "scoped_use_list"
          || child.kind() == "use_list"
        {
          traverse_use_node(child, source, prefix, imports);
        }
      }
    }
    "use_clause" => {
      let mut cursor = node.walk();
      for child in node.children(&mut cursor) {
        if child.is_named() {
          traverse_use_node(child, source, prefix, imports);
        }
      }
    }
    "scoped_use_list" => {
      if let Some(path_node) = node.child_by_field_name("path") {
        let path_text = path_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string();
        let new_prefix = if prefix.is_empty() {
          path_text
        } else {
          format!("{prefix}::{path_text}")
        };
        if let Some(list_node) = node.child_by_field_name("list") {
          traverse_use_node(list_node, source, &new_prefix, imports);
        }
      }
    }
    "use_list" => {
      let mut cursor = node.walk();
      for child in node.children(&mut cursor) {
        if child.is_named() {
          traverse_use_node(child, source, prefix, imports);
        }
      }
    }
    "use_as_clause" => {
      if let Some(path_node) = node.child_by_field_name("path") {
        traverse_use_node(path_node, source, prefix, imports);
      }
    }
    "use_wildcard" => {
      if let Some(path_node) = node.child_by_field_name("path") {
        let path_text = path_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .trim()
          .to_string();
        let qname = if prefix.is_empty() {
          format!("{path_text}::*")
        } else {
          format!("{prefix}::{path_text}::*")
        };
        let start = node.start_position();
        let end = node.end_position();
        imports.push(ExtractedImport {
          target_qualified_name: qname,
          start_line: start.row + 1,
          start_column: start.column,
          end_line: end.row + 1,
          end_column: end.column,
        });
      }
    }
    "scoped_identifier" | "identifier" => {
      let path_text = node
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .trim()
        .to_string();
      if !path_text.is_empty() {
        let qname = if prefix.is_empty() {
          path_text
        } else {
          format!("{prefix}::{path_text}")
        };
        let start = node.start_position();
        let end = node.end_position();
        imports.push(ExtractedImport {
          target_qualified_name: qname,
          start_line: start.row + 1,
          start_column: start.column,
          end_line: end.row + 1,
          end_column: end.column,
        });
      }
    }
    _ => {
      let mut cursor = node.walk();
      for child in node.children(&mut cursor) {
        traverse_use_node(child, source, prefix, imports);
      }
    }
  }
}

pub fn extract_implementations_from_source(
  source: &str,
  _file_path: &str,
  active_features: &[String],
) -> Result<Vec<ExtractedImplementation>, String> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();
  let mut implementations = Vec::new();
  traverse_implementations(root, source, &mut implementations, active_features);
  Ok(implementations)
}

fn parse_derive_traits(attr: &str) -> Vec<String> {
  let mut traits = Vec::new();
  let mut clean = attr.trim();
  if clean.starts_with("#[") && clean.ends_with(']') {
    clean = clean[2..clean.len() - 1].trim();
  }
  if clean.starts_with("derive(") && clean.ends_with(')') {
    let inner = &clean[7..clean.len() - 1];
    for part in inner.split(',') {
      let t = part.trim();
      if !t.is_empty() {
        traits.push(t.to_string());
      }
    }
  }
  traits
}

fn traverse_implementations(
  node: Node<'_>,
  source: &str,
  implementations: &mut Vec<ExtractedImplementation>,
  active_features: &[String],
) {
  if !check_node_cfg(node, source, active_features) {
    return;
  }
  if node.kind() == "impl_item" {
    if let Some(type_node) = node.child_by_field_name("type") {
      let struct_name = get_type_name_text(type_node, source);
      if !struct_name.is_empty() {
        let trait_name = node
          .child_by_field_name("trait")
          .map(|t| get_type_name_text(t, source));
        let start = node.start_position();
        let end = node.end_position();
        implementations.push(ExtractedImplementation {
          struct_name,
          trait_name,
          start_line: start.row + 1,
          start_column: start.column,
          end_line: end.row + 1,
          end_column: end.column,
        });
      }
    }
  }

  if node.kind() == "struct_item" || node.kind() == "enum_item" {
    if let Some(name_node) = node.child_by_field_name("name") {
      let name = name_node
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .trim()
        .to_string();
      if !name.is_empty() {
        let attrs = get_attributes_for_node(node, source);
        for attr in attrs {
          for derived_trait in parse_derive_traits(&attr) {
            let start = node.start_position();
            let end = node.end_position();

            let mapped_traits = match derived_trait.as_str() {
              "Serialize" => vec!["serde::Serialize".to_string()],
              "Deserialize" => vec!["serde::Deserialize".to_string()],
              "Error" => vec![
                "std::error::Error".to_string(),
                "std::fmt::Display".to_string(),
              ],
              "Parser" => vec!["clap::Parser".to_string()],
              "Args" => vec!["clap::Args".to_string()],
              "Subcommand" => vec!["clap::Subcommand".to_string()],
              "ValueEnum" => vec!["clap::ValueEnum".to_string()],
              other => vec![other.to_string()],
            };

            for t_name in mapped_traits {
              implementations.push(ExtractedImplementation {
                struct_name: name.clone(),
                trait_name: Some(t_name),
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

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_implementations(child, source, implementations, active_features);
  }
}

pub fn extract_calls_from_source(
  source: &str,
  file_path: &str,
  active_features: &[String],
) -> Result<Vec<ExtractedCall>, String> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();
  let module_name = module_name_from_path(file_path);
  let mut scope_stack = Vec::new();
  let mut calls = Vec::new();
  traverse_calls(
    root,
    source,
    &module_name,
    &mut scope_stack,
    &mut calls,
    active_features,
  );
  Ok(calls)
}

fn traverse_calls(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, bool)>,
  calls: &mut Vec<ExtractedCall>,
  active_features: &[String],
) {
  if !check_node_cfg(node, source, active_features) {
    return;
  }
  let kind = node.kind();
  let mut scope_pushed = None;

  match kind {
    "mod_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let mod_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !mod_name.is_empty() {
          scope_pushed = Some((mod_name, false));
        }
      }
    }
    "struct_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let struct_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !struct_name.is_empty() {
          scope_pushed = Some((struct_name, false));
        }
      }
    }
    "enum_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let enum_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !enum_name.is_empty() {
          scope_pushed = Some((enum_name, false));
        }
      }
    }
    "trait_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let trait_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !trait_name.is_empty() {
          scope_pushed = Some((trait_name, true));
        }
      }
    }
    "impl_item" => {
      if let Some(type_node) = node.child_by_field_name("type") {
        let type_name = get_type_name_text(type_node, source);
        if !type_name.is_empty() {
          if let Some(trait_node) = node.child_by_field_name("trait") {
            let trait_name = get_type_name_text(trait_node, source);
            let scope_name = format!("<{} as {}>", type_name, trait_name);
            scope_pushed = Some((scope_name, true));
          } else {
            scope_pushed = Some((type_name, true));
          }
        }
      }
    }
    "function_item" | "function_signature_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let func_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !func_name.is_empty() {
          scope_pushed = Some((func_name, false));
        }
      }
    }
    "call_expression" => {
      if let Some(caller_qname) = get_current_enclosing_symbol(module_name, scope_stack) {
        if let Some(func_node) = node.child_by_field_name("function") {
          if func_node.kind() == "field_expression" {
            if let Some(field_node) = func_node.child_by_field_name("field") {
              let method_name = field_node
                .utf8_text(source.as_bytes())
                .unwrap_or("")
                .trim()
                .to_string();
              if !method_name.is_empty() {
                let receiver_text = func_node
                  .child_by_field_name("value")
                  .map(|v| v.utf8_text(source.as_bytes()).unwrap_or("").trim())
                  .unwrap_or("");
                let is_self_call = receiver_text == "self";
                let target_name = if is_self_call {
                  format!("self.{method_name}")
                } else {
                  method_name.clone()
                };
                let start = node.start_position();
                let end = node.end_position();
                calls.push(ExtractedCall {
                  source_symbol_qualified_name: caller_qname,
                  target_name,
                  is_self_call,
                  method_name: Some(method_name),
                  start_line: start.row + 1,
                  start_column: start.column,
                  end_line: end.row + 1,
                  end_column: end.column,
                });
              }
            }
          } else {
            let target_name = func_node
              .utf8_text(source.as_bytes())
              .unwrap_or("")
              .trim()
              .to_string();
            if !target_name.is_empty() {
              let start = node.start_position();
              let end = node.end_position();
              calls.push(ExtractedCall {
                source_symbol_qualified_name: caller_qname,
                target_name,
                is_self_call: false,
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
    "method_call_expression" => {
      if let Some(caller_qname) = get_current_enclosing_symbol(module_name, scope_stack) {
        if let Some(name_node) = node.child_by_field_name("name") {
          let method_name = name_node
            .utf8_text(source.as_bytes())
            .unwrap_or("")
            .trim()
            .to_string();
          if !method_name.is_empty() {
            let receiver_text = node
              .child_by_field_name("value")
              .map(|v| v.utf8_text(source.as_bytes()).unwrap_or("").trim())
              .unwrap_or("");
            let is_self_call = receiver_text == "self";
            let target_name = if is_self_call {
              format!("self.{method_name}")
            } else {
              method_name.clone()
            };
            let start = node.start_position();
            let end = node.end_position();
            calls.push(ExtractedCall {
              source_symbol_qualified_name: caller_qname,
              target_name,
              is_self_call,
              method_name: Some(method_name),
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

  if let Some(pushed) = scope_pushed.clone() {
    scope_stack.push(pushed);
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_calls(
      child,
      source,
      module_name,
      scope_stack,
      calls,
      active_features,
    );
  }

  if scope_pushed.is_some() {
    scope_stack.pop();
  }
}

pub fn extract_type_references_from_source(
  source: &str,
  file_path: &str,
  active_features: &[String],
) -> Result<Vec<ExtractedTypeReference>, String> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();
  let module_name = module_name_from_path(file_path);
  let mut scope_stack = Vec::new();
  let mut type_refs = Vec::new();
  traverse_type_references(
    root,
    source,
    &module_name,
    &mut scope_stack,
    &mut type_refs,
    active_features,
  );
  Ok(type_refs)
}

fn traverse_type_references(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, bool)>,
  type_refs: &mut Vec<ExtractedTypeReference>,
  active_features: &[String],
) {
  if !check_node_cfg(node, source, active_features) {
    return;
  }
  let kind = node.kind();
  let mut scope_pushed = None;

  match kind {
    "mod_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let mod_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !mod_name.is_empty() {
          scope_pushed = Some((mod_name, false));
        }
      }
    }
    "struct_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let struct_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !struct_name.is_empty() {
          scope_pushed = Some((struct_name, false));
        }
      }
    }
    "enum_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let enum_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !enum_name.is_empty() {
          scope_pushed = Some((enum_name, false));
        }
      }
    }
    "trait_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let trait_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !trait_name.is_empty() {
          scope_pushed = Some((trait_name, true));
        }
      }
    }
    "impl_item" => {
      if let Some(type_node) = node.child_by_field_name("type") {
        let type_name = get_type_name_text(type_node, source);
        if !type_name.is_empty() {
          if let Some(trait_node) = node.child_by_field_name("trait") {
            let trait_name = get_type_name_text(trait_node, source);
            let scope_name = format!("<{} as {}>", type_name, trait_name);
            scope_pushed = Some((scope_name, true));
          } else {
            scope_pushed = Some((type_name, true));
          }
        }
      }
    }
    "function_item" | "function_signature_item" => {
      if let Some(ret_node) = node.child_by_field_name("return_type") {
        let type_str = ret_node.utf8_text(source.as_bytes()).unwrap_or("");
        let extracted_types = extract_type_names_from_string(type_str);
        if !extracted_types.is_empty() {
          if let Some(name_node) = node.child_by_field_name("name") {
            let func_name = name_node
              .utf8_text(source.as_bytes())
              .unwrap_or("")
              .trim()
              .to_string();
            if !func_name.is_empty() {
              let caller_qname = build_qualified_name(module_name, scope_stack, &func_name);
              let start = ret_node.start_position();
              let end = ret_node.end_position();
              for type_name in extracted_types {
                type_refs.push(ExtractedTypeReference {
                  source_symbol_qualified_name: caller_qname.clone(),
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
      if let Some(name_node) = node.child_by_field_name("name") {
        let func_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !func_name.is_empty() {
          scope_pushed = Some((func_name, false));
        }
      }
    }
    _ => {}
  }

  match kind {
    "parameter" => {
      if let Some(type_node) = node.child_by_field_name("type") {
        let type_str = type_node.utf8_text(source.as_bytes()).unwrap_or("");
        let extracted_types = extract_type_names_from_string(type_str);
        if !extracted_types.is_empty() {
          if let Some(caller_qname) = get_current_enclosing_symbol(module_name, scope_stack) {
            let start = type_node.start_position();
            let end = type_node.end_position();
            for type_name in extracted_types {
              type_refs.push(ExtractedTypeReference {
                source_symbol_qualified_name: caller_qname.clone(),
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
    "field_declaration" => {
      if let Some(type_node) = node.child_by_field_name("type") {
        let type_str = type_node.utf8_text(source.as_bytes()).unwrap_or("");
        let extracted_types = extract_type_names_from_string(type_str);
        if !extracted_types.is_empty() {
          if let Some(caller_qname) = get_current_enclosing_symbol(module_name, scope_stack) {
            let start = type_node.start_position();
            let end = type_node.end_position();
            for type_name in extracted_types {
              type_refs.push(ExtractedTypeReference {
                source_symbol_qualified_name: caller_qname.clone(),
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

  if let Some(pushed) = scope_pushed.clone() {
    scope_stack.push(pushed);
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_type_references(
      child,
      source,
      module_name,
      scope_stack,
      type_refs,
      active_features,
    );
  }

  if scope_pushed.is_some() {
    scope_stack.pop();
  }
}

pub fn extract_tests_from_source(
  source: &str,
  file_path: &str,
  active_features: &[String],
) -> Result<Vec<ExtractedTest>, String> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();
  let module_name = module_name_from_path(file_path);
  let mut scope_stack = Vec::new();
  let mut tests = Vec::new();
  traverse_tests(
    root,
    source,
    &module_name,
    &mut scope_stack,
    &mut tests,
    active_features,
  );
  Ok(tests)
}

fn traverse_tests(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, bool)>,
  tests: &mut Vec<ExtractedTest>,
  active_features: &[String],
) {
  if !check_node_cfg(node, source, active_features) {
    return;
  }
  let kind = node.kind();
  let mut scope_pushed = None;

  match kind {
    "mod_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let mod_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !mod_name.is_empty() {
          let is_test = is_test_module(node, source);
          if is_test {
            let qname = build_qualified_name(module_name, scope_stack, &mod_name);
            let start = node.start_position();
            let end = node.end_position();
            tests.push(ExtractedTest {
              name: mod_name.clone(),
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
          scope_pushed = Some((mod_name, false));
        }
      }
    }
    "struct_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let struct_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !struct_name.is_empty() {
          scope_pushed = Some((struct_name, false));
        }
      }
    }
    "enum_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let enum_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !enum_name.is_empty() {
          scope_pushed = Some((enum_name, false));
        }
      }
    }
    "trait_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let trait_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !trait_name.is_empty() {
          scope_pushed = Some((trait_name, true));
        }
      }
    }
    "impl_item" => {
      if let Some(type_node) = node.child_by_field_name("type") {
        let type_name = get_type_name_text(type_node, source);
        if !type_name.is_empty() {
          if let Some(trait_node) = node.child_by_field_name("trait") {
            let trait_name = get_type_name_text(trait_node, source);
            let scope_name = format!("<{} as {}>", type_name, trait_name);
            scope_pushed = Some((scope_name, true));
          } else {
            scope_pushed = Some((type_name, true));
          }
        }
      }
    }
    "function_item" | "function_signature_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let func_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !func_name.is_empty() {
          if has_test_attribute(node, source) {
            let qname = build_qualified_name(module_name, scope_stack, &func_name);
            let start = node.start_position();
            let end = node.end_position();
            let is_parametrized = is_parametrized_test(node, source);
            let parameters = get_parameter_names(node, source);

            tests.push(ExtractedTest {
              name: func_name.clone(),
              qualified_name: qname,
              kind: ExtractedTestKind::Function,
              is_parametrized,
              parameters,
              start_line: start.row + 1,
              start_column: start.column,
              end_line: end.row + 1,
              end_column: end.column,
            });
          }
          scope_pushed = Some((func_name, false));
        }
      }
    }
    _ => {}
  }

  if let Some(pushed) = scope_pushed.clone() {
    scope_stack.push(pushed);
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_tests(
      child,
      source,
      module_name,
      scope_stack,
      tests,
      active_features,
    );
  }

  if scope_pushed.is_some() {
    scope_stack.pop();
  }
}

fn has_test_attribute(node: Node<'_>, source: &str) -> bool {
  let attrs = get_attributes_for_node(node, source);
  for attr in attrs {
    if is_actual_test_attribute(&attr) {
      return true;
    }
  }
  false
}

fn is_test_module(node: Node<'_>, source: &str) -> bool {
  if let Some(name_node) = node.child_by_field_name("name") {
    let name = name_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
    if name.starts_with("test") || name.ends_with("test") || name.ends_with("tests") {
      return true;
    }
  }

  let attrs = get_attributes_for_node(node, source);
  for attr in attrs {
    if attr.contains("cfg(test)") || attr.contains("test") {
      return true;
    }
  }
  false
}

fn is_parametrized_test(node: Node<'_>, source: &str) -> bool {
  let attrs = get_attributes_for_node(node, source);
  for attr in attrs {
    let mut clean = attr.trim();
    if clean.starts_with("#[") && clean.ends_with(']') {
      clean = &clean[2..clean.len() - 1];
    }
    let name = if let Some(pos) = clean.find('(') {
      clean[..pos].trim()
    } else {
      clean.trim()
    };
    if name == "test_case"
      || name.ends_with("::test_case")
      || name == "rstest"
      || name.ends_with("::rstest")
    {
      return true;
    }
  }
  false
}

fn get_parameter_names(func_node: Node<'_>, source: &str) -> Vec<String> {
  let mut params = Vec::new();
  if let Some(params_node) = func_node.child_by_field_name("parameters") {
    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
      if child.kind() == "parameter" {
        if let Some(pattern_node) = child.child_by_field_name("pattern") {
          let name = pattern_node
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

fn get_attributes_for_node(node: Node<'_>, source: &str) -> Vec<String> {
  let mut attrs = Vec::new();
  let mut current = node.prev_sibling();
  while let Some(sibling) = current {
    let kind = sibling.kind();
    if kind == "attribute_item" {
      let text = sibling.utf8_text(source.as_bytes()).unwrap_or("").trim();
      attrs.push(text.to_string());
      current = sibling.prev_sibling();
    } else if kind == "line_comment" || kind == "block_comment" || kind == "comment" {
      current = sibling.prev_sibling();
    } else {
      break;
    }
  }
  attrs
}

fn is_actual_test_attribute(attr_text: &str) -> bool {
  let mut clean = attr_text.trim();
  if clean.starts_with("#[") && clean.ends_with(']') {
    clean = &clean[2..clean.len() - 1];
  }

  let name = if let Some(pos) = clean.find('(') {
    clean[..pos].trim()
  } else {
    clean.trim()
  };

  name == "test"
    || name.ends_with("::test")
    || name == "rstest"
    || name.ends_with("::rstest")
    || name == "test_case"
    || name.ends_with("::test_case")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedUnsafeBlock {
  pub source_symbol_qualified_name: String,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedUnsafeFunction {
  pub qualified_name: String,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedFFIBinding {
  pub source_symbol_qualified_name: String,
  pub foreign_item_name: String,
  pub abi: String,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedSafetyMetadata {
  pub unsafe_blocks: Vec<ExtractedUnsafeBlock>,
  pub unsafe_functions: Vec<ExtractedUnsafeFunction>,
  pub ffi_bindings: Vec<ExtractedFFIBinding>,
}

pub fn extract_safety_metadata_from_source(
  source: &str,
  file_path: &str,
  active_features: &[String],
) -> Result<ExtractedSafetyMetadata, String> {
  let tree = parse_to_tree(source)?;
  let root = tree.root_node();
  let module_name = module_name_from_path(file_path);
  let mut scope_stack = Vec::new();

  let mut unsafe_blocks = Vec::new();
  let mut unsafe_functions = Vec::new();
  let mut ffi_bindings = Vec::new();

  traverse_safety(
    root,
    source,
    &module_name,
    &mut scope_stack,
    &mut unsafe_blocks,
    &mut unsafe_functions,
    &mut ffi_bindings,
    active_features,
  );

  Ok(ExtractedSafetyMetadata {
    unsafe_blocks,
    unsafe_functions,
    ffi_bindings,
  })
}

fn extract_foreign_items(
  node: Node<'_>,
  source: &str,
  abi: &str,
  enclosing_symbol: &str,
  ffi_bindings: &mut Vec<ExtractedFFIBinding>,
) {
  let kind = node.kind();
  if kind == "function_signature_item"
    || kind == "foreign_function_signature"
    || kind == "function_item"
    || kind == "static_item"
    || kind == "type_alias_item"
    || kind == "type_item"
    || kind == "associated_type"
  {
    if let Some(name_node) = node.child_by_field_name("name") {
      let name = name_node
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .to_string();
      if !name.is_empty() {
        let start = node.start_position();
        let end = node.end_position();
        ffi_bindings.push(ExtractedFFIBinding {
          source_symbol_qualified_name: enclosing_symbol.to_string(),
          foreign_item_name: name,
          abi: abi.to_string(),
          start_line: start.row + 1,
          start_column: start.column,
          end_line: end.row + 1,
          end_column: end.column,
        });
      }
    }
    return;
  }
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    extract_foreign_items(child, source, abi, enclosing_symbol, ffi_bindings);
  }
}

fn node_has_unsafe_modifier(node: Node<'_>) -> bool {
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    let ck = child.kind();
    if ck == "unsafe" {
      return true;
    }
    if ck == "function_modifiers" || ck == "impl_modifiers" || ck == "trait_modifiers" {
      let mut c2 = child.walk();
      for grandchild in child.children(&mut c2) {
        if grandchild.kind() == "unsafe" {
          return true;
        }
      }
    }
  }
  false
}

fn extract_abi_from_foreign_mod(node: Node<'_>, source: &str) -> String {
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    if child.kind() == "string_literal" {
      return child
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .trim()
        .replace('"', "");
    }
    if child.kind() == "abi" {
      return child
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .trim()
        .replace('"', "");
    }
  }
  "C".to_string() // default in Rust
}

#[allow(clippy::too_many_arguments)]
fn traverse_safety(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, bool)>,
  unsafe_blocks: &mut Vec<ExtractedUnsafeBlock>,
  unsafe_functions: &mut Vec<ExtractedUnsafeFunction>,
  ffi_bindings: &mut Vec<ExtractedFFIBinding>,
  active_features: &[String],
) {
  if !check_node_cfg(node, source, active_features) {
    return;
  }
  let kind = node.kind();
  let mut scope_pushed = None;

  let is_unsafe = node_has_unsafe_modifier(node);

  match kind {
    "mod_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let mod_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !mod_name.is_empty() {
          let attrs = get_attributes_for_node(node, source);
          let is_cxx = attrs
            .iter()
            .any(|a| a.contains("cxx::bridge") || a.contains("cxx_bridge"));
          if is_cxx {
            let start = node.start_position();
            let end = node.end_position();
            ffi_bindings.push(ExtractedFFIBinding {
              source_symbol_qualified_name: get_current_enclosing_symbol(module_name, scope_stack)
                .unwrap_or_else(|| module_name.to_string()),
              foreign_item_name: mod_name.clone(),
              abi: "cxx".to_string(),
              start_line: start.row + 1,
              start_column: start.column,
              end_line: end.row + 1,
              end_column: end.column,
            });
          }
          scope_pushed = Some((mod_name, false));
        }
      }
    }
    "struct_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let struct_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !struct_name.is_empty() {
          scope_pushed = Some((struct_name, false));
        }
      }
    }
    "enum_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let enum_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !enum_name.is_empty() {
          scope_pushed = Some((enum_name, false));
        }
      }
    }
    "trait_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let trait_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !trait_name.is_empty() {
          if is_unsafe {
            let qname = build_qualified_name(module_name, scope_stack, &trait_name);
            let start = node.start_position();
            let end = node.end_position();
            unsafe_functions.push(ExtractedUnsafeFunction {
              qualified_name: qname,
              start_line: start.row + 1,
              start_column: start.column,
              end_line: end.row + 1,
              end_column: end.column,
            });
          }
          scope_pushed = Some((trait_name, true));
        }
      }
    }
    "impl_item" => {
      if let Some(type_node) = node.child_by_field_name("type") {
        let type_name = get_type_name_text(type_node, source);
        if !type_name.is_empty() {
          let scope_name = if let Some(trait_node) = node.child_by_field_name("trait") {
            let trait_name = get_type_name_text(trait_node, source);
            format!("<{} as {}>", type_name, trait_name)
          } else {
            type_name
          };
          if is_unsafe {
            let qname = build_qualified_name(module_name, scope_stack, &scope_name);
            let start = node.start_position();
            let end = node.end_position();
            unsafe_functions.push(ExtractedUnsafeFunction {
              qualified_name: qname,
              start_line: start.row + 1,
              start_column: start.column,
              end_line: end.row + 1,
              end_column: end.column,
            });
          }
          scope_pushed = Some((scope_name, true));
        }
      }
    }
    "function_item" | "function_signature_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let func_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !func_name.is_empty() {
          let qname = build_qualified_name(module_name, scope_stack, &func_name);
          if is_unsafe {
            let start = node.start_position();
            let end = node.end_position();
            unsafe_functions.push(ExtractedUnsafeFunction {
              qualified_name: qname.clone(),
              start_line: start.row + 1,
              start_column: start.column,
              end_line: end.row + 1,
              end_column: end.column,
            });
          }
          scope_pushed = Some((func_name, false));
        }
      }
    }
    "unsafe_block" => {
      let enclosing = get_current_enclosing_symbol(module_name, scope_stack)
        .unwrap_or_else(|| module_name.to_string());
      let start = node.start_position();
      let end = node.end_position();
      unsafe_blocks.push(ExtractedUnsafeBlock {
        source_symbol_qualified_name: enclosing,
        start_line: start.row + 1,
        start_column: start.column,
        end_line: end.row + 1,
        end_column: end.column,
      });
    }
    "extern_block" | "foreign_mod_item" => {
      let enclosing = get_current_enclosing_symbol(module_name, scope_stack)
        .unwrap_or_else(|| module_name.to_string());
      let abi = extract_abi_from_foreign_mod(node, source);
      extract_foreign_items(node, source, &abi, &enclosing, ffi_bindings);
      return; // Do not recurse into extern blocks
    }
    _ => {}
  }

  if let Some(pushed) = scope_pushed.clone() {
    scope_stack.push(pushed);
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    traverse_safety(
      child,
      source,
      module_name,
      scope_stack,
      unsafe_blocks,
      unsafe_functions,
      ffi_bindings,
      active_features,
    );
  }

  if scope_pushed.is_some() {
    scope_stack.pop();
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedConcurrencyEdge {
  pub source_symbol_qualified_name: String,
  pub target_name: String,
  pub kind: String,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct SpawnData {
  source_symbol: String,
  target_name: String,
  start_line: usize,
  start_column: usize,
  end_line: usize,
  end_column: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct ChannelUsageData {
  source_symbol: String,
  var_name: String,
  start_line: usize,
  start_column: usize,
  end_line: usize,
  end_column: usize,
}

pub fn extract_concurrency_from_source(
  source: &str,
  file_path: &str,
  active_features: &[String],
) -> Result<Vec<ExtractedConcurrencyEdge>, String> {
  let mut parser = Parser::new();
  parser
    .set_language(&tree_sitter_rust::LANGUAGE.into())
    .map_err(|e| e.to_string())?;

  let tree = parser
    .parse(source, None)
    .ok_or_else(|| "failed to parse source".to_string())?;

  let root = tree.root_node();
  let module_name = module_name_from_path(file_path);

  let mut scope_stack = Vec::new();
  let mut spawns = Vec::new();
  let mut channel_decls = Vec::new();
  let mut sends = Vec::new();
  let mut recvs = Vec::new();

  collect_concurrency_data(
    root,
    source,
    &module_name,
    &mut scope_stack,
    &mut spawns,
    &mut channel_decls,
    &mut sends,
    &mut recvs,
    active_features,
  );

  let mut unique_edges = Vec::new();

  // 1. Process Spawns
  for spawn in &spawns {
    let edge = ExtractedConcurrencyEdge {
      source_symbol_qualified_name: spawn.source_symbol.clone(),
      target_name: spawn.target_name.clone(),
      kind: "Spawns".to_string(),
      start_line: spawn.start_line,
      start_column: spawn.start_column,
      end_line: spawn.end_line,
      end_column: spawn.end_column,
    };
    if !unique_edges.contains(&edge) {
      unique_edges.push(edge);
    }
  }

  // 2. Process channel pairings
  for (t_var, r_var) in &channel_decls {
    let mut channel_senders = Vec::new();
    let mut channel_receivers = Vec::new();

    for send in &sends {
      if send.var_name == *t_var
        || send.var_name.starts_with(&format!("{t_var}."))
        || send.var_name.starts_with(&format!("{t_var}_"))
        || send.var_name.ends_with(&format!(".{t_var}"))
      {
        if !channel_senders.contains(send) {
          channel_senders.push(send.clone());
        }
      }
    }

    for recv in &recvs {
      if recv.var_name == *r_var
        || recv.var_name.starts_with(&format!("{r_var}."))
        || recv.var_name.starts_with(&format!("{r_var}_"))
        || recv.var_name.ends_with(&format!(".{r_var}"))
      {
        if !channel_receivers.contains(recv) {
          channel_receivers.push(recv.clone());
        }
      }
    }

    for s in &channel_senders {
      for r in &channel_receivers {
        if s.source_symbol != r.source_symbol {
          let edge = ExtractedConcurrencyEdge {
            source_symbol_qualified_name: s.source_symbol.clone(),
            target_name: r.source_symbol.clone(),
            kind: "SendsTo".to_string(),
            start_line: s.start_line,
            start_column: s.start_column,
            end_line: r.end_line,
            end_column: r.end_column,
          };
          if !unique_edges.contains(&edge) {
            unique_edges.push(edge);
          }
        }
      }
    }
  }

  // 3. Fallback heuristic for channels without explicit decls
  let paired_local_transmitters: std::collections::HashSet<String> =
    channel_decls.iter().map(|(t, _)| t.clone()).collect();

  for send in &sends {
    if paired_local_transmitters.contains(&send.var_name) {
      continue;
    }
    let is_likely_sender = send.var_name.contains("tx")
      || send.var_name.contains("sender")
      || send.var_name.ends_with(".tx");
    if !is_likely_sender {
      continue;
    }

    for recv in &recvs {
      let is_likely_receiver = recv.var_name.contains("rx")
        || recv.var_name.contains("receiver")
        || recv.var_name.ends_with(".rx");
      if !is_likely_receiver {
        continue;
      }

      if send.source_symbol != recv.source_symbol {
        let edge = ExtractedConcurrencyEdge {
          source_symbol_qualified_name: send.source_symbol.clone(),
          target_name: recv.source_symbol.clone(),
          kind: "SendsTo".to_string(),
          start_line: send.start_line,
          start_column: send.start_column,
          end_line: recv.end_line,
          end_column: recv.end_column,
        };
        if !unique_edges.contains(&edge) {
          unique_edges.push(edge);
        }
      }
    }
  }

  Ok(unique_edges)
}

#[allow(clippy::too_many_arguments)]
fn collect_concurrency_data(
  node: Node<'_>,
  source: &str,
  module_name: &str,
  scope_stack: &mut Vec<(String, bool)>,
  spawns: &mut Vec<SpawnData>,
  channel_decls: &mut Vec<(String, String)>,
  sends: &mut Vec<ChannelUsageData>,
  recvs: &mut Vec<ChannelUsageData>,
  active_features: &[String],
) {
  if !check_node_cfg(node, source, active_features) {
    return;
  }
  let kind = node.kind();
  let mut scope_pushed = None;

  match kind {
    "mod_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let mod_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !mod_name.is_empty() {
          scope_pushed = Some((mod_name, false));
        }
      }
    }
    "struct_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let struct_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !struct_name.is_empty() {
          scope_pushed = Some((struct_name, false));
        }
      }
    }
    "enum_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let enum_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !enum_name.is_empty() {
          scope_pushed = Some((enum_name, false));
        }
      }
    }
    "trait_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let trait_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !trait_name.is_empty() {
          scope_pushed = Some((trait_name, true));
        }
      }
    }
    "impl_item" => {
      if let Some(type_node) = node.child_by_field_name("type") {
        let type_name = get_type_name_text(type_node, source);
        if !type_name.is_empty() {
          if let Some(trait_node) = node.child_by_field_name("trait") {
            let trait_name = get_type_name_text(trait_node, source);
            let scope_name = format!("<{} as {}>", type_name, trait_name);
            scope_pushed = Some((scope_name, true));
          } else {
            scope_pushed = Some((type_name, true));
          }
        }
      }
    }
    "function_item" | "function_signature_item" => {
      if let Some(name_node) = node.child_by_field_name("name") {
        let func_name = name_node
          .utf8_text(source.as_bytes())
          .unwrap_or("")
          .to_string();
        if !func_name.is_empty() {
          scope_pushed = Some((func_name, false));
        }
      }
    }
    "let_declaration" => {
      let mut has_channel = false;
      let text = node.utf8_text(source.as_bytes()).unwrap_or("");
      if text.contains("channel")
        || text.contains("mpsc")
        || text.contains("oneshot")
        || text.contains("broadcast")
      {
        has_channel = true;
      }
      if has_channel {
        if let Some(pattern) = node.child_by_field_name("pattern") {
          if pattern.kind() == "tuple_pattern" {
            let mut idents = Vec::new();
            collect_identifiers(pattern, source, &mut idents);
            if idents.len() == 2 {
              channel_decls.push((idents[0].clone(), idents[1].clone()));
            }
          }
        }
      }
    }
    "call_expression" => {
      if let Some(caller_qname) = get_current_enclosing_symbol(module_name, scope_stack) {
        if let Some(func_node) = node.child_by_field_name("function") {
          let func_text = func_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
          if func_text == "tokio::spawn"
            || func_text == "spawn"
            || func_text == "std::thread::spawn"
            || func_text == "thread::spawn"
          {
            let mut spawn_targets = Vec::new();
            if let Some(args_node) = node.child_by_field_name("arguments") {
              extract_spawned_calls(args_node, source, &mut spawn_targets);
            }
            let start = node.start_position();
            let end = node.end_position();
            for target in spawn_targets {
              spawns.push(SpawnData {
                source_symbol: caller_qname.clone(),
                target_name: target,
                start_line: start.row + 1,
                start_column: start.column,
                end_line: end.row + 1,
                end_column: end.column,
              });
            }
          } else if let Some(field_node) = func_node.child_by_field_name("field") {
            let method_name = field_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
            if let Some(value_node) = func_node.child_by_field_name("value") {
              let receiver_name = value_node
                .utf8_text(source.as_bytes())
                .unwrap_or("")
                .trim()
                .to_string();
              let start = node.start_position();
              let end = node.end_position();

              if method_name == "send" || method_name == "send_timeout" {
                sends.push(ChannelUsageData {
                  source_symbol: caller_qname.clone(),
                  var_name: receiver_name,
                  start_line: start.row + 1,
                  start_column: start.column,
                  end_line: end.row + 1,
                  end_column: end.column,
                });
              } else if method_name == "recv"
                || method_name == "recv_async"
                || method_name == "recv_timeout"
                || method_name == "next"
              {
                recvs.push(ChannelUsageData {
                  source_symbol: caller_qname.clone(),
                  var_name: receiver_name,
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
    "method_call_expression" => {
      if let Some(caller_qname) = get_current_enclosing_symbol(module_name, scope_stack) {
        if let Some(name_node) = node.child_by_field_name("name") {
          let method_name = name_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
          if let Some(value_node) = node.child_by_field_name("value") {
            let receiver_name = value_node
              .utf8_text(source.as_bytes())
              .unwrap_or("")
              .trim()
              .to_string();
            let start = node.start_position();
            let end = node.end_position();

            if method_name == "send" || method_name == "send_timeout" {
              sends.push(ChannelUsageData {
                source_symbol: caller_qname,
                var_name: receiver_name,
                start_line: start.row + 1,
                start_column: start.column,
                end_line: end.row + 1,
                end_column: end.column,
              });
            } else if method_name == "recv"
              || method_name == "recv_async"
              || method_name == "recv_timeout"
              || method_name == "next"
            {
              recvs.push(ChannelUsageData {
                source_symbol: caller_qname,
                var_name: receiver_name,
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
    "macro_invocation" => {
      if let Some(caller_qname) = get_current_enclosing_symbol(module_name, scope_stack) {
        if let Some(macro_node) = node.child_by_field_name("macro") {
          let macro_text = macro_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
          if macro_text == "select" || macro_text == "tokio::select" {
            let start = node.start_position();
            let end = node.end_position();
            let idents = get_all_identifiers_in_node(node, source);
            for ident in idents {
              if ident.contains("rx") || ident.contains("receiver") {
                recvs.push(ChannelUsageData {
                  source_symbol: caller_qname.clone(),
                  var_name: ident,
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
    _ => {}
  }

  if let Some(pushed) = scope_pushed.clone() {
    scope_stack.push(pushed);
  }

  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    collect_concurrency_data(
      child,
      source,
      module_name,
      scope_stack,
      spawns,
      channel_decls,
      sends,
      recvs,
      active_features,
    );
  }

  if scope_pushed.is_some() {
    scope_stack.pop();
  }
}

fn get_all_identifiers_in_node(node: Node<'_>, source: &str) -> Vec<String> {
  let mut idents = Vec::new();
  collect_identifiers(node, source, &mut idents);
  idents
}

fn collect_identifiers(node: Node<'_>, source: &str, idents: &mut Vec<String>) {
  if node.kind() == "identifier" {
    let text = node
      .utf8_text(source.as_bytes())
      .unwrap_or("")
      .trim()
      .to_string();
    if !text.is_empty() && !idents.contains(&text) {
      idents.push(text);
    }
  }
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    collect_identifiers(child, source, idents);
  }
}

fn extract_spawned_calls(node: Node<'_>, source: &str, targets: &mut Vec<String>) {
  extract_spawned_calls_inner(node, source, targets, true);
}

fn extract_spawned_calls_inner(
  node: Node<'_>,
  source: &str,
  targets: &mut Vec<String>,
  is_top_level: bool,
) {
  let kind = node.kind();
  match kind {
    "call_expression" => {
      if let Some(func_node) = node.child_by_field_name("function") {
        let func_text = func_node.utf8_text(source.as_bytes()).unwrap_or("").trim();
        if !func_text.is_empty() && func_text != "tokio::spawn" && func_text != "spawn" {
          let mut clean = func_text;
          if let Some(pos) = clean.rfind("::") {
            clean = &clean[pos + 2..];
          }
          if !targets.contains(&clean.to_string()) {
            targets.push(clean.to_string());
          }
        }
      }
    }
    "identifier" | "scoped_identifier" => {
      if is_top_level {
        let func_text = node.utf8_text(source.as_bytes()).unwrap_or("").trim();
        if !func_text.is_empty() && func_text != "tokio::spawn" && func_text != "spawn" {
          let mut clean = func_text;
          if let Some(pos) = clean.rfind("::") {
            clean = &clean[pos + 2..];
          }
          if !targets.contains(&clean.to_string()) {
            targets.push(clean.to_string());
          }
        }
      }
    }
    "arguments" => {
      let mut cursor = node.walk();
      for child in node.children(&mut cursor) {
        extract_spawned_calls_inner(child, source, targets, is_top_level);
      }
    }
    "async_block" | "block" | "closure_expression" => {
      let mut cursor = node.walk();
      for child in node.children(&mut cursor) {
        if child.kind() == "arguments" && node.kind() == "call_expression" {
          continue;
        }
        extract_spawned_calls_inner(child, source, targets, false);
      }
    }
    _ => {
      let mut cursor = node.walk();
      for child in node.children(&mut cursor) {
        extract_spawned_calls_inner(child, source, targets, false);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_extract_rust_symbols() {
    let source = r#"
      mod inner {
        pub struct User {
          pub name: String,
        }

        impl User {
          pub fn new(name: String) -> Self {
            Self { name }
          }
        }
      }

      pub trait Greeter {
        fn greet(&self);
      }

      impl Greeter for inner::User {
        fn greet(&self) {
          println!("Hello, {}", self.name);
        }
      }

      pub fn main() {
        let u = inner::User::new("Alice".to_string());
        u.greet();
      }
    "#;

    let symbols = extract_symbols_from_source(source, "src/main.rs", &[]).unwrap();

    assert!(
      symbols
        .iter()
        .any(|s| s.name == "main" && s.kind == SymbolKind::Module)
    );
    assert!(symbols.iter().any(|s| s.name == "inner"
      && s.kind == SymbolKind::Module
      && s.qualified_name == "src::main::inner"));
    assert!(symbols.iter().any(|s| s.name == "User"
      && s.kind == SymbolKind::Struct
      && s.qualified_name == "src::main::inner::User"));
    assert!(symbols.iter().any(|s| s.name == "new"
      && s.kind == SymbolKind::Method
      && s.qualified_name == "src::main::inner::User::new"));
    assert!(symbols.iter().any(|s| s.name == "Greeter"
      && s.kind == SymbolKind::Trait
      && s.qualified_name == "src::main::Greeter"));
    assert!(symbols.iter().any(|s| s.name == "greet"
      && s.kind == SymbolKind::Method
      && s.qualified_name == "src::main::Greeter::greet"));
    assert!(symbols.iter().any(|s| s.name == "greet"
      && s.kind == SymbolKind::Method
      && s.qualified_name == "src::main::<User as Greeter>::greet"));
    assert!(symbols.iter().any(|s| s.name == "main"
      && s.kind == SymbolKind::Function
      && s.qualified_name == "src::main::main"));
  }

  #[test]
  fn test_mod_rs_normalization() {
    let source = r#"
      pub struct Config;
    "#;
    let symbols = extract_symbols_from_source(source, "src/foo/mod.rs", &[]).unwrap();
    assert!(
      symbols
        .iter()
        .any(|s| s.name == "foo" && s.kind == SymbolKind::Module && s.qualified_name == "src::foo")
    );
    assert!(symbols.iter().any(|s| s.name == "Config"
      && s.kind == SymbolKind::Struct
      && s.qualified_name == "src::foo::Config"));
  }

  #[test]
  fn test_extract_rust_relationships() {
    let source = r#"
      use std::collections::HashMap;
      use inner::{A, B};

      pub struct Context {
        pub data: Option<QueryEngine>,
      }

      pub fn execute(ctx: Option<Context>) -> Result<Response, QueryError> {
        let engine = ctx.data;
        engine.run();
      }
    "#;

    let imports = extract_imports_from_source(source, "src/lib.rs", &[]).unwrap();
    assert_eq!(imports.len(), 3);
    assert!(
      imports
        .iter()
        .any(|imp| imp.target_qualified_name == "std::collections::HashMap")
    );
    assert!(
      imports
        .iter()
        .any(|imp| imp.target_qualified_name == "inner::A")
    );
    assert!(
      imports
        .iter()
        .any(|imp| imp.target_qualified_name == "inner::B")
    );

    let type_refs = extract_type_references_from_source(source, "src/lib.rs", &[]).unwrap();
    assert!(
      type_refs
        .iter()
        .any(|tr| tr.target_type_name == "QueryEngine"
          && tr.source_symbol_qualified_name == "src::lib::Context")
    );
    assert!(type_refs.iter().any(|tr| tr.target_type_name == "Context"
      && tr.source_symbol_qualified_name == "src::lib::execute"));
    assert!(type_refs.iter().any(|tr| tr.target_type_name == "Response"
      && tr.source_symbol_qualified_name == "src::lib::execute"));
    assert!(
      type_refs
        .iter()
        .any(|tr| tr.target_type_name == "QueryError"
          && tr.source_symbol_qualified_name == "src::lib::execute")
    );

    let calls = extract_calls_from_source(source, "src/lib.rs", &[]).unwrap();
    assert!(
      calls
        .iter()
        .any(|c| c.target_name == "run" && c.source_symbol_qualified_name == "src::lib::execute")
    );
  }

  #[test]
  fn test_extract_rust_tests() {
    let source = r#"
      #[cfg(test)]
      mod tests {
        use super::*;

        #[test]
        fn test_addition() {
          assert_eq!(1 + 1, 2);
        }

        #[tokio::test]
        async fn test_async_addition() {
          assert_eq!(1 + 1, 2);
        }

        #[rstest]
        fn test_parametrized(#[values(1, 2)] x: i32) {
          assert!(x > 0);
        }

        #[cfg(test)]
        fn helper_util() {}
      }
    "#;

    let tests = extract_tests_from_source(source, "src/lib.rs", &[]).unwrap();

    // There should be 4 test items: 1 module and 3 functions
    assert_eq!(tests.len(), 4);

    assert!(tests.iter().any(|t| t.name == "tests"
      && t.kind == ExtractedTestKind::Class
      && t.qualified_name == "src::lib::tests"));

    assert!(tests.iter().any(|t| t.name == "test_addition"
      && t.kind == ExtractedTestKind::Function
      && !t.is_parametrized
      && t.qualified_name == "src::lib::tests::test_addition"));

    assert!(tests.iter().any(|t| t.name == "test_async_addition"
      && t.kind == ExtractedTestKind::Function
      && !t.is_parametrized
      && t.qualified_name == "src::lib::tests::test_async_addition"));

    assert!(tests.iter().any(|t| t.name == "test_parametrized"
      && t.kind == ExtractedTestKind::Function
      && t.is_parametrized
      && t.parameters == vec!["x"]
      && t.qualified_name == "src::lib::tests::test_parametrized"));

    assert!(!tests.iter().any(|t| t.name == "helper_util"));
  }

  #[test]
  fn test_cfg_feature_flag_evaluation() {
    let source = r#"
      #[cfg(feature = "premium")]
      pub struct PremiumFeature;

      #[cfg(not(feature = "premium"))]
      pub struct StandardFeature;

      pub fn common_fn() {}
    "#;

    let symbols = extract_symbols_from_source(source, "src/lib.rs", &[]).unwrap();
    assert!(symbols.iter().any(|s| s.name == "StandardFeature"));
    assert!(!symbols.iter().any(|s| s.name == "PremiumFeature"));
    assert!(symbols.iter().any(|s| s.name == "common_fn"));

    let symbols_premium =
      extract_symbols_from_source(source, "src/lib.rs", &["premium".to_string()]).unwrap();
    assert!(!symbols_premium.iter().any(|s| s.name == "StandardFeature"));
    assert!(symbols_premium.iter().any(|s| s.name == "PremiumFeature"));
    assert!(symbols_premium.iter().any(|s| s.name == "common_fn"));
  }

  #[test]
  fn test_extract_simulated_derive_implementations() {
    let source = r#"
      #[derive(Debug, Serialize, Deserialize)]
      pub struct User {
        pub name: String,
      }

      #[derive(Error)]
      pub enum MyError {
        #[error("not found")]
        NotFound,
      }

      #[derive(Parser)]
      pub struct CliOpt {}
    "#;

    let impls = extract_implementations_from_source(source, "src/lib.rs", &[]).unwrap();

    assert!(impls.iter().any(
      |imp| imp.struct_name == "User" && imp.trait_name == Some("serde::Serialize".to_string())
    ));
    assert!(impls.iter().any(
      |imp| imp.struct_name == "User" && imp.trait_name == Some("serde::Deserialize".to_string())
    ));
    assert!(
      impls
        .iter()
        .any(|imp| imp.struct_name == "User" && imp.trait_name == Some("Debug".to_string()))
    );

    assert!(
      impls.iter().any(|imp| imp.struct_name == "MyError"
        && imp.trait_name == Some("std::error::Error".to_string()))
    );
    assert!(
      impls.iter().any(|imp| imp.struct_name == "MyError"
        && imp.trait_name == Some("std::fmt::Display".to_string()))
    );

    assert!(
      impls.iter().any(
        |imp| imp.struct_name == "CliOpt" && imp.trait_name == Some("clap::Parser".to_string())
      )
    );
  }

  #[test]
  fn test_extract_rust_concurrency() {
    let source = r#"
      use tokio::sync::mpsc;

      pub async fn worker() {}

      pub async fn main_flow() {
        let (mut tx, rx) = mpsc::channel(10);
        let (std_tx, std_rx) = std::sync::mpsc::channel();
        
        tokio::spawn(async move {
          worker().await;
        });

        tokio::select! {
          _ = rx.recv() => {}
        }
      }
    "#;

    let topo = extract_concurrency_topology_from_source(source, "src/lib.rs", &[]).unwrap();

    assert_eq!(topo.spawns.len(), 1);
    assert_eq!(
      topo.spawns[0].source_symbol_qualified_name,
      "src::lib::main_flow"
    );
    assert_eq!(topo.spawns[0].spawn_kind, "tokio::spawn");
    assert_eq!(topo.spawns[0].target_name, Some("worker".to_string()));

    assert_eq!(topo.channels.len(), 2);
    assert_eq!(
      topo.channels[0].source_symbol_qualified_name,
      "src::lib::main_flow"
    );
    assert_eq!(topo.channels[0].channel_kind, "mpsc");
    assert_eq!(topo.channels[0].tx_name, "tx");
    assert_eq!(topo.channels[0].rx_name, "rx");

    assert_eq!(
      topo.channels[1].source_symbol_qualified_name,
      "src::lib::main_flow"
    );
    assert_eq!(topo.channels[1].channel_kind, "std_mpsc");
    assert_eq!(topo.channels[1].tx_name, "std_tx");
    assert_eq!(topo.channels[1].rx_name, "std_rx");

    assert_eq!(topo.selects.len(), 1);
    assert_eq!(
      topo.selects[0].source_symbol_qualified_name,
      "src::lib::main_flow"
    );
  }

  #[test]
  fn test_extract_rust_routes() {
    let source = r#"
      use axum::{extract::{Path, State}, Json};
      use actix_web::{get, post, web, Responder};

      #[derive(Clone)]
      pub struct AppState {
        pub db: String,
      }

      pub struct UserPayload {
        pub username: String,
      }

      pub struct UserResponse {
        pub id: u64,
        pub username: String,
      }

      pub struct ItemPath {
        pub id: u32,
      }

      #[get("/items/{id}")]
      pub async fn get_item(
        state: web::Data<AppState>,
        path: web::Path<ItemPath>,
        raw_id: Path<u32>,
        tuple_path: web::Path<(String, u32)>,
      ) -> impl Responder {
        "item"
      }

      pub async fn create_user(
        State(state): State<AppState>,
        Json(payload): Json<UserPayload>,
      ) -> Json<UserResponse> {
        Json(UserResponse { id: 1, username: payload.username })
      }

      pub async fn list_users(
        State(state): State<AppState>,
      ) -> Json<Vec<UserResponse>> {
        Json(vec![])
      }

      pub fn app_router() {
        let app = Router::new()
          .route("/users", post(create_user).get(list_users))
          .route("/legacy", web::get().to(get_item));
      }
    "#;

    let (routes, deps) =
      extract_routes_and_dependencies_from_source(source, "src/lib.rs", &[]).unwrap();

    // Verify parsed routes
    assert_eq!(routes.len(), 4);

    // 1. Actix attribute macro
    let actix_route = routes
      .iter()
      .find(|r| r.handler_name == "get_item")
      .unwrap();
    assert_eq!(actix_route.method, "GET");
    assert_eq!(actix_route.path, "/items/{id}");
    assert_eq!(actix_route.qualified_name, "src::lib::get_item");

    // 2. Axum post route
    let post_route = routes
      .iter()
      .find(|r| r.path == "/users" && r.method == "POST")
      .unwrap();
    assert_eq!(post_route.handler_name, "create_user");
    assert_eq!(post_route.qualified_name, "src::lib::create_user");
    assert_eq!(post_route.response_model, Some("UserResponse".to_string()));

    // 3. Axum get route
    let get_route = routes
      .iter()
      .find(|r| r.path == "/users" && r.method == "GET")
      .unwrap();
    assert_eq!(get_route.handler_name, "list_users");
    assert_eq!(get_route.qualified_name, "src::lib::list_users");
    assert_eq!(
      get_route.response_model,
      Some("Vec<UserResponse>".to_string())
    );

    // 4. Actix chain route
    let actix_chain = routes.iter().find(|r| r.path == "/legacy").unwrap();
    assert_eq!(actix_chain.handler_name, "get_item");
    assert_eq!(actix_chain.method, "GET");

    // Verify dependencies
    // get_item should have AppState dependency
    assert!(
      deps
        .iter()
        .any(|(h, d)| h == "src::lib::get_item" && d == "AppState")
    );
    assert!(
      deps
        .iter()
        .any(|(h, d)| h == "src::lib::get_item" && d == "ItemPath")
    );
    assert!(!deps.iter().any(|(_, d)| d == "u32"));
    assert!(!deps.iter().any(|(_, d)| d == "(String, u32)"));

    // create_user should have AppState and UserPayload dependencies
    assert!(
      deps
        .iter()
        .any(|(h, d)| h == "src::lib::create_user" && d == "AppState")
    );
    assert!(
      deps
        .iter()
        .any(|(h, d)| h == "src::lib::create_user" && d == "UserPayload")
    );

    // list_users should have AppState dependency
    assert!(
      deps
        .iter()
        .any(|(h, d)| h == "src::lib::list_users" && d == "AppState")
    );
  }

  #[test]
  fn test_nested_same_name_handlers_routes() {
    let source = r#"
      use axum::{routing::get, Router};

      mod admin {
        use axum::Json;
        pub struct AdminResponse {}
        pub async fn index() -> Json<AdminResponse> {
          Json(AdminResponse {})
        }
        pub fn router() {
          let app = Router::new().route("/admin", get(index));
        }
      }

      mod public {
        use axum::Json;
        pub struct PublicResponse {}
        pub async fn index() -> Json<PublicResponse> {
          Json(PublicResponse {})
        }
        pub fn router() {
          let app = Router::new().route("/public", get(index));
        }
      }
    "#;

    let (routes, _deps) =
      extract_routes_and_dependencies_from_source(source, "src/lib.rs", &[]).unwrap();

    assert_eq!(routes.len(), 2);

    let admin_route = routes.iter().find(|r| r.path == "/admin").unwrap();
    assert_eq!(admin_route.handler_name, "index");
    assert_eq!(admin_route.qualified_name, "src::lib::admin::index");
    assert_eq!(
      admin_route.response_model,
      Some("AdminResponse".to_string())
    );

    let public_route = routes.iter().find(|r| r.path == "/public").unwrap();
    assert_eq!(public_route.handler_name, "index");
    assert_eq!(public_route.qualified_name, "src::lib::public::index");
    assert_eq!(
      public_route.response_model,
      Some("PublicResponse".to_string())
    );
  }

  #[test]
  fn test_extract_rust_safety_metadata() {
    let source = r#"
      #[cxx::bridge]
      mod ffi_bridge {
        unsafe extern "C++" {
          fn cxx_func();
        }
      }

      pub unsafe fn do_raw_stuff() {
        let ptr = std::ptr::null::<i32>();
      }

      pub fn safe_wrapper() {
        // Safe wrapper around an unsafe block
        unsafe {
          let val = *std::ptr::null::<i32>();
        }
      }

      extern "C" {
        fn c_api_func(x: i32) -> i32;
      }
    "#;

    let meta = extract_safety_metadata_from_source(source, "src/lib.rs", &[]).unwrap();

    // Verify unsafe functions
    assert_eq!(meta.unsafe_functions.len(), 1);
    assert_eq!(
      meta.unsafe_functions[0].qualified_name,
      "src::lib::do_raw_stuff"
    );

    // Verify unsafe blocks
    assert_eq!(meta.unsafe_blocks.len(), 1);
    assert_eq!(
      meta.unsafe_blocks[0].source_symbol_qualified_name,
      "src::lib::safe_wrapper"
    );

    // Verify FFI bindings (cxx bridge and extern "C" declarations)
    // ffi_bridge module itself (cxx bridge) + cxx_func inside + c_api_func inside extern C
    assert_eq!(meta.ffi_bindings.len(), 3);

    let cxx_bridge = meta.ffi_bindings.iter().find(|f| f.abi == "cxx").unwrap();
    assert_eq!(cxx_bridge.foreign_item_name, "ffi_bridge");

    let c_api = meta
      .ffi_bindings
      .iter()
      .find(|f| f.foreign_item_name == "c_api_func")
      .unwrap();
    assert_eq!(c_api.abi, "C");
  }

  #[test]
  fn test_extract_rust_safety_traits_and_statics() {
    let source = r#"
      pub trait MyUnsafeTrait {
        unsafe fn unsafe_trait_method();
        fn safe_trait_method();
      }

      unsafe extern "C" {
        static mut errno: i32;
        type MyOpaqueType;
        fn foreign_fn();
      }
    "#;

    let meta = extract_safety_metadata_from_source(source, "src/lib.rs", &[]).unwrap();

    // Verify unsafe trait method is extracted as unsafe function
    assert_eq!(meta.unsafe_functions.len(), 1);
    assert_eq!(
      meta.unsafe_functions[0].qualified_name,
      "src::lib::MyUnsafeTrait::unsafe_trait_method"
    );

    // Verify foreign static, type, and function are all extracted as FFI bindings
    assert_eq!(meta.ffi_bindings.len(), 3);

    let foreign_static = meta
      .ffi_bindings
      .iter()
      .find(|f| f.foreign_item_name == "errno")
      .unwrap();
    assert_eq!(foreign_static.abi, "C");

    let foreign_type = meta
      .ffi_bindings
      .iter()
      .find(|f| f.foreign_item_name == "MyOpaqueType")
      .unwrap();
    assert_eq!(foreign_type.abi, "C");

    let foreign_fn = meta
      .ffi_bindings
      .iter()
      .find(|f| f.foreign_item_name == "foreign_fn")
      .unwrap();
    assert_eq!(foreign_fn.abi, "C");
  }

  #[test]
  fn test_extract_concurrency_edges() {
    let source = r#"
      use tokio::sync::mpsc;

      pub async fn main_task() {
        let (tx, rx) = mpsc::channel(32);
        
        tokio::spawn(async move {
          worker_task(tx).await;
        });

        tokio::spawn(worker_task_direct);

        std::thread::spawn(move || {
          thread_worker();
        });

        tokio::select! {
          val = rx.recv() => {
            println!("Got: {:?}", val);
          }
        }
      }

      pub async fn worker_task(tx: mpsc::Sender<i32>) {
        tx.send(42).await.unwrap();
      }

      pub fn worker_task_direct() {}
      pub fn thread_worker() {}
    "#;

    let edges = extract_concurrency_from_source(source, "src/lib.rs", &[]).unwrap();

    // 1. Verify spawns
    assert!(
      edges
        .iter()
        .any(|e| e.source_symbol_qualified_name == "src::lib::main_task"
          && e.target_name == "worker_task"
          && e.kind == "Spawns")
    );
    assert!(
      edges
        .iter()
        .any(|e| e.source_symbol_qualified_name == "src::lib::main_task"
          && e.target_name == "worker_task_direct"
          && e.kind == "Spawns")
    );
    assert!(
      edges
        .iter()
        .any(|e| e.source_symbol_qualified_name == "src::lib::main_task"
          && e.target_name == "thread_worker"
          && e.kind == "Spawns")
    );

    // 2. Verify channel SendsTo edge
    assert!(edges.iter().any(
      |e| e.source_symbol_qualified_name == "src::lib::worker_task"
        && e.target_name == "src::lib::main_task"
        && e.kind == "SendsTo"
    ));
  }

  #[test]
  fn test_extract_concurrency_edges_edge_cases() {
    let source = r#"
      pub async fn main_task() {
        tokio::spawn(async move {
          let my_var = 10;
          worker_task(my_var).await;
        });

        let worker = MyWorker;
        tokio::spawn(worker.run());

        tokio::spawn(async move {
          worker.run().await;
        });
      }

      pub async fn worker_task(x: i32) {}
      pub struct MyWorker;
      impl MyWorker {
        pub fn run(&self) {}
      }
    "#;

    let edges = extract_concurrency_from_source(source, "src/lib.rs", &[]).unwrap();
    println!("Extracted edges: {:#?}", edges);

    // We expect ONLY worker_task and worker.run() (or similar) to be spawned,
    // NOT my_var or worker.
    for edge in &edges {
      if edge.kind == "Spawns" {
        assert!(
          edge.target_name == "worker_task"
            || edge.target_name == "worker.run"
            || edge.target_name == "run",
          "Incorrect spawn target extracted: {}",
          edge.target_name
        );
      }
    }
  }

  #[test]
  fn test_attribute_interrupted_by_comment() {
    let source = r#"
      #[cfg(feature = "premium")]
      // A comment about premium
      pub struct PremiumFeature;

      #[test]
      /// Doc comment about the test
      fn test_with_comment() {}
    "#;

    let tests = extract_tests_from_source(source, "src/lib.rs", &[]).unwrap();
    println!("Extracted tests: {:#?}", tests);
    assert!(
      tests.iter().any(|t| t.name == "test_with_comment"),
      "Test was not extracted because the attribute was interrupted by a comment!"
    );
  }
}
