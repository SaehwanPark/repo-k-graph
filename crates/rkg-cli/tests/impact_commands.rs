use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_impact_via_cli() {
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

  // 4. Test rkg impact bar --depth 2
  let mut impact_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  impact_cmd
    .current_dir(temp_dir.path())
    .args(["impact", "bar", "--depth", "2"]);

  let output = impact_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");

  // Validate matched target symbols
  assert!(stdout.contains("Target Symbol(s) matched:"));
  assert!(stdout.contains("src.main::bar [Function] (File: src/main.py, Lines: 1-3)"));

  // Validate upstream impact (foo calls bar)
  assert!(stdout.contains("UPSTREAM IMPACT (Affected Code / Backward)"));
  assert!(stdout.contains("Depth 1:"));
  assert!(stdout.contains("<- src.main::foo [Function] (File: src/main.py, Lines: 5-6) via Calls"));

  // Validate affected tests (test_foo tests foo, which transitively calls bar)
  assert!(stdout.contains("AFFECTED TESTS"));
  assert!(stdout.contains("test_foo [Function] (File: tests/test_main.py) -> Tests src.main::foo"));

  // Validate affected documentation (bar docstring)
  assert!(stdout.contains("AFFECTED DOCUMENTATION"));
  assert!(stdout.contains("File: src/main.py: Docstring (Lines: 2-2) -> Documents src.main::bar"));
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
