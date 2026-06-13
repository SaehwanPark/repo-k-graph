use std::fs;

use assert_cmd::Command;
use rusqlite::Connection;
use tempfile::TempDir;

#[test]
fn indexes_and_persists_pytest_elements_correctly() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create a test file, a fixture file, and a production file (to verify exclusion)
  let fixture_code = r#"
import pytest

@pytest.fixture
def api_client():
  return "client"
"#;
  write_file(temp_dir.path(), "tests/conftest.py", fixture_code);

  let test_code = r#"
import pytest

class TestAuthentication:
  @pytest.mark.parametrize("user,token", [("admin", "123")])
  def test_login_success(self, api_client, user, token):
    assert True

  def test_logout(self):
    pass
"#;
  write_file(temp_dir.path(), "tests/test_auth.py", test_code);

  let prod_code = r#"
class TestClient:
  pass

def test_helper():
  pass
"#;
  write_file(temp_dir.path(), "src/service.py", prod_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("python files parsed: 3"));

  // 4. Open SQLite DB directly and verify the tests table content
  let db_path = temp_dir.path().join(".rkg").join("rkg.db");
  let connection = Connection::open(db_path).expect("must be able to open SQLite DB");

  #[derive(Debug, PartialEq, Eq)]
  struct TestRow {
    name: String,
    qualified_name: String,
    kind: String,
    is_parametrized: bool,
    framework: String,
    start_line: Option<i64>,
    end_line: Option<i64>,
  }

  let mut stmt = connection
    .prepare("SELECT name, qualified_name, kind, is_parametrized, framework, start_line, end_line FROM tests ORDER BY name")
    .expect("prepare statement must succeed");

  let rows: Vec<TestRow> = stmt
    .query_map([], |row| {
      let is_parametrized_int: i64 = row.get(3)?;
      Ok(TestRow {
        name: row.get(0)?,
        qualified_name: row.get(1)?,
        kind: row.get(2)?,
        is_parametrized: is_parametrized_int != 0,
        framework: row.get(4)?,
        start_line: row.get(5)?,
        end_line: row.get(6)?,
      })
    })
    .expect("query map must succeed")
    .map(|r| r.unwrap())
    .collect();

  // We expect exactly 4 rows, as src/service.py must be excluded from test collection
  assert_eq!(rows.len(), 4);

  // Assertions
  // 1. api_client (Fixture)
  let api_client_row = rows
    .iter()
    .find(|r| r.name == "api_client")
    .expect("api_client fixture must exist");
  assert_eq!(
    api_client_row.qualified_name,
    "tests.conftest::api_client".to_string()
  );
  assert_eq!(api_client_row.kind, "Fixture");
  assert!(!api_client_row.is_parametrized);

  // 2. TestAuthentication (Class)
  let test_auth_class_row = rows
    .iter()
    .find(|r| r.name == "TestAuthentication")
    .expect("TestAuthentication class must exist");
  assert_eq!(
    test_auth_class_row.qualified_name,
    "tests.test_auth::TestAuthentication".to_string()
  );
  assert_eq!(test_auth_class_row.kind, "Class");
  assert!(!test_auth_class_row.is_parametrized);

  // 3. test_login_success (Function, parametrized)
  let test_login_row = rows
    .iter()
    .find(|r| r.name == "test_login_success")
    .expect("test_login_success must exist");
  assert_eq!(
    test_login_row.qualified_name,
    "tests.test_auth::TestAuthentication.test_login_success".to_string()
  );
  assert_eq!(test_login_row.kind, "Function");
  assert!(test_login_row.is_parametrized);

  // 4. test_logout (Function, not parametrized)
  let test_logout_row = rows
    .iter()
    .find(|r| r.name == "test_logout")
    .expect("test_logout must exist");
  assert_eq!(
    test_logout_row.qualified_name,
    "tests.test_auth::TestAuthentication.test_logout".to_string()
  );
  assert_eq!(test_logout_row.kind, "Function");
  assert!(!test_logout_row.is_parametrized);
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
fn indexes_and_persists_rust_tests_correctly() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create lib.rs with implementation and unit tests
  let lib_code = r#"
pub fn add(x: i32, y: i32) -> i32 {
  x + y
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_add() {
    assert_eq!(add(1, 2), 3);
  }
}
"#;
  write_file(temp_dir.path(), "src/lib.rs", lib_code);

  // 3. Create integration test
  let integration_code = r#"
#[test]
fn test_integration_add() {
  assert!(true);
}
"#;
  write_file(
    temp_dir.path(),
    "tests/integration_add.rs",
    integration_code,
  );

  // 4. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");

  // We expect both Rust files parsed
  assert!(stdout.contains("scanned: 2") || stdout.contains("scanned: 3"));

  // 5. Open SQLite DB directly and verify the tests table content
  let db_path = temp_dir.path().join(".rkg").join("rkg.db");
  let connection = Connection::open(db_path).expect("must be able to open SQLite DB");

  #[derive(Debug, PartialEq, Eq)]
  struct TestRow {
    name: String,
    qualified_name: String,
    kind: String,
    framework: String,
  }

  let mut stmt = connection
    .prepare("SELECT name, qualified_name, kind, framework FROM tests ORDER BY name")
    .expect("prepare statement must succeed");

  let rows: Vec<TestRow> = stmt
    .query_map([], |row| {
      Ok(TestRow {
        name: row.get(0)?,
        qualified_name: row.get(1)?,
        kind: row.get(2)?,
        framework: row.get(3)?,
      })
    })
    .expect("query map must succeed")
    .map(|r| r.unwrap())
    .collect();

  // We expect exactly 3 rows: mod tests, test_add, test_integration_add
  assert_eq!(rows.len(), 3);

  // 1. tests (Class)
  let tests_mod = rows
    .iter()
    .find(|r| r.name == "tests")
    .expect("tests module must exist");
  assert_eq!(tests_mod.qualified_name, "src::lib::tests");
  assert_eq!(tests_mod.kind, "Class");
  assert_eq!(tests_mod.framework, "cargo");

  // 2. test_add (Function)
  let test_add = rows
    .iter()
    .find(|r| r.name == "test_add")
    .expect("test_add must exist");
  assert_eq!(test_add.qualified_name, "src::lib::tests::test_add");
  assert_eq!(test_add.kind, "Function");
  assert_eq!(test_add.framework, "cargo");

  // 3. test_integration_add (Function)
  let test_integration = rows
    .iter()
    .find(|r| r.name == "test_integration_add")
    .expect("test_integration_add must exist");
  assert_eq!(
    test_integration.qualified_name,
    "tests::integration_add::test_integration_add"
  );
  assert_eq!(test_integration.kind, "Function");
  assert_eq!(test_integration.framework, "cargo");
}
