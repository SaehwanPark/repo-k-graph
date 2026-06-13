use std::fs;
use std::process::Command as StdCommand;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_git_intelligence_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");

  // 1. Initialize Git Repository
  run_git(temp_dir.path(), &["init"]);
  run_git(temp_dir.path(), &["config", "user.name", "Test Author"]);
  run_git(
    temp_dir.path(),
    &["config", "user.email", "test@example.com"],
  );
  run_git(temp_dir.path(), &["config", "commit.gpgsign", "false"]);

  // 2. Initialize rkg DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 3. Write first version of src/utils.py
  let code1 = r#"def add_numbers(a, b):
  return a + b
"#;
  write_file(temp_dir.path(), "src/utils.py", code1);

  // Commit 1
  run_git(temp_dir.path(), &["add", "."]);
  run_git(
    temp_dir.path(),
    &["commit", "-m", "Initial commit with utils"],
  );

  // 4. Write second version modifying src/utils.py and adding src/main.py
  let code1_mod = r#"def add_numbers(a, b):
  # Modified comment
  return a + b

def subtract_numbers(a, b):
  return a - b
"#;
  write_file(temp_dir.path(), "src/utils.py", code1_mod);

  let code2 = r#"from src.utils import add_numbers

def run_main():
  print(add_numbers(1, 2))
"#;
  write_file(temp_dir.path(), "src/main.py", code2);

  // Commit 2 (Co-changing commit!)
  run_git(temp_dir.path(), &["add", "."]);
  run_git(
    temp_dir.path(),
    &["commit", "-m", "Modify utils and add main"],
  );

  // 5. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().clone();
  println!("INDEX STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));

  // 6. Test rkg git src/utils.py
  let mut git_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  git_cmd
    .current_dir(temp_dir.path())
    .args(["git", "src/utils.py"]);
  let git_output = git_cmd.assert().success().get_output().stdout.clone();
  let git_stdout = String::from_utf8(git_output).expect("stdout must be utf8");

  assert!(git_stdout.contains("Git Metadata for src/utils.py:"));
  assert!(git_stdout.contains("File Churn: 2 commits"));
  assert!(git_stdout.contains("Last Modified:"));
  assert!(git_stdout.contains("Author:  Test Author <test@example.com>"));
  assert!(git_stdout.contains("Modify utils and add main"));
  assert!(git_stdout.contains("Author Frequency:"));
  assert!(git_stdout.contains("Test Author <test@example.com>: 2 commits (100.0%)"));

  // 7. Test rkg cochange src/utils.py
  let mut cochange_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  cochange_cmd
    .current_dir(temp_dir.path())
    .args(["cochange", "src/utils.py"]);
  let cochange_output = cochange_cmd.assert().success().get_output().stdout.clone();
  let cochange_stdout = String::from_utf8(cochange_output).expect("stdout must be utf8");

  assert!(cochange_stdout.contains("Co-change Analysis for 'src/utils.py':"));
  assert!(cochange_stdout.contains("Churn: 2 commits"));
  assert!(cochange_stdout.contains("Files Changed Together:"));
  assert!(cochange_stdout.contains("src/main.py: 1 times (50.0% co-change rate)"));

  // 8. Test rkg cochange add_numbers
  let mut sym_cochange_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  sym_cochange_cmd
    .current_dir(temp_dir.path())
    .args(["cochange", "add_numbers"]);
  let sym_output = sym_cochange_cmd
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let sym_stdout = String::from_utf8(sym_output).expect("stdout must be utf8");

  assert!(sym_stdout.contains("Co-change Analysis for 'src.utils::add_numbers':"));
  assert!(sym_stdout.contains("Churn: 2 commits"));
  assert!(sym_stdout.contains("Symbols Changed Together:"));
  assert!(sym_stdout.contains("src.utils::subtract_numbers: 1 times (50.0% co-change rate)"));
  assert!(sym_stdout.contains("Files Changed Together:"));
  assert!(sym_stdout.contains("src/main.py: 1 times (50.0% co-change rate)"));
}

fn run_git(root: &std::path::Path, args: &[&str]) {
  let output = StdCommand::new("git")
    .args(args)
    .current_dir(root)
    .output()
    .expect("git command execution failed");

  if !output.status.success() {
    panic!(
      "git command failed with stderr: {}",
      String::from_utf8_lossy(&output.stderr)
    );
  }
}

fn write_file(root: &std::path::Path, relative_path: &str, content: &str) {
  let path = root.join(relative_path);
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).expect("parent directories must be created");
  }
  fs::write(path, content).expect("file must be written");
}
