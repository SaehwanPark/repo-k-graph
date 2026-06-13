use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn test_fts5_and_combined_search_commands() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create python source file
  let python_code = r#""""
Module-level patient medical records documentation.
"""

class PatientManager:
  """Manager class for managing patient checkouts and registration."""
  def validate_patient(self):
    """Verifies that patient records exist."""
    return True
"#;
  write_file(temp_dir.path(), "src/patient.py", python_code);

  // 3. Create markdown documentation file
  let markdown_code = r#"# validate_patient

Detailed checkout guidelines for patient billing.
References the validate_patient function on PatientManager.
"#;
  write_file(temp_dir.path(), "README.md", markdown_code);

  // 4. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  index_cmd.assert().success();

  // 5. Test FTS5 doc-search
  let mut doc_search_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  doc_search_cmd
    .current_dir(temp_dir.path())
    .args(["doc-search", "checkout guidelines"]);
  let doc_search_stdout = String::from_utf8(
    doc_search_cmd
      .assert()
      .success()
      .get_output()
      .stdout
      .clone(),
  )
  .expect("stdout must be valid UTF-8");

  assert!(doc_search_stdout.contains("README.md"));
  assert!(doc_search_stdout.contains("Detailed checkout guidelines for patient billing."));

  // 6. Test FTS5 combined search
  let mut search_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  search_cmd
    .current_dir(temp_dir.path())
    .args(["search", "patient"]);
  let search_stdout = String::from_utf8(search_cmd.assert().success().get_output().stdout.clone())
    .expect("stdout must be valid UTF-8");

  assert!(search_stdout.contains("SEARCH RESULTS FOR \"PATIENT\""));
  assert!(search_stdout.contains("MATCHING SYMBOLS:"));
  assert!(search_stdout.contains("MATCHING DOCUMENTATION:"));
  assert!(search_stdout.contains("PatientManager"));
  assert!(search_stdout.contains("src/patient.py"));
  assert!(search_stdout.contains("Module-level patient medical records documentation."));

  // 7. Test graceful fallback on FTS5 syntax errors (e.g. mismatched quotes or operators)
  let mut fallback_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  fallback_cmd
    .current_dir(temp_dir.path())
    .args(["search", "patient\""]); // query with trailing unclosed quote
  let fallback_stdout =
    String::from_utf8(fallback_cmd.assert().success().get_output().stdout.clone())
      .expect("stdout must be valid UTF-8");

  // Verify that it still falls back to LIKE matching and returns matches cleanly without error
  assert!(fallback_stdout.contains("MATCHING SYMBOLS:"));
  assert!(fallback_stdout.contains("PatientManager"));

  // 8. Test FTS trigger synchronization during incremental deletion
  // Delete the source file and re-run indexer to clean up records
  fs::remove_file(temp_dir.path().join("src/patient.py")).expect("file must be deleted");

  let mut reindex_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  reindex_cmd.current_dir(temp_dir.path()).arg("index");
  reindex_cmd.assert().success();

  // Search again: PatientManager symbol should no longer be present
  let mut post_delete_search = Command::cargo_bin("rkg").expect("rkg binary should compile");
  post_delete_search
    .current_dir(temp_dir.path())
    .args(["search", "PatientManager"]);
  let post_delete_stdout = String::from_utf8(
    post_delete_search
      .assert()
      .success()
      .get_output()
      .stdout
      .clone(),
  )
  .expect("stdout must be valid UTF-8");

  assert!(!post_delete_stdout.contains("PatientManager"));
  assert!(!post_delete_stdout.contains("src/patient.py"));
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
