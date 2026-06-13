use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn index_and_files_report_incremental_progress() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());
  write_file(temp_dir.path(), "src/a.py", "print('a')\n");
  write_file(temp_dir.path(), "src/b.py", "print('b')\n");

  let mut first_index = Command::cargo_bin("rkg").expect("rkg binary should compile");
  first_index.current_dir(temp_dir.path()).arg("index");
  let first_output = first_index.assert().success().get_output().stdout.clone();
  let first_stdout = String::from_utf8(first_output).expect("stdout should be valid utf8");
  assert!(first_stdout.contains("files scanned: 2"));
  assert!(first_stdout.contains("files changed: 2"));
  assert!(first_stdout.contains("files unchanged: 0"));
  assert!(first_stdout.contains("files deleted: 0"));
  assert!(first_stdout.contains("python files parsed: 2"));
  assert!(first_stdout.contains("python files with syntax errors: 0"));
  assert!(first_stdout.contains("python syntax errors: 0"));
  assert!(first_stdout.contains("mode: incremental"));

  let mut second_index = Command::cargo_bin("rkg").expect("rkg binary should compile");
  second_index.current_dir(temp_dir.path()).arg("index");
  let second_output = second_index.assert().success().get_output().stdout.clone();
  let second_stdout = String::from_utf8(second_output).expect("stdout should be valid utf8");
  assert!(second_stdout.contains("files scanned: 2"));
  assert!(second_stdout.contains("files changed: 0"));
  assert!(second_stdout.contains("files unchanged: 2"));
  assert!(second_stdout.contains("files deleted: 0"));
  assert!(second_stdout.contains("python files parsed: 0"));
  assert!(second_stdout.contains("python syntax errors: 0"));

  let mut files_command = Command::cargo_bin("rkg").expect("rkg binary should compile");
  files_command.current_dir(temp_dir.path()).arg("files");
  let files_output = files_command.assert().success().get_output().stdout.clone();
  let files_stdout = String::from_utf8(files_output).expect("stdout should be valid utf8");
  assert_eq!(files_stdout, "src/a.py\nsrc/b.py\n");
}

#[test]
fn index_changed_alias_matches_incremental_mode() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());
  write_file(temp_dir.path(), "src/a.py", "print('a')\n");

  let mut first_index = Command::cargo_bin("rkg").expect("rkg binary should compile");
  first_index.current_dir(temp_dir.path()).arg("index");
  first_index.assert().success();

  let mut changed_index = Command::cargo_bin("rkg").expect("rkg binary should compile");
  changed_index
    .current_dir(temp_dir.path())
    .args(["index", "--changed"]);
  let output = changed_index.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("files changed: 0"));
  assert!(stdout.contains("python files parsed: 0"));
  assert!(stdout.contains("mode: incremental"));
}

#[test]
fn index_force_reindexes_all_discovered_files_and_tracks_deletions() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());
  write_file(temp_dir.path(), "src/a.py", "print('a')\n");
  write_file(temp_dir.path(), "src/b.py", "print('b')\n");

  let mut first_index = Command::cargo_bin("rkg").expect("rkg binary should compile");
  first_index.current_dir(temp_dir.path()).arg("index");
  first_index.assert().success();

  fs::remove_file(temp_dir.path().join("src/b.py")).expect("file should be removed");

  let mut force_index = Command::cargo_bin("rkg").expect("rkg binary should compile");
  force_index
    .current_dir(temp_dir.path())
    .args(["index", "--force"]);
  let output = force_index.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("files scanned: 1"));
  assert!(stdout.contains("files changed: 1"));
  assert!(stdout.contains("files unchanged: 0"));
  assert!(stdout.contains("files deleted: 1"));
  assert!(stdout.contains("python files parsed: 1"));
  assert!(stdout.contains("python files with syntax errors: 0"));
  assert!(stdout.contains("mode: force"));
}

#[test]
fn index_reports_python_parse_errors_without_failing() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());
  write_file(temp_dir.path(), "src/bad.py", "def broken(:\n  return 1\n");

  let mut index = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index.current_dir(temp_dir.path()).arg("index");
  let output = index.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");

  assert!(stdout.contains("files changed: 1"));
  assert!(stdout.contains("python files parsed: 1"));
  assert!(stdout.contains("python files with syntax errors: 1"));
  assert!(stdout.contains("python syntax errors:"));
  assert!(stdout.contains("python parse issues:"));
  assert!(stdout.contains("src/bad.py:"));
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
