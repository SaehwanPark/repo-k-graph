use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_rust_relationships_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create Rust files representing imports, calls, types, trait implements
  let helper_code = r#"
pub trait Executor {
  fn execute(&self);
}

pub struct SimpleExecutor;

impl Executor for SimpleExecutor {
  fn execute(&self) {
    println!("Executing!");
  }
}
"#;

  let main_code = r#"
use helper::SimpleExecutor;
use helper::Executor;

pub fn run_executor(exec: SimpleExecutor) {
  exec.execute();
}
"#;

  write_file(temp_dir.path(), "src/helper.rs", helper_code);
  write_file(temp_dir.path(), "src/main.rs", main_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  println!("INDEX STDOUT: {stdout}");

  // 4. Query imports for src/main.rs
  let mut imports_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  imports_cmd
    .current_dir(temp_dir.path())
    .args(["imports", "src/main.rs"]);
  let imp_output = imports_cmd.assert().success().get_output().stdout.clone();
  let imp_stdout = String::from_utf8(imp_output).expect("stdout should be valid utf8");
  println!("IMPORTS STDOUT: {imp_stdout}");

  assert!(
    imp_stdout.contains("helper::SimpleExecutor [Resolved]")
      || imp_stdout.contains("SimpleExecutor")
  );

  // 5. Query callers of execute method
  let mut callers_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  callers_cmd
    .current_dir(temp_dir.path())
    .args(["callers", "execute"]);
  let callers_output = callers_cmd.assert().success().get_output().stdout.clone();
  let callers_stdout = String::from_utf8(callers_output).expect("stdout should be valid utf8");
  println!("CALLERS STDOUT: {callers_stdout}");

  // 6. Query type references of SimpleExecutor
  let mut types_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  types_cmd
    .current_dir(temp_dir.path())
    .args(["types", "SimpleExecutor"]);
  let types_output = types_cmd.assert().success().get_output().stdout.clone();
  let types_stdout = String::from_utf8(types_output).expect("stdout should be valid utf8");
  println!("TYPES STDOUT: {types_stdout}");

  // 7. Verify Implements edge in SQLite DB directly
  let db_path = temp_dir.path().join(".rkg").join("rkg.db");
  let conn = rusqlite::Connection::open(db_path).expect("failed to open sqlite DB");
  let mut stmt = conn
    .prepare(
      "SELECT e.kind, s1.qualified_name, s2.qualified_name, e.unresolved_target
     FROM edges e
     INNER JOIN symbols s1 ON e.source_symbol_id = s1.id
     LEFT JOIN symbols s2 ON e.target_symbol_id = s2.id
     WHERE e.kind = 'Implements'",
    )
    .expect("failed to prepare query");

  let mut rows = stmt.query([]).expect("failed to query edges");
  let mut found_implements = false;
  while let Some(row) = rows.next().expect("failed to fetch row") {
    let _kind: String = row.get(0).unwrap();
    let source_qname: String = row.get(1).unwrap();
    let target_qname: Option<String> = row.get(2).unwrap();
    let unresolved: Option<String> = row.get(3).unwrap();
    println!(
      "IMPLEMENTS EDGE: source={}, target={:?}, unresolved={:?}",
      source_qname, target_qname, unresolved
    );
    if source_qname == "src::helper::SimpleExecutor"
      && (target_qname == Some("src::helper::Executor".to_string())
        || unresolved == Some("Executor".to_string()))
    {
      found_implements = true;
    }
  }
  assert!(
    found_implements,
    "Implements relationship was not found in DB!"
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
