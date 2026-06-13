use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_mojo_relationships_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create target source file containing the resolved type and function
  let target_mojo = r#"
struct TargetClass:
    fn __init__(out self):
        pass

fn target_fn():
    pass
"#;

  // 3. Create caller source file referencing them
  let main_mojo = r#"
import sys
from python import Python

fn caller_fn(x: TargetClass):
    target_fn()
"#;

  write_file(temp_dir.path(), "src/target.mojo", target_mojo);
  write_file(temp_dir.path(), "src/main.mojo", main_mojo);

  // 4. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  index_cmd.assert().success();

  // 5. Query Imports for main.mojo
  let mut imports_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  imports_cmd
    .current_dir(temp_dir.path())
    .args(["imports", "src/main.mojo"]);
  let imp_output = imports_cmd.assert().success().get_output().stdout.clone();
  let imp_stdout = String::from_utf8(imp_output).expect("stdout should be valid utf8");
  assert!(imp_stdout.contains("sys [Unresolved]"));
  assert!(imp_stdout.contains("python.Python [Unresolved]"));

  // 6. Query Callers / Callees for target_fn
  let mut callers_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  callers_cmd
    .current_dir(temp_dir.path())
    .args(["callers", "target_fn"]);
  let callers_output = callers_cmd.assert().success().get_output().stdout.clone();
  let callers_stdout = String::from_utf8(callers_output).expect("stdout should be valid utf8");
  assert!(callers_stdout.contains("src.main::caller_fn"));

  let mut callees_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  callees_cmd
    .current_dir(temp_dir.path())
    .args(["callees", "caller_fn"]);
  let callees_output = callees_cmd.assert().success().get_output().stdout.clone();
  let callees_stdout = String::from_utf8(callees_output).expect("stdout should be valid utf8");
  assert!(callees_stdout.contains("src.target::target_fn"));

  // 7. Query Types for TargetClass
  let mut types_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  types_cmd
    .current_dir(temp_dir.path())
    .args(["types", "TargetClass"]);
  let types_output = types_cmd.assert().success().get_output().stdout.clone();
  let types_stdout = String::from_utf8(types_output).expect("stdout should be valid utf8");
  assert!(types_stdout.contains("Symbols referencing type TargetClass:"));
  assert!(types_stdout.contains("src.main::caller_fn"));
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
