use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
  pub jsonrpc: String,
  pub id: Option<serde_json::Value>,
  pub method: String,
  pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
  pub jsonrpc: String,
  pub id: serde_json::Value,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub result: Option<serde_json::Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
  pub code: i64,
  pub message: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
  pub name: String,
  pub description: String,
  #[serde(rename = "inputSchema")]
  pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolParams {
  pub name: String,
  pub arguments: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolTextContent {
  #[serde(rename = "type")]
  pub content_type: String,
  pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolResult {
  pub content: Vec<CallToolTextContent>,
  #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
  pub is_error: Option<bool>,
}

fn get_tools_list() -> Vec<Tool> {
  vec![
    Tool {
      name: "find_symbol".to_string(),
      description: "Find symbols in the repository by name (exact or fuzzy)".to_string(),
      input_schema: serde_json::json!({
        "type": "object",
        "properties": {
          "name": {
            "type": "string",
            "description": "Symbol name or substring to search for"
          }
        },
        "required": ["name"]
      }),
    },
    Tool {
      name: "get_symbol".to_string(),
      description: "Retrieve a symbol definition and its code snippet by qualified name"
        .to_string(),
      input_schema: serde_json::json!({
        "type": "object",
        "properties": {
          "qualified_name": {
            "type": "string",
            "description": "The qualified name of the symbol (e.g. src.module::ClassName.method_name)"
          }
        },
        "required": ["qualified_name"]
      }),
    },
    Tool {
      name: "get_callers".to_string(),
      description: "Retrieve callers of a symbol".to_string(),
      input_schema: serde_json::json!({
        "type": "object",
        "properties": {
          "name": {
            "type": "string",
            "description": "Symbol name to find callers for"
          }
        },
        "required": ["name"]
      }),
    },
    Tool {
      name: "get_callees".to_string(),
      description: "Retrieve callees called by a symbol".to_string(),
      input_schema: serde_json::json!({
        "type": "object",
        "properties": {
          "name": {
            "type": "string",
            "description": "Symbol name to find callees for"
          }
        },
        "required": ["name"]
      }),
    },
    Tool {
      name: "get_docs".to_string(),
      description: "Retrieve documentation for a symbol".to_string(),
      input_schema: serde_json::json!({
        "type": "object",
        "properties": {
          "name": {
            "type": "string",
            "description": "Symbol name to find documentation for"
          }
        },
        "required": ["name"]
      }),
    },
    Tool {
      name: "get_tests".to_string(),
      description: "Retrieve test cases associated with a symbol".to_string(),
      input_schema: serde_json::json!({
        "type": "object",
        "properties": {
          "name": {
            "type": "string",
            "description": "Symbol name to find test cases for"
          }
        },
        "required": ["name"]
      }),
    },
    Tool {
      name: "get_impact_analysis".to_string(),
      description: "Retrieve upstream/downstream impact analysis for a symbol".to_string(),
      input_schema: serde_json::json!({
        "type": "object",
        "properties": {
          "name": {
            "type": "string",
            "description": "Symbol name to run impact analysis for"
          },
          "depth": {
            "type": "integer",
            "description": "Maximum depth of traversal (default: 2)"
          }
        },
        "required": ["name"]
      }),
    },
    Tool {
      name: "get_context_pack".to_string(),
      description: "Retrieve a token-budgeted context pack for a target symbol".to_string(),
      input_schema: serde_json::json!({
        "type": "object",
        "properties": {
          "symbol": {
            "type": "string",
            "description": "Target symbol to build context pack for"
          },
          "budget": {
            "type": "integer",
            "description": "Optional token budget (default: 2000)"
          },
          "format": {
            "type": "string",
            "description": "Output format: 'markdown' or 'json' (default: 'markdown')"
          }
        },
        "required": ["symbol"]
      }),
    },
  ]
}

pub fn run_mcp_server(
  connection: &Connection,
  repository_id: i64,
  repo_root: &Path,
) -> Result<(), String> {
  let stdin = std::io::stdin();
  let stdout = std::io::stdout();
  let mut stdin_lock = stdin.lock();
  let mut stdout_lock = stdout.lock();
  let mut line = String::new();

  eprintln!("rkg-mcp server started. Reading from stdin...");

  loop {
    line.clear();
    let bytes_read = stdin_lock
      .read_line(&mut line)
      .map_err(|e| format!("failed to read line from stdin: {e}"))?;

    if bytes_read == 0 {
      eprintln!("EOF reached. Shutting down.");
      break;
    }

    let trimmed = line.trim();
    if trimmed.is_empty() {
      continue;
    }

    eprintln!("Received request: {}", trimmed);

    let request: JsonRpcRequest = match serde_json::from_str(trimmed) {
      Ok(req) => req,
      Err(err) => {
        let response = JsonRpcResponse {
          jsonrpc: "2.0".to_string(),
          id: serde_json::Value::Null,
          result: None,
          error: Some(JsonRpcError {
            code: -32700,
            message: format!("Parse error: {err}"),
            data: None,
          }),
        };
        send_response(&mut stdout_lock, &response)?;
        continue;
      }
    };

    let response = handle_request(connection, repository_id, repo_root, &request);
    if let Some(res) = response {
      send_response(&mut stdout_lock, &res)?;
    }
  }

  Ok(())
}

pub fn handle_request(
  connection: &Connection,
  repository_id: i64,
  repo_root: &Path,
  request: &JsonRpcRequest,
) -> Option<JsonRpcResponse> {
  let id = request.id.clone().unwrap_or(serde_json::Value::Null);

  match request.method.as_str() {
    "initialize" => {
      let result = serde_json::json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
          "tools": {}
        },
        "serverInfo": {
          "name": "rkg-mcp",
          "version": "0.1.0"
        }
      });
      Some(JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(result),
        error: None,
      })
    }
    "notifications/initialized" => None,
    "tools/list" => {
      let tools = get_tools_list();
      let result = serde_json::json!({
        "tools": tools
      });
      Some(JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(result),
        error: None,
      })
    }
    "tools/call" => {
      let call_params: Result<CallToolParams, String> = request
        .params
        .as_ref()
        .ok_or_else(|| "missing params for tools/call".to_string())
        .and_then(|p| serde_json::from_value(p.clone()).map_err(|e| e.to_string()));

      match call_params {
        Ok(params) => {
          match handle_tool_call(
            connection,
            repository_id,
            repo_root,
            &params.name,
            params.arguments,
          ) {
            Ok(tool_result) => match serde_json::to_value(tool_result) {
              Ok(result) => Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(result),
                error: None,
              }),
              Err(error) => Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                  code: -32603,
                  message: format!("failed to serialize tool result: {error}"),
                  data: None,
                }),
              }),
            },
            Err(err_msg) => Some(JsonRpcResponse {
              jsonrpc: "2.0".to_string(),
              id,
              result: None,
              error: Some(JsonRpcError {
                code: -32603,
                message: err_msg,
                data: None,
              }),
            }),
          }
        }
        Err(err_msg) => Some(JsonRpcResponse {
          jsonrpc: "2.0".to_string(),
          id,
          result: None,
          error: Some(JsonRpcError {
            code: -32602,
            message: format!("Invalid params: {err_msg}"),
            data: None,
          }),
        }),
      }
    }
    "ping" => Some(JsonRpcResponse {
      jsonrpc: "2.0".to_string(),
      id,
      result: Some(serde_json::json!({})),
      error: None,
    }),
    _ => Some(JsonRpcResponse {
      jsonrpc: "2.0".to_string(),
      id,
      result: None,
      error: Some(JsonRpcError {
        code: -32601,
        message: format!("Method not found: {}", request.method),
        data: None,
      }),
    }),
  }
}

