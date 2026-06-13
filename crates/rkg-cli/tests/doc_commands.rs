use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_docs_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create python file with docstrings
  let python_code = r#""""
Module docstring
"""

class Patient:
  """Class docstring"""
  def validate_patient(self):
    """Function docstring"""
    return True
"#;
  write_file(temp_dir.path(), "src/patient.py", python_code);

  // 3. Create markdown file with heading and symbol mention
  let markdown_code = r#"# validate_patient

Detailed info about the validate_patient function.
It works in conjunction with src.patient::Patient.
"#;
  write_file(temp_dir.path(), "README.md", markdown_code);

  // 4. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("files scanned: 2"));

  // 5. Query documentation for `validate_patient`
  let mut docs_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  docs_cmd
    .current_dir(temp_dir.path())
    .args(["docs", "validate_patient"]);
  let docs_output = docs_cmd.assert().success().get_output().stdout.clone();
  let docs_stdout = String::from_utf8(docs_output).expect("stdout should be valid utf8");

  // Verify it contains the docstring of the function
  assert!(docs_stdout.contains("[Docstring]"));
  assert!(docs_stdout.contains("Function docstring"));

  // Verify it contains the linked markdown section (heading same-name match)
  assert!(docs_stdout.contains("[Markdown]"));
  assert!(docs_stdout.contains("Heading: validate_patient"));
  assert!(docs_stdout.contains("Detailed info about the validate_patient function."));

  // 6. Query documentation for `Patient` class to verify qualified name mention linkage
  let mut class_docs_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  class_docs_cmd
    .current_dir(temp_dir.path())
    .args(["docs", "Patient"]);
  let class_docs_output = class_docs_cmd
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let class_docs_stdout =
    String::from_utf8(class_docs_output).expect("stdout should be valid utf8");

  // Verify it contains the class docstring
  assert!(class_docs_stdout.contains("Class docstring"));
  // Verify it contains the linked markdown section (since it mentions src.patient::Patient)
  assert!(class_docs_stdout.contains("Detailed info about the validate_patient function."));

  // 7. Search documentation for a query
  let mut search_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  search_cmd
    .current_dir(temp_dir.path())
    .args(["doc-search", "Module docstring"]);
  let search_output = search_cmd.assert().success().get_output().stdout.clone();
  let search_stdout = String::from_utf8(search_output).expect("stdout should be valid utf8");

  assert!(search_stdout.contains("src/patient.py"));
  assert!(search_stdout.contains("Module docstring"));
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
