use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_context_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create python source files with dependencies and docstring
  let src_code = r#"def bar():
  """Validate the database bar."""
  pass

def foo():
  bar()
"#;
  write_file(temp_dir.path(), "src/main.py", src_code);

  let test_code = r#"from src.main import foo

def test_foo():
  foo()
"#;
  write_file(temp_dir.path(), "tests/test_main.py", test_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  index_cmd.assert().success();

  // 4. Test rkg context bar (default Markdown format)
  let mut context_md_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  context_md_cmd
    .current_dir(temp_dir.path())
    .args(["context", "bar"]);

  let output_md = context_md_cmd
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let stdout_md = String::from_utf8(output_md).expect("stdout should be valid utf8");

  // Validate Markdown structure and content
  assert!(stdout_md.contains("# rkg Context Pack: bar"));
  assert!(stdout_md.contains("## Discovered Files"));
  assert!(stdout_md.contains("src/main.py"));
  assert!(stdout_md.contains("## Symbol Definitions"));
  assert!(stdout_md.contains("Symbol: `src.main::bar`"));
  assert!(stdout_md.contains("def bar():"));
  assert!(stdout_md.contains("## Documentation Blocks"));
  assert!(stdout_md.contains("Validate the database bar."));
  assert!(stdout_md.contains("## Relationships"));
  assert!(stdout_md.contains("`src.main::foo` -> `Calls` -> `src.main::bar`"));

  // 5. Test rkg context bar --format json
  let mut context_json_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  context_json_cmd
    .current_dir(temp_dir.path())
    .args(["context", "bar", "--format", "json"]);

  let output_json = context_json_cmd
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let stdout_json = String::from_utf8(output_json).expect("stdout should be valid utf8");

  // Validate JSON structure and content
  assert!(stdout_json.contains("\"target\": \"bar\""));
  assert!(stdout_json.contains("\"estimated_tokens\""));
  assert!(stdout_json.contains("\"qualified_name\": \"src.main::bar\""));
  assert!(stdout_json.contains("Validate the database bar."));
  assert!(stdout_json.contains("\"kind\": \"Calls\""));

  // 6. Test rkg context bar --budget 50 (budget trimming)
  let mut context_budget_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  context_budget_cmd
    .current_dir(temp_dir.path())
    .args(["context", "bar", "--budget", "150"]);

  let output_budget = context_budget_cmd
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let stdout_budget = String::from_utf8(output_budget).expect("stdout should be valid utf8");

  // Under small budget, some larger/less-priority context should be pruned, but target symbol remains
  assert!(stdout_budget.contains("# rkg Context Pack: bar"));
  assert!(stdout_budget.contains("Symbol: `src.main::bar`"));
}

fn setup_repo(root: &std::path::Path) {
  fs::create_dir(root.join(".git")).expect(".git directory should be created");
}

fn write_file(root: &std::path::Path, relative_path: &str, content: &str) {
  let path = root.join(relative_path);
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).expect("parent directories should be created");
  }
  fs::write(path, content).expect("file should be written");
}
