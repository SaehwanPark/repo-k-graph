use rkg_mcp::server::{JsonRpcRequest, handle_request};
use rusqlite::Connection;
use serde_json::json;
use std::fs;
use std::path::Path;

fn setup_test_db() -> (Connection, i64) {
  let connection = Connection::open_in_memory().expect("in-memory db must open");
  rkg_db::initialize_schema(&connection).expect("schema initialization must succeed");

  let repo =
    rkg_db::upsert_repository(&connection, "/test/repo").expect("repository upsert must succeed");
  (connection, repo.id)
}

#[test]
fn test_initialize_handshake() {
  let (connection, repo_id) = setup_test_db();
  let repo_root = Path::new("/test/repo");

  let req = JsonRpcRequest {
    jsonrpc: "2.0".to_string(),
    id: Some(json!(1)),
    method: "initialize".to_string(),
    params: Some(json!({
      "protocolVersion": "2024-11-05",
      "capabilities": {},
      "clientInfo": {
        "name": "test-client",
        "version": "1.0"
      }
    })),
  };

  let response_opt = handle_request(&connection, repo_id, repo_root, &req);
  assert!(response_opt.is_some());
  let res = response_opt.unwrap();
  assert_eq!(res.jsonrpc, "2.0");
  assert_eq!(res.id, json!(1));
  assert!(res.error.is_none());

  let result = res.result.unwrap();
  assert_eq!(result["protocolVersion"], "2024-11-05");
  assert_eq!(result["serverInfo"]["name"], "rkg-mcp");
}

#[test]
fn test_notification_initialized() {
  let (connection, repo_id) = setup_test_db();
  let repo_root = Path::new("/test/repo");

  let req = JsonRpcRequest {
    jsonrpc: "2.0".to_string(),
    id: None,
    method: "notifications/initialized".to_string(),
    params: None,
  };

  let response_opt = handle_request(&connection, repo_id, repo_root, &req);
  assert!(response_opt.is_none());
}

#[test]
fn test_tools_list() {
  let (connection, repo_id) = setup_test_db();
  let repo_root = Path::new("/test/repo");

  let req = JsonRpcRequest {
    jsonrpc: "2.0".to_string(),
    id: Some(json!("abc")),
    method: "tools/list".to_string(),
    params: None,
  };

  let response_opt = handle_request(&connection, repo_id, repo_root, &req);
  assert!(response_opt.is_some());
  let res = response_opt.unwrap();
  assert_eq!(res.id, json!("abc"));

  let result = res.result.unwrap();
  let tools = result["tools"].as_array().unwrap();
  assert!(tools.len() >= 8);

  let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
  assert!(tool_names.contains(&"find_symbol"));
  assert!(tool_names.contains(&"get_symbol"));
  assert!(tool_names.contains(&"get_callers"));
  assert!(tool_names.contains(&"get_callees"));
  assert!(tool_names.contains(&"get_docs"));
  assert!(tool_names.contains(&"get_tests"));
  assert!(tool_names.contains(&"get_impact_analysis"));
  assert!(tool_names.contains(&"get_context_pack"));
}

#[test]
fn test_tool_call_find_symbol() {
  let (connection, repo_id) = setup_test_db();
  let repo_root = Path::new("/test/repo");

  // Insert mock symbol
  let file_record = rkg_db::reindex_file(
    &connection,
    &rkg_db::NewFileRecord {
      repository_id: repo_id,
      path: "src/main.py".to_string(),
      language: Some("python".to_string()),
      content_hash: Some("hash123".to_string()),
      line_count: Some(10),
      last_index_run_id: None,
    },
  )
  .unwrap();

  rkg_db::insert_symbol(
    &connection,
    &rkg_db::NewSymbolRecord {
      file_id: file_record.id,
      name: "calculate_sum".to_string(),
      qualified_name: "src.main::calculate_sum".to_string(),
      kind: "Function".to_string(),
      start_line: 1,
      end_line: 4,
      start_column: Some(0),
      end_column: Some(15),
    },
  )
  .unwrap();

  let req = JsonRpcRequest {
    jsonrpc: "2.0".to_string(),
    id: Some(json!(2)),
    method: "tools/call".to_string(),
    params: Some(json!({
      "name": "find_symbol",
      "arguments": {
        "name": "calculate"
      }
    })),
  };

  let response_opt = handle_request(&connection, repo_id, repo_root, &req);
  assert!(response_opt.is_some());
  let res = response_opt.unwrap();
  assert!(res.error.is_none());

  let result = res.result.unwrap();
  let content = result["content"].as_array().unwrap();
  assert_eq!(content.len(), 1);
  assert_eq!(content[0]["type"], "text");

  let text = content[0]["text"].as_str().unwrap();
  assert!(text.contains("src.main::calculate_sum"));
  assert!(text.contains("Function"));
}

