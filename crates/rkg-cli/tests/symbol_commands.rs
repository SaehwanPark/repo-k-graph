use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_symbols_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create python file with class and method
  let python_code = r#"class Patient:
  def validate_patient(self):
    return True
"#;
  write_file(temp_dir.path(), "src/patient.py", python_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("python files parsed: 1"));

  // 4. Test rkg symbols
  let mut symbols_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  symbols_cmd.current_dir(temp_dir.path()).arg("symbols");
  let sym_output = symbols_cmd.assert().success().get_output().stdout.clone();
  let sym_stdout = String::from_utf8(sym_output).expect("stdout should be valid utf8");
  assert!(sym_stdout.contains("src.patient [Module]"));
  assert!(sym_stdout.contains("src.patient::Patient [Class]"));
  assert!(sym_stdout.contains("src.patient::Patient.validate_patient [Method]"));

  // 5. Test rkg find validate_patient
  let mut find_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  find_cmd
    .current_dir(temp_dir.path())
    .args(["find", "validate_patient"]);
  let find_output = find_cmd.assert().success().get_output().stdout.clone();
  let find_stdout = String::from_utf8(find_output).expect("stdout should be valid utf8");
  assert!(
    find_stdout.contains("src.patient::Patient.validate_patient [Method] (src/patient.py:2-3)")
  );

  // 6. Test rkg show src.patient::Patient.validate_patient
  let mut show_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  show_cmd
    .current_dir(temp_dir.path())
    .args(["show", "src.patient::Patient.validate_patient"]);
  let show_output = show_cmd.assert().success().get_output().stdout.clone();
  let show_stdout = String::from_utf8(show_output).expect("stdout should be valid utf8");
  assert!(show_stdout.contains("Symbol: src.patient::Patient.validate_patient [Method]"));
  assert!(show_stdout.contains("File: src/patient.py (lines 2-3)"));
  assert!(show_stdout.contains("  def validate_patient(self):"));
  assert!(show_stdout.contains("    return True"));
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
