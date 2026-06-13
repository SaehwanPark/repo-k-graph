use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_kotlin_relationships_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create Kotlin files
  let db_content = r#"
    package com.example.db
    class Database {
      fun query(sql: String) {}
    }
  "#;

  let patient_content = r#"
    package com.example.patient
    import com.example.db.Database
    import com.example.log.Logger as SimpleLogger

    @Entity
    class Patient(val id: String) : Person(), Validatable {
      fun validate(db: Database) {
        val logger = SimpleLogger()
        logger.info(id)
        db.query("SELECT *")
      }

      inner class Detail : InfoRecord()

      companion object : Factory() {
        fun createDefault(): Patient = Patient("default")
      }
    }

    open class Person
    interface Validatable
    open class InfoRecord
    open class Factory
  "#;

  let logger_content = r#"
    package com.example.log
    class Logger {
      fun info(msg: String) {}
    }
  "#;

  write_file(temp_dir.path(), "src/db/Database.kt", db_content);
  write_file(temp_dir.path(), "src/patient/Patient.kt", patient_content);
  write_file(temp_dir.path(), "src/log/Logger.kt", logger_content);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  println!("=== index_stdout ===\n{}", stdout);
  assert!(stdout.contains("files scanned: 3"));

  // 4. Test rkg imports src/patient/Patient.kt
  let mut imports_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  imports_cmd
    .current_dir(temp_dir.path())
    .args(["imports", "src/patient/Patient.kt"]);
  let imp_output = imports_cmd.assert().success().get_output().stdout.clone();
  let imp_stdout = String::from_utf8(imp_output).expect("stdout should be valid utf8");
  println!("=== imp_stdout ===\n{}", imp_stdout);
  assert!(imp_stdout.contains("com.example.db::Database"));
  assert!(imp_stdout.contains("com.example.log::Logger"));

  // 5. Test rkg callers info
  let mut callers_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  callers_cmd
    .current_dir(temp_dir.path())
    .args(["callers", "com.example.log::Logger.info"]);
  let call_output = callers_cmd.assert().success().get_output().stdout.clone();
  let call_stdout = String::from_utf8(call_output).expect("stdout should be valid utf8");
  println!("=== call_stdout ===\n{}", call_stdout);
  assert!(call_stdout.contains("com.example.patient::Patient.validate"));

  // 6. Test rkg types com.example.db::Database
  let mut types_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  types_cmd
    .current_dir(temp_dir.path())
    .args(["types", "com.example.db::Database"]);
  let types_output = types_cmd.assert().success().get_output().stdout.clone();
  let types_stdout = String::from_utf8(types_output).expect("stdout should be valid utf8");
  println!("=== types_stdout ===\n{}", types_stdout);
  assert!(types_stdout.contains("com.example.patient::Patient.validate"));

  // 7. Test rkg decorators on com.example.patient::Patient
  let mut dec_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  dec_cmd
    .current_dir(temp_dir.path())
    .args(["decorators", "com.example.patient::Patient"]);
  let dec_output = dec_cmd.assert().success().get_output().stdout.clone();
  let dec_stdout = String::from_utf8(dec_output).expect("stdout should be valid utf8");
  println!("=== dec_stdout ===\n{}", dec_stdout);
  assert!(dec_stdout.contains("Entity"));

  // 8. Direct database verification for extends/implements
  let db_conn = rusqlite::Connection::open(temp_dir.path().join(".rkg/rkg.db")).unwrap();
  let mut stmt = db_conn
    .prepare(
      "
    SELECT s1.qualified_name, s2.qualified_name, e.kind
    FROM edges e
    INNER JOIN symbols s1 ON e.source_symbol_id = s1.id
    INNER JOIN symbols s2 ON e.target_symbol_id = s2.id
  ",
    )
    .unwrap();
  let edge_rows = stmt
    .query_map([], |row| {
      Ok((
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
      ))
    })
    .unwrap()
    .collect::<Result<Vec<_>, _>>()
    .unwrap();

  println!("=== Indexed Edges in DB ===");
  for (src, tgt, kind) in &edge_rows {
    println!("  {src} --[{kind}]--> {tgt}");
  }

  // Check Extends / Implements edges for nested classes and objects
  assert!(edge_rows.iter().any(|(src, tgt, kind)| {
    src == "com.example.patient::Patient"
      && tgt == "com.example.patient::Person"
      && kind == "Extends"
  }));
  assert!(edge_rows.iter().any(|(src, tgt, kind)| {
    src == "com.example.patient::Patient"
      && tgt == "com.example.patient::Validatable"
      && kind == "Implements"
  }));
  assert!(edge_rows.iter().any(|(src, tgt, kind)| {
    src == "com.example.patient::Patient.Detail"
      && tgt == "com.example.patient::InfoRecord"
      && kind == "Extends"
  }));
  assert!(edge_rows.iter().any(|(src, tgt, kind)| {
    src == "com.example.patient::Patient.Companion"
      && tgt == "com.example.patient::Factory"
      && kind == "Extends"
  }));
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
