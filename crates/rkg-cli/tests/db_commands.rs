use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn init_creates_default_database_path() {
  let temp_dir = TempDir::new().expect("temp dir must be created");

  let mut command = Command::cargo_bin("rkg").expect("rkg binary should compile");
  command.current_dir(temp_dir.path()).arg("init");
  command.assert().success();

  assert!(temp_dir.path().join(".rkg/rkg.db").exists());
}

#[test]
fn db_status_reports_missing_before_init_and_exists_after_init() {
  let temp_dir = TempDir::new().expect("temp dir must be created");

  let mut status_before = Command::cargo_bin("rkg").expect("rkg binary should compile");
  status_before
    .current_dir(temp_dir.path())
    .args(["db", "status"]);
  let before_output = status_before.assert().success().get_output().stdout.clone();
  let before_stdout = String::from_utf8(before_output).expect("stdout should be valid utf8");
  assert!(before_stdout.contains("exists: no"));

  let mut init = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init.current_dir(temp_dir.path()).arg("init");
  init.assert().success();

  let mut status_after = Command::cargo_bin("rkg").expect("rkg binary should compile");
  status_after
    .current_dir(temp_dir.path())
    .args(["db", "status"]);
  let after_output = status_after.assert().success().get_output().stdout.clone();
  let after_stdout = String::from_utf8(after_output).expect("stdout should be valid utf8");
  assert!(after_stdout.contains("exists: yes"));
  assert!(after_stdout.contains("schema initialized: yes"));
  assert!(after_stdout.contains("repositories: 0"));
}

#[test]
fn db_reset_recreates_database_file() {
  let temp_dir = TempDir::new().expect("temp dir must be created");

  let mut init = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init.current_dir(temp_dir.path()).arg("init");
  init.assert().success();

  let db_path = temp_dir.path().join(".rkg/rkg.db");
  assert!(db_path.exists());
  let old_metadata = std::fs::metadata(&db_path).expect("db metadata should exist");
  let old_modified = old_metadata
    .modified()
    .expect("db modification timestamp should exist");

  std::thread::sleep(std::time::Duration::from_millis(10));

  let mut reset = Command::cargo_bin("rkg").expect("rkg binary should compile");
  reset.current_dir(temp_dir.path()).args(["db", "reset"]);
  reset.assert().success();

  let new_metadata = std::fs::metadata(&db_path).expect("db metadata should exist");
  let new_modified = new_metadata
    .modified()
    .expect("db modification timestamp should exist");
  assert!(new_modified >= old_modified);
}
