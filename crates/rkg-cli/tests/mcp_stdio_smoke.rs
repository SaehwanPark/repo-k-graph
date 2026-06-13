use std::fs;
use std::io::Write;
use std::process::{Command as StdCommand, Stdio};
use std::time::{Duration, Instant};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

const TRANSCRIPTS: &[(&str, &str)] = &[
  (
    "codex-basic",
    include_str!("fixtures/mcp-transcripts/codex-basic.jsonl"),
  ),
  (
    "claude-code-basic",
    include_str!("fixtures/mcp-transcripts/claude-code-basic.jsonl"),
  ),
  (
    "forgecode-error",
    include_str!("fixtures/mcp-transcripts/forgecode-error.jsonl"),
  ),
  (
    "antigravity-basic",
    include_str!("fixtures/mcp-transcripts/antigravity-basic.jsonl"),
  ),
];
const MCP_SERVER_TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn mcp_stdio_transcripts_keep_stdout_protocol_clean() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_indexed_repo(temp_dir.path());

  for (name, transcript) in TRANSCRIPTS {
    let output = run_mcp_transcript(temp_dir.path(), transcript);
    assert!(
      output.status.success(),
      "{name} transcript should exit successfully; stderr: {}",
      String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    let responses = parse_stdout_json_lines(name, &stdout);

    assert!(
      stderr.contains("rkg-mcp server started"),
      "{name} should keep diagnostics on stderr"
    );
    assert_transcript_semantics(name, &responses);
  }
}

fn setup_indexed_repo(root: &std::path::Path) {
  let git_status = StdCommand::new("git")
    .current_dir(root)
    .args(["init", "--quiet"])
    .status()
    .expect("git init should run");
  assert!(git_status.success(), "git init should succeed");

  write_file(
    root,
    "src/math.py",
    "def calculate_sum(a, b):\n  return a + b\n",
  );

  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(root).arg("init");
  init_cmd.assert().success();

  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(root).arg("index");
  index_cmd.assert().success();
}

fn write_file(root: &std::path::Path, relative_path: &str, content: &str) {
  let path = root.join(relative_path);
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).expect("parent directories should be created");
  }
  fs::write(path, content).expect("file should be written");
}

fn run_mcp_transcript(root: &std::path::Path, transcript: &str) -> std::process::Output {
  let binary_path = assert_cmd::cargo::cargo_bin("rkg");
  let mut child = StdCommand::new(binary_path)
    .current_dir(root)
    .args(["mcp", "serve"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("mcp server process should spawn");

  {
    let stdin = child.stdin.as_mut().expect("stdin should be piped");
    stdin
      .write_all(transcript.as_bytes())
      .expect("transcript should write to stdin");
    stdin
      .write_all(b"\n")
      .expect("final newline should write to stdin");
  }
  drop(child.stdin.take());

  let deadline = Instant::now() + MCP_SERVER_TIMEOUT;
  loop {
    if child
      .try_wait()
      .expect("mcp server process status should be readable")
      .is_some()
    {
      return child
        .wait_with_output()
        .expect("mcp server output should be collected");
    }

    if Instant::now() >= deadline {
      child
        .kill()
        .expect("hung mcp server process should be killable");
      let output = child
        .wait_with_output()
        .expect("killed mcp server output should be collected");
      panic!(
        "mcp server did not exit within {:?}; stdout: {}; stderr: {}",
        MCP_SERVER_TIMEOUT,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
      );
    }

    std::thread::sleep(Duration::from_millis(10));
  }
}

fn parse_stdout_json_lines(transcript_name: &str, stdout: &str) -> Vec<Value> {
  let lines: Vec<&str> = stdout.lines().collect();
  assert!(
    !lines.is_empty(),
    "{transcript_name} should produce JSON-RPC responses"
  );

  lines
    .into_iter()
    .map(|line| {
      serde_json::from_str(line).unwrap_or_else(|error| {
        panic!(
          "{transcript_name} stdout line should be protocol JSON only: {line:?}; parse error: {error}"
        )
      })
    })
    .collect()
}

fn assert_transcript_semantics(transcript_name: &str, responses: &[Value]) {
  match transcript_name {
    "codex-basic" => {
      assert_eq!(
        responses.len(),
        3,
        "initialized notification has no response"
      );
      assert_initialize_response(&responses[0]);
      assert_tools_list_response(&responses[1]);
      assert_tool_text_contains(&responses[2], "src.math::calculate_sum");
    }
    "claude-code-basic" => {
      assert_eq!(responses.len(), 3);
      assert_initialize_response(&responses[0]);
      assert_eq!(responses[1]["result"], serde_json::json!({}));
      assert_tool_text_contains(&responses[2], "return a + b");
    }
    "forgecode-error" => {
      assert_eq!(responses.len(), 2);
      assert_initialize_response(&responses[0]);
      assert_eq!(responses[1]["error"]["code"], -32603);
      assert!(
        responses[1]["error"]["message"]
          .as_str()
          .unwrap_or_default()
          .contains("invalid arguments")
      );
    }
    "antigravity-basic" => {
      assert_eq!(responses.len(), 3);
      assert_initialize_response(&responses[0]);
      assert_tools_list_response(&responses[1]);
      assert_eq!(
        responses[2]["id"],
        serde_json::json!({"client": "antigravity", "seq": 3})
      );
      assert_tool_text_contains(&responses[2], "Function");
    }
    other => panic!("unexpected transcript name: {other}"),
  }
}

fn assert_initialize_response(response: &Value) {
  assert_eq!(response["jsonrpc"], "2.0");
  assert_eq!(response["result"]["serverInfo"]["name"], "rkg-mcp");
  assert_eq!(response["result"]["protocolVersion"], "2024-11-05");
}

fn assert_tools_list_response(response: &Value) {
  let tools = response["result"]["tools"]
    .as_array()
    .expect("tools/list response should include tools array");
  let names: Vec<&str> = tools
    .iter()
    .filter_map(|tool| tool["name"].as_str())
    .collect();

  for expected in [
    "find_symbol",
    "get_symbol",
    "get_callers",
    "get_callees",
    "get_docs",
    "get_tests",
    "get_impact_analysis",
    "get_context_pack",
  ] {
    assert!(
      names.contains(&expected),
      "tools/list should include {expected}"
    );
  }
}

fn assert_tool_text_contains(response: &Value, expected: &str) {
  let text = response["result"]["content"][0]["text"]
    .as_str()
    .expect("tool response should contain text content");
  assert!(
    text.contains(expected),
    "tool response text should contain {expected:?}; text was {text:?}"
  );
}
