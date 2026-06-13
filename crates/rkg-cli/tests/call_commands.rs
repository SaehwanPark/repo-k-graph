use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_calls_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create python files representing calls
  let caller_code = r#"
class Patient:
  def __init__(self, patient_id):
    self.id = patient_id
    self.setup_logging()

  def validate(self):
    return self.check_fields()

def helper_fn():
  return global_val()

helper_fn()
"#;
  write_file(temp_dir.path(), "src/patient.py", caller_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("python files parsed: 1"));

  // 4. Test rkg callees validate
  let mut callees_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  callees_cmd
    .current_dir(temp_dir.path())
    .args(["callees", "validate"]);
  let call_output = callees_cmd.assert().success().get_output().stdout.clone();
  let call_stdout = String::from_utf8(call_output).expect("stdout should be valid utf8");

  assert!(call_stdout.contains("Patient.check_fields [Unresolved]"));

  // 5. Test rkg callers helper_fn
  let mut callers_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  callers_cmd
    .current_dir(temp_dir.path())
    .args(["callers", "helper_fn"]);
  let callers_output = callers_cmd.assert().success().get_output().stdout.clone();
  let callers_stdout = String::from_utf8(callers_output).expect("stdout should be valid utf8");

  assert!(callers_stdout.contains("src.patient [Module] (File: src/patient.py)"));
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
