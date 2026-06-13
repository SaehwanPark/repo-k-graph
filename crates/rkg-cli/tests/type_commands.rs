use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_types_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create python files representing types
  let base_code = r#"
class BaseModel:
  pass
"#;
  write_file(temp_dir.path(), "src/base.py", base_code);

  let patient_code = r#"
from .base import BaseModel

class Patient(BaseModel):
  def validate(self, record: MedicalRecord) -> bool:
    x: Diagnosis = Diagnosis()
    return True

class MedicalRecord:
  pass
"#;
  write_file(temp_dir.path(), "src/patient.py", patient_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("python files parsed: 2"));

  // 4. Test forward lookup: rkg types Patient
  let mut forward_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  forward_cmd
    .current_dir(temp_dir.path())
    .args(["types", "Patient"]);
  let forward_output = forward_cmd.assert().success().get_output().stdout.clone();
  let forward_stdout = String::from_utf8(forward_output).expect("stdout should be valid utf8");

  assert!(forward_stdout.contains("Types referenced by Patient:"));
  assert!(forward_stdout.contains("src.base::BaseModel [Resolved] [Class]"));

  // Query types referenced by Patient.validate using simple name "validate"
  let mut method_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  method_cmd
    .current_dir(temp_dir.path())
    .args(["types", "validate"]);
  let method_output = method_cmd.assert().success().get_output().stdout.clone();
  let method_stdout = String::from_utf8(method_output).expect("stdout should be valid utf8");

  assert!(method_stdout.contains("Types referenced by validate:"));
  assert!(method_stdout.contains("src.patient::MedicalRecord [Resolved] [Class]"));
  assert!(method_stdout.contains("Diagnosis [Unresolved]"));

  // 5. Test backward lookup: rkg types BaseModel
  let mut backward_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  backward_cmd
    .current_dir(temp_dir.path())
    .args(["types", "BaseModel"]);
  let backward_output = backward_cmd.assert().success().get_output().stdout.clone();
  let backward_stdout = String::from_utf8(backward_output).expect("stdout should be valid utf8");

  assert!(backward_stdout.contains("Symbols referencing type BaseModel:"));
  assert!(backward_stdout.contains("src.patient::Patient [Class] (File: src/patient.py)"));
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
