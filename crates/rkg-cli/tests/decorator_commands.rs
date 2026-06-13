use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_decorators_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create python files representing decorators
  let decorator_def_code = r#"
def route(path):
  def decorator(func):
    return func
  return decorator

def singleton(cls):
  return cls
"#;
  write_file(temp_dir.path(), "src/decorators.py", decorator_def_code);

  let patient_code = r#"
from .decorators import route, singleton

@singleton
class PatientService:
  @route("/patient")
  def validate_patient(self):
    pass
"#;
  write_file(temp_dir.path(), "src/patient.py", patient_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("python files parsed: 2"));

  // 4. Test forward lookup: rkg decorators validate_patient
  let mut forward_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  forward_cmd
    .current_dir(temp_dir.path())
    .args(["decorators", "validate_patient"]);
  let forward_output = forward_cmd.assert().success().get_output().stdout.clone();
  let forward_stdout = String::from_utf8(forward_output).expect("stdout should be valid utf8");

  assert!(forward_stdout.contains("Decorators modifying validate_patient:"));
  assert!(forward_stdout.contains("src.decorators::route [Resolved] [Function]"));

  // Test forward lookup for class: rkg decorators PatientService
  let mut class_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  class_cmd
    .current_dir(temp_dir.path())
    .args(["decorators", "PatientService"]);
  let class_output = class_cmd.assert().success().get_output().stdout.clone();
  let class_stdout = String::from_utf8(class_output).expect("stdout should be valid utf8");

  assert!(class_stdout.contains("Decorators modifying PatientService:"));
  assert!(class_stdout.contains("src.decorators::singleton [Resolved] [Function]"));

  // 5. Test backward lookup: rkg decorators route
  let mut backward_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  backward_cmd
    .current_dir(temp_dir.path())
    .args(["decorators", "route"]);
  let backward_output = backward_cmd.assert().success().get_output().stdout.clone();
  let backward_stdout = String::from_utf8(backward_output).expect("stdout should be valid utf8");

  assert!(backward_stdout.contains("Symbols decorated by route:"));
  assert!(
    backward_stdout
      .contains("src.patient::PatientService.validate_patient [Method] (File: src/patient.py)")
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
