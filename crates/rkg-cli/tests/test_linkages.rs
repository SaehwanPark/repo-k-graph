use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_test_linkages_correctly() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create production file and test files
  let prod_code = r#"
class PatientService:
  def validate_patient(self, name):
    return len(name) > 0
"#;
  write_file(temp_dir.path(), "src/service.py", prod_code);

  let test_code = r#"
import pytest
from src.service import PatientService

@pytest.fixture
def my_service():
  return PatientService()

class TestPatientService:
  def test_validate_patient(self, my_service):
    # Direct call to PatientService.validate_patient
    svc = my_service
    svc.validate_patient("John")
"#;
  write_file(temp_dir.path(), "tests/test_service.py", test_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  index_cmd.assert().success();

  // 4. Query tests testing PatientService
  let mut tests_cmd1 = Command::cargo_bin("rkg").expect("rkg binary should compile");
  let output1 = tests_cmd1
    .current_dir(temp_dir.path())
    .args(["tests", "PatientService"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let stdout1 = String::from_utf8(output1).expect("stdout must be valid utf8");
  // TestPatientService matches PatientService via name similarity
  assert!(stdout1.contains("tests.test_service::TestPatientService"));

  // 5. Query tests testing validate_patient
  let mut tests_cmd2 = Command::cargo_bin("rkg").expect("rkg binary should compile");
  let output2 = tests_cmd2
    .current_dir(temp_dir.path())
    .args(["tests", "validate_patient"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let stdout2 = String::from_utf8(output2).expect("stdout must be valid utf8");
  // test_validate_patient matches validate_patient via name similarity
  assert!(stdout2.contains("tests.test_service::TestPatientService.test_validate_patient"));

  // 6. Query test-deps test_validate_patient
  let mut deps_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  let output_deps = deps_cmd
    .current_dir(temp_dir.path())
    .args(["test-deps", "test_validate_patient"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let stdout_deps = String::from_utf8(output_deps).expect("stdout must be valid utf8");
  assert!(stdout_deps.contains("Implementation symbols tested by"));
  assert!(stdout_deps.contains("src.service::PatientService.validate_patient"));
  assert!(stdout_deps.contains("Fixtures used by"));
  assert!(stdout_deps.contains("tests.test_service::my_service"));

  // 7. Query fixtures test_validate_patient
  let mut fixtures_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  let output_fixtures = fixtures_cmd
    .current_dir(temp_dir.path())
    .args(["fixtures", "test_validate_patient"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let stdout_fixtures = String::from_utf8(output_fixtures).expect("stdout must be valid utf8");
  assert!(stdout_fixtures.contains("Fixtures used by"));
  assert!(stdout_fixtures.contains("tests.test_service::my_service"));
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

#[test]
fn extracts_and_queries_rust_test_linkages_correctly() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create Rust code with implementation and tests
  let rust_code = r#"
pub struct PatientService;

impl PatientService {
  pub fn validate_patient(&self, name: &str) -> bool {
    name.len() > 0
  }
}

#[cfg(test)]
mod TestPatientService {
  use super::*;

  #[test]
  fn test_validate_patient() {
    let service = PatientService;
    // Direct call to PatientService::validate_patient
    service.validate_patient("John");
  }
}
"#;
  write_file(temp_dir.path(), "src/lib.rs", rust_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  index_cmd.assert().success();

  // 4. Query tests testing PatientService
  let mut tests_cmd1 = Command::cargo_bin("rkg").expect("rkg binary should compile");
  let output1 = tests_cmd1
    .current_dir(temp_dir.path())
    .args(["tests", "PatientService"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let stdout1 = String::from_utf8(output1).expect("stdout must be valid utf8");
  // mod TestPatientService matches PatientService via name similarity / tested-by linkage
  assert!(stdout1.contains("src::lib::TestPatientService"));

  // 5. Query tests testing validate_patient
  let mut tests_cmd2 = Command::cargo_bin("rkg").expect("rkg binary should compile");
  let output2 = tests_cmd2
    .current_dir(temp_dir.path())
    .args(["tests", "validate_patient"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let stdout2 = String::from_utf8(output2).expect("stdout must be valid utf8");
  // test_validate_patient matches validate_patient
  assert!(stdout2.contains("src::lib::TestPatientService::test_validate_patient"));

  // 6. Query test-deps test_validate_patient
  let mut deps_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  let output_deps = deps_cmd
    .current_dir(temp_dir.path())
    .args(["test-deps", "test_validate_patient"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let stdout_deps = String::from_utf8(output_deps).expect("stdout must be valid utf8");
  assert!(stdout_deps.contains("Implementation symbols tested by"));
  assert!(stdout_deps.contains("src::lib::PatientService::validate_patient"));
}