#[test]
fn test_tool_call_get_symbol() {
  let (connection, repo_id) = setup_test_db();

  let temp_dir = tempfile::tempdir().unwrap();
  let repo_root = temp_dir.path();

  let relative_file_path = "src/math.py";
  let abs_file_path = repo_root.join(relative_file_path);
  fs::create_dir_all(abs_file_path.parent().unwrap()).unwrap();

  let source_code = "def multiply(a, b):\n    return a * b\n\n# Some comments\n";
  fs::write(&abs_file_path, source_code).unwrap();

  let file_record = rkg_db::reindex_file(
    &connection,
    &rkg_db::NewFileRecord {
      repository_id: repo_id,
      path: relative_file_path.to_string(),
      language: Some("python".to_string()),
      content_hash: Some("hash456".to_string()),
      line_count: Some(5),
      last_index_run_id: None,
    },
  )
  .unwrap();

  rkg_db::insert_symbol(
    &connection,
    &rkg_db::NewSymbolRecord {
      file_id: file_record.id,
      name: "multiply".to_string(),
      qualified_name: "src.math::multiply".to_string(),
      kind: "Function".to_string(),
      start_line: 1,
      end_line: 2,
      start_column: Some(0),
      end_column: Some(19),
    },
  )
  .unwrap();

  let req = JsonRpcRequest {
    jsonrpc: "2.0".to_string(),
    id: Some(json!(3)),
    method: "tools/call".to_string(),
    params: Some(json!({
      "name": "get_symbol",
      "arguments": {
        "qualified_name": "src.math::multiply"
      }
    })),
  };

  let response_opt = handle_request(&connection, repo_id, repo_root, &req);
  assert!(response_opt.is_some());
  let res = response_opt.unwrap();
  assert!(res.error.is_none());

  let result = res.result.unwrap();
  let content = result["content"].as_array().unwrap();
  assert_eq!(content.len(), 1);

  let text = content[0]["text"].as_str().unwrap();
  assert!(text.contains("src.math::multiply"));
  assert!(text.contains("multiply(a, b)"));
  assert!(text.contains("return a * b"));
}

#[test]
fn test_unknown_method() {
  let (connection, repo_id) = setup_test_db();
  let repo_root = Path::new("/test/repo");

  let req = JsonRpcRequest {
    jsonrpc: "2.0".to_string(),
    id: Some(json!(4)),
    method: "unknown_method".to_string(),
    params: None,
  };

  let response_opt = handle_request(&connection, repo_id, repo_root, &req);
  assert!(response_opt.is_some());
  let res = response_opt.unwrap();
  assert!(res.result.is_none());

  let error = res.error.unwrap();
  assert_eq!(error.code, -32601);
  assert!(error.message.contains("Method not found"));
}

#[test]
fn test_ping_method() {
  let (connection, repo_id) = setup_test_db();
  let repo_root = Path::new("/test/repo");

  let req = JsonRpcRequest {
    jsonrpc: "2.0".to_string(),
    id: Some(json!(5)),
    method: "ping".to_string(),
    params: None,
  };

  let response_opt = handle_request(&connection, repo_id, repo_root, &req);
  assert!(response_opt.is_some());
  let res = response_opt.unwrap();
  assert!(res.error.is_none());

  let result = res.result.unwrap();
  assert_eq!(result, json!({}));
}