fn send_response<W: Write>(writer: &mut W, response: &JsonRpcResponse) -> Result<(), String> {
  let serialized = serde_json::to_string(response)
    .map_err(|e| format!("failed to serialize JSON-RPC response: {e}"))?;
  writer
    .write_all(format!("{serialized}\n").as_bytes())
    .map_err(|e| format!("failed to write JSON-RPC response: {e}"))?;
  writer
    .flush()
    .map_err(|e| format!("failed to flush stdout: {e}"))?;
  Ok(())
}

fn handle_tool_call(
  connection: &Connection,
  repository_id: i64,
  repo_root: &Path,
  name: &str,
  arguments: Option<serde_json::Value>,
) -> Result<CallToolResult, String> {
  match name {
    "find_symbol" => {
      #[derive(Deserialize)]
      struct Args {
        name: String,
      }
      let args: Args = serde_json::from_value(arguments.ok_or("missing arguments")?)
        .map_err(|e| format!("invalid arguments: {e}"))?;

      let mut symbols = rkg_query::resolve_start_symbols(connection, repository_id, &args.name)
        .map_err(|e| e.to_string())?;

      if symbols.is_empty() {
        let mut stmt = connection
          .prepare(
            "SELECT s.id, s.file_id, s.name, s.qualified_name, s.kind, s.start_line, s.end_line, s.start_column, s.end_column
             FROM symbols s
             INNER JOIN files f ON f.id = s.file_id
             WHERE f.repository_id = ?1 AND (s.name LIKE ?2 OR s.qualified_name LIKE ?2)
             ORDER BY s.qualified_name ASC, s.id ASC",
          )
          .map_err(|e| e.to_string())?;

        let search_pattern = format!("%{}%", args.name);
        let rows = stmt
          .query_map((repository_id, search_pattern), |row| {
            Ok(rkg_db::SymbolRecord {
              id: row.get(0)?,
              file_id: row.get(1)?,
              name: row.get(2)?,
              qualified_name: row.get(3)?,
              kind: row.get(4)?,
              start_line: row.get(5)?,
              end_line: row.get(6)?,
              start_column: row.get(7)?,
              end_column: row.get(8)?,
            })
          })
          .map_err(|e| e.to_string())?;

        for sym in rows.flatten() {
          symbols.push(sym);
        }
      }

      if symbols.is_empty() {
        return Ok(CallToolResult {
          content: vec![CallToolTextContent {
            content_type: "text".to_string(),
            text: format!("No symbols found matching name: {}", args.name),
          }],
          is_error: None,
        });
      }

      let mut lines = Vec::new();
      for sym in symbols {
        lines.push(format!(
          "- `{}` (Kind: {}, Lines {}-{})",
          sym.qualified_name, sym.kind, sym.start_line, sym.end_line
        ));
      }

      Ok(CallToolResult {
        content: vec![CallToolTextContent {
          content_type: "text".to_string(),
          text: lines.join("\n"),
        }],
        is_error: None,
      })
    }
    "get_symbol" => {
      #[derive(Deserialize)]
      struct Args {
        qualified_name: String,
      }
      let args: Args = serde_json::from_value(arguments.ok_or("missing arguments")?)
        .map_err(|e| format!("invalid arguments: {e}"))?;

      let symbol =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &args.qualified_name)
          .map_err(|e| e.to_string())?;

      let Some(sym) = symbol else {
        return Ok(CallToolResult {
          content: vec![CallToolTextContent {
            content_type: "text".to_string(),
            text: format!("Symbol not found: {}", args.qualified_name),
          }],
          is_error: Some(true),
        });
      };

      let mut stmt = connection
        .prepare("SELECT path FROM files WHERE id = ?1")
        .map_err(|e| e.to_string())?;
      let path_rel: String = stmt
        .query_row([sym.file_id], |row| row.get(0))
        .map_err(|e| e.to_string())?;

      let file_path = repo_root.join(&path_rel);
      let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("failed to read file {}: {e}", file_path.display()))?;

      let file_lines: Vec<&str> = content.lines().collect();
      let start_idx = (sym.start_line as usize).saturating_sub(1);
      let end_idx = (sym.end_line as usize).min(file_lines.len());
      let snippet = if start_idx < file_lines.len() && start_idx < end_idx {
        file_lines[start_idx..end_idx].join("\n")
      } else {
        String::new()
      };

      let output = format!(
        "Symbol: {} [{}]\nFile: {} (lines {}-{})\n----------------------------------------\n{}",
        sym.qualified_name, sym.kind, path_rel, sym.start_line, sym.end_line, snippet
      );

      Ok(CallToolResult {
        content: vec![CallToolTextContent {
          content_type: "text".to_string(),
          text: output,
        }],
        is_error: None,
      })
    }
    "get_callers" => {
      #[derive(Deserialize)]
      struct Args {
        name: String,
      }
      let args: Args = serde_json::from_value(arguments.ok_or("missing arguments")?)
        .map_err(|e| format!("invalid arguments: {e}"))?;

      let callers =
        rkg_query::get_callers(connection, repository_id, &args.name).map_err(|e| e.to_string())?;

      if callers.is_empty() {
        return Ok(CallToolResult {
          content: vec![CallToolTextContent {
            content_type: "text".to_string(),
            text: format!("No callers found for symbol: {}", args.name),
          }],
          is_error: None,
        });
      }

      let mut lines = Vec::new();
      for (caller_name, caller_kind, caller_file_path, confidence) in callers {
        let conf_str = confidence
          .map(|c| format!(" (Confidence: {:.1})", c))
          .unwrap_or_default();
        lines.push(format!(
          "{} [{}] (File: {}){}",
          caller_name, caller_kind, caller_file_path, conf_str
        ));
      }

      Ok(CallToolResult {
        content: vec![CallToolTextContent {
          content_type: "text".to_string(),
          text: lines.join("\n"),
        }],
        is_error: None,
      })
    }
    "get_callees" => {
      #[derive(Deserialize)]
      struct Args {
        name: String,
      }
      let args: Args = serde_json::from_value(arguments.ok_or("missing arguments")?)
        .map_err(|e| format!("invalid arguments: {e}"))?;

      let callees =
        rkg_query::get_callees(connection, repository_id, &args.name).map_err(|e| e.to_string())?;

      if callees.is_empty() {
        return Ok(CallToolResult {
          content: vec![CallToolTextContent {
            content_type: "text".to_string(),
            text: format!("No callees found for symbol: {}", args.name),
          }],
          is_error: None,
        });
      }

      let mut lines = Vec::new();
      for (target_name, target_kind, target_file_path, confidence) in callees {
        let conf_str = confidence
          .map(|c| format!(" (Confidence: {:.1})", c))
          .unwrap_or_default();
        if let Some(file) = target_file_path {
          let kind = target_kind.unwrap_or_else(|| "unknown".to_string());
          lines.push(format!(
            "{} [Resolved] [{}] (File: {}){}",
            target_name, kind, file, conf_str
          ));
        } else {
          lines.push(format!("{} [Unresolved]{}", target_name, conf_str));
        }
      }

      Ok(CallToolResult {
        content: vec![CallToolTextContent {
          content_type: "text".to_string(),
          text: lines.join("\n"),
        }],
        is_error: None,
      })
    }
    "get_docs" => {
      #[derive(Deserialize)]
      struct Args {
        name: String,
      }
      let args: Args = serde_json::from_value(arguments.ok_or("missing arguments")?)
        .map_err(|e| format!("invalid arguments: {e}"))?;

      let docs = rkg_query::get_docs_for_symbol(connection, repository_id, &args.name)
        .map_err(|e| e.to_string())?;

      if docs.is_empty() {
        return Ok(CallToolResult {
          content: vec![CallToolTextContent {
            content_type: "text".to_string(),
            text: format!("No documentation found for symbol: {}", args.name),
          }],
          is_error: None,
        });
      }

      let mut output = format!("Documentation for {}:\n", args.name);
      for doc in docs {
        let mut stmt = connection
          .prepare("SELECT path FROM files WHERE id = ?1")
          .map_err(|e| e.to_string())?;
        let path: String = stmt
          .query_row([doc.file_id], |row| row.get(0))
          .map_err(|e| e.to_string())?;

        let lines_str = if let (Some(start), Some(end)) = (doc.start_line, doc.end_line) {
          format!(" (lines {}-{})", start, end)
        } else {
          String::new()
        };

        let title_str = if let Some(t) = &doc.title {
          format!(" - Heading: {t}")
        } else {
          String::new()
        };

        output.push_str(&format!(
          "--- [{}] {}{}{} ---\n",
          doc.source_kind, path, lines_str, title_str
        ));
        output.push_str(&format!("{}\n", doc.body.trim()));
      }
      output.push_str("----------------------------------------");

      Ok(CallToolResult {
        content: vec![CallToolTextContent {
          content_type: "text".to_string(),
          text: output,
        }],
        is_error: None,
      })
    }
    "get_tests" => {
      #[derive(Deserialize)]
      struct Args {
        name: String,
      }
      let args: Args = serde_json::from_value(arguments.ok_or("missing arguments")?)
        .map_err(|e| format!("invalid arguments: {e}"))?;

      let tests = rkg_query::get_tests_for_symbol(connection, repository_id, &args.name)
        .map_err(|e| e.to_string())?;

      if tests.is_empty() {
        return Ok(CallToolResult {
          content: vec![CallToolTextContent {
            content_type: "text".to_string(),
            text: format!("No tests found associated with symbol: {}", args.name),
          }],
          is_error: None,
        });
      }

      let mut lines = Vec::new();
      lines.push(format!("Tests testing {}:", args.name));
      for (test_name, test_file_path, confidence) in tests {
        let conf_str = confidence
          .map(|c| format!(" (Confidence: {:.1})", c))
          .unwrap_or_default();
        lines.push(format!(
          "  - {} (File: {}){}",
          test_name, test_file_path, conf_str
        ));
      }

      Ok(CallToolResult {
        content: vec![CallToolTextContent {
          content_type: "text".to_string(),
          text: lines.join("\n"),
        }],
        is_error: None,
      })
    }
    "get_impact_analysis" => {
      #[derive(Deserialize)]
      struct Args {
        name: String,
        depth: Option<usize>,
      }
      let args: Args = serde_json::from_value(arguments.ok_or("missing arguments")?)
        .map_err(|e| format!("invalid arguments: {e}"))?;

      let depth = args.depth.unwrap_or(2);
      let results = rkg_query::analyze_impact(connection, repository_id, &args.name, depth)
        .map_err(|e| e.to_string())?;

      if results.target_symbols.is_empty() {
        return Ok(CallToolResult {
          content: vec![CallToolTextContent {
            content_type: "text".to_string(),
            text: format!("No matching symbols found for: {}", args.name),
          }],
          is_error: None,
        });
      }

      let mut out = String::new();
      out.push_str("Target Symbol(s) matched:\n");
      for sym in &results.target_symbols {
        let mut stmt = connection
          .prepare("SELECT path FROM files WHERE id = ?1")
          .map_err(|e| e.to_string())?;
        let path: String = stmt
          .query_row([sym.file_id], |row| row.get(0))
          .map_err(|e| e.to_string())?;

        out.push_str(&format!(
          "  - {} [{}] (File: {}, Lines: {}-{})\n",
          sym.qualified_name, sym.kind, path, sym.start_line, sym.end_line
        ));
      }
      out.push('\n');

      out.push_str("=========================================\n");
      out.push_str("DOWNSTREAM IMPACT (Blast Radius / Forward)\n");
      out.push_str("=========================================\n");
      let mut has_downstream = false;
      for d in 1..=depth {
        let depth_nodes: Vec<_> = results
          .downstream_nodes
          .iter()
          .filter(|n| n.depth == d)
          .collect();
        if !depth_nodes.is_empty() {
          has_downstream = true;
          out.push_str(&format!("Depth {}:\n", d));
          for node in depth_nodes {
            let edge_info = results
              .downstream_edges
              .iter()
              .find(|e| e.target_symbol_id == node.symbol.id);
            let rel_str = if let Some(edge) = edge_info {
              format!(" via {}", edge.kind)
            } else {
              String::new()
            };
            out.push_str(&format!(
              "  -> {} [{}] (File: {}, Lines: {}-{}){}\n",
              node.symbol.qualified_name,
              node.symbol.kind,
              node.file_path,
              node.symbol.start_line,
              node.symbol.end_line,
              rel_str
            ));
          }
        }
      }
      if !has_downstream {
        out.push_str("  None\n");
      }
      out.push('\n');

      out.push_str("=========================================\n");
      out.push_str("UPSTREAM IMPACT (Affected Code / Backward)\n");
      out.push_str("=========================================\n");
      let mut has_upstream = false;
      for d in 1..=depth {
        let depth_nodes: Vec<_> = results
          .upstream_nodes
          .iter()
          .filter(|n| n.depth == d)
          .collect();
        if !depth_nodes.is_empty() {
          has_upstream = true;
          out.push_str(&format!("Depth {}:\n", d));
          for node in depth_nodes {
            let edge_info = results
              .upstream_edges
              .iter()
              .find(|e| e.source_symbol_id == node.symbol.id);
            let rel_str = if let Some(edge) = edge_info {
              format!(" via {}", edge.kind)
            } else {
              String::new()
            };
            out.push_str(&format!(
              "  <- {} [{}] (File: {}, Lines: {}-{}){}\n",
              node.symbol.qualified_name,
              node.symbol.kind,
              node.file_path,
              node.symbol.start_line,
              node.symbol.end_line,
              rel_str
            ));
          }
        }
      }
      if !has_upstream {
        out.push_str("  None\n");
      }
      out.push('\n');

      out.push_str("=========================================\n");
      out.push_str("AFFECTED TESTS\n");
      out.push_str("=========================================\n");
      if results.affected_tests.is_empty() {
        out.push_str("  None\n");
      } else {
        for test in &results.affected_tests {
          let conf_str = test
            .confidence
            .map(|c| format!(" (Confidence: {:.1})", c))
            .unwrap_or_default();
          out.push_str(&format!(
            "  - {} [{}] (File: {}) -> Tests {}{}\n",
            test.test_qualified_name,
            test.test_kind,
            test.file_path,
            test.linked_symbol_qualified_name,
            conf_str
          ));
        }
      }
      out.push('\n');

      out.push_str("=========================================\n");
      out.push_str("AFFECTED DOCUMENTATION\n");
      out.push_str("=========================================\n");
      if results.affected_docs.is_empty() {
        out.push_str("  None\n");
      } else {
        for doc in &results.affected_docs {
          let title_str = if let Some(t) = &doc.title {
            format!(": Heading \"{}\"", t)
          } else {
            ": Docstring".to_string()
          };
          let lines_str = if let (Some(start), Some(end)) = (doc.start_line, doc.end_line) {
            format!(" (Lines: {}-{})", start, end)
          } else {
            String::new()
          };
          out.push_str(&format!(
            "  - File: {}{}{} -> Documents {}\n",
            doc.file_path, title_str, lines_str, doc.linked_symbol_qualified_name
          ));
        }
      }
      out.push('\n');

      Ok(CallToolResult {
        content: vec![CallToolTextContent {
          content_type: "text".to_string(),
          text: out,
        }],
        is_error: None,
      })
    }
    "get_context_pack" => {
      #[derive(Deserialize)]
      struct Args {
        symbol: String,
        budget: Option<usize>,
        format: Option<String>,
      }
      let args: Args = serde_json::from_value(arguments.ok_or("missing arguments")?)
        .map_err(|e| format!("invalid arguments: {e}"))?;

      let budget = args.budget;
      let format_str = args.format.unwrap_or_else(|| "markdown".to_string());

      let pack = rkg_query::pack_context(
        connection,
        repository_id,
        repo_root,
        &args.symbol,
        budget,
        &format_str,
      )
      .map_err(|e| e.to_string())?;

      if pack.symbols.is_empty() {
        return Ok(CallToolResult {
          content: vec![CallToolTextContent {
            content_type: "text".to_string(),
            text: format!(
              "No matching symbols found for context building: {}",
              args.symbol
            ),
          }],
          is_error: None,
        });
      }

      let formatted = if format_str == "json" {
        rkg_query::format_context_pack_json(&pack, repo_root)
      } else {
        rkg_query::format_context_pack_markdown(&pack, repo_root)
      };

      Ok(CallToolResult {
        content: vec![CallToolTextContent {
          content_type: "text".to_string(),
          text: formatted,
        }],
        is_error: None,
      })
    }
    _ => Err(format!("Unknown tool: {name}")),
  }
}
