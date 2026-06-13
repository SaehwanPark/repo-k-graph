use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_mojo_tests_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create Mojo implementation file containing a symbol
  let patient_mojo = r#"
fn validate_patient(age: Int) -> Bool:
    return age >= 0
"#;

  // 3. Create Mojo test file containing test classes and functions
  let test_mojo = r#"
struct TestPatientValidator:
    fn test_validate_patient(self, mock_patient_fixture: String):
        validate_patient(42)

fn test_validate_patient_invalid():
    validate_patient(-1)
"#;

  write_file(temp_dir.path(), "src/patient.mojo", patient_mojo);
  write_file(temp_dir.path(), "tests/test_patient.mojo", test_mojo);

  // 4. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  index_cmd.assert().success();

  // 5. Query all tests in CLI
  let mut tests_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  tests_cmd
    .current_dir(temp_dir.path())
    .args(["tests", "validate_patient"]); // Search by similar name or tested symbol
  let tests_output = tests_cmd.assert().success().get_output().stdout.clone();
  let tests_stdout = String::from_utf8(tests_output).expect("stdout should be valid utf8");
  println!("EXTRACTED MOJO TESTS:\n{}", tests_stdout);

  // Assert both test class and test function are discovered
  assert!(tests_stdout.contains("tests.test_patient::TestPatientValidator.test_validate_patient"));
  assert!(tests_stdout.contains("tests.test_patient::test_validate_patient_invalid"));

  // 6. Query test dependencies (fixtures)
  let mut test_deps_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  test_deps_cmd.current_dir(temp_dir.path()).args([
    "test-deps",
    "tests.test_patient::TestPatientValidator.test_validate_patient",
  ]);
  let deps_output = test_deps_cmd.assert().success().get_output().stdout.clone();
  let deps_stdout = String::from_utf8(deps_output).expect("stdout should be valid utf8");
  assert!(deps_stdout.contains("mock_patient_fixture [Unresolved]"));
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
