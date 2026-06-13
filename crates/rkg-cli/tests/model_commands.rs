use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_models_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create python files representing Pydantic models with dependencies
  let models_code = r#"
from pydantic import BaseModel, Field, field_validator, model_validator

class Address(BaseModel):
    city: str
    state: str

class Patient(BaseModel):
    id: int
    name: str = Field(..., max_length=50)
    age: int = 30
    address: Address

    @field_validator("name")
    def name_must_contain_space(cls, v):
        return v

    @model_validator(mode="before")
    def check_all(cls, data):
        return data
"#;
  write_file(temp_dir.path(), "src/models.py", models_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("python files parsed: 1"));

  // 4. Test model query command: rkg model Patient
  let mut model_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  model_cmd
    .current_dir(temp_dir.path())
    .arg("model")
    .arg("Patient");
  let model_output = model_cmd.assert().success().get_output().stdout.clone();
  let model_stdout = String::from_utf8(model_output).expect("stdout should be valid utf8");

  // Verify headers/structure
  assert!(model_stdout.contains("Model: src.models::Patient"));
  assert!(model_stdout.contains("Fields:"));
  assert!(model_stdout.contains("- id: int (required)"));
  assert!(model_stdout.contains("- name: str = Field(..., max_length=50) (required)"));
  assert!(model_stdout.contains("- age: int = 30 (optional)"));
  assert!(model_stdout.contains("- address: Address (required)"));

  assert!(model_stdout.contains("Validators:"));
  assert!(model_stdout.contains("- name_must_contain_space (validates: name)"));
  assert!(model_stdout.contains("- check_all (model validator)"));

  assert!(model_stdout.contains("Dependencies:"));
  assert!(model_stdout.contains("- src.models::Address"));
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
