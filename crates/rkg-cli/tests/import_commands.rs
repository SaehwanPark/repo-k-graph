use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_imports_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create python files representing imports
  let importer_code = r#"import sys
from src.imported import func
"#;
  let imported_code = r#"def func():
  return 42
"#;
  write_file(temp_dir.path(), "src/importer.py", importer_code);
  write_file(temp_dir.path(), "src/imported.py", imported_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("python files parsed: 2"));

  // 4. Test rkg imports src/importer.py
  let mut imports_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  imports_cmd
    .current_dir(temp_dir.path())
    .args(["imports", "src/importer.py"]);
  let imp_output = imports_cmd.assert().success().get_output().stdout.clone();
  let imp_stdout = String::from_utf8(imp_output).expect("stdout should be valid utf8");

  assert!(imp_stdout.contains("sys [Unresolved]"));
  assert!(imp_stdout.contains("src.imported::func [Resolved] (File: src/imported.py)"));

  // 5. Test rkg imported-by src/imported.py
  let mut imported_by_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  imported_by_cmd
    .current_dir(temp_dir.path())
    .args(["imported-by", "src/imported.py"]);
  let by_output = imported_by_cmd
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let by_stdout = String::from_utf8(by_output).expect("stdout should be valid utf8");

  assert!(by_stdout.contains("src.importer (src/importer.py)"));
}

#[test]
fn handles_duplicate_imports_and_incremental_reindexing_correctly() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create python files representing duplicate imports
  let importer_code = r#"import sys
import sys
from src.imported import func
from src.imported import func
"#;
  let imported_code = r#"def func():
  return 42
"#;
  write_file(temp_dir.path(), "src/importer.py", importer_code);
  write_file(temp_dir.path(), "src/imported.py", imported_code);

  // 3. Index repository first time (should succeed despite duplicate imports due to deduplication)
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  index_cmd.assert().success();

  // 4. Test rkg imports src/importer.py (verify deduplicated imports)
  let mut imports_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  imports_cmd
    .current_dir(temp_dir.path())
    .args(["imports", "src/importer.py"]);
  let imp_stdout =
    String::from_utf8(imports_cmd.assert().success().get_output().stdout.clone()).unwrap();

  // sys should only show once, and func should only show once
  // Since we print them, let's verify they exist in the output
  assert!(imp_stdout.contains("sys [Unresolved]"));
  assert!(imp_stdout.contains("src.imported::func [Resolved] (File: src/imported.py)"));

  // Check count of "sys [Unresolved]" in stdout
  let sys_count = imp_stdout.matches("sys [Unresolved]").count();
  assert_eq!(sys_count, 1, "sys import should be deduplicated");

  // 5. Test imported-by src/imported.py (verify resolved correctly)
  let mut imported_by_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  imported_by_cmd
    .current_dir(temp_dir.path())
    .args(["imported-by", "src/imported.py"]);
  let by_stdout = String::from_utf8(
    imported_by_cmd
      .assert()
      .success()
      .get_output()
      .stdout
      .clone(),
  )
  .unwrap();
  assert!(by_stdout.contains("src.importer (src/importer.py)"));

  // 6. Modify the imported file (simulate file change, e.g. updating a line to trigger reindexing)
  let imported_code_new = r#"def func():
  # some comment
  return 43
"#;
  write_file(temp_dir.path(), "src/imported.py", imported_code_new);

  // 7. Index repository again (incremental run)
  let mut index_cmd2 = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd2.current_dir(temp_dir.path()).arg("index");
  index_cmd2.assert().success();

  // 8. Test imported-by src/imported.py again
  // Even though src/importer.py was NOT changed, its incoming edge to src/imported.py
  // should have been converted to unresolved, and then re-resolved during the resolution pass of the second index run!
  let mut imported_by_cmd2 = Command::cargo_bin("rkg").expect("rkg binary should compile");
  imported_by_cmd2
    .current_dir(temp_dir.path())
    .args(["imported-by", "src/imported.py"]);
  let by_stdout2 = String::from_utf8(
    imported_by_cmd2
      .assert()
      .success()
      .get_output()
      .stdout
      .clone(),
  )
  .unwrap();
  assert!(
    by_stdout2.contains("src.importer (src/importer.py)"),
    "Incoming edge from unchanged importer should be preserved after reindexing imported file"
  );
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
