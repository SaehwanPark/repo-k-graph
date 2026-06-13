use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_rust_simulated_derives_and_expand_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create Cargo.toml and src/main.rs with derives
  let cargo_toml = r#"
[package]
name = "test-macro"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
clap = { version = "4.0", features = ["derive"] }
thiserror = "1.0"
"#;

  let main_code = r#"
use serde::{Serialize, Deserialize};
use thiserror::Error;
use clap::Parser;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
  pub name: String,
}

#[derive(Debug, Error)]
pub enum ConfigError {
  #[error("invalid config")]
  Invalid,
}

#[derive(Parser)]
pub struct CliArgs {
  #[arg(long)]
  pub debug: bool,
}

fn main() {}
"#;

  write_file(temp_dir.path(), "Cargo.toml", cargo_toml);
  write_file(temp_dir.path(), "src/main.rs", main_code);

  // 3. Index repository using standard static derive simulation
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  index_cmd.assert().success();

  // 4. Verify Implements edges in SQLite DB directly
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
  let mut has_serialize = false;
  let mut has_deserialize = false;
  let mut has_parser = false;
  let mut has_error = false;

  while let Some(row) = rows.next().expect("failed to fetch row") {
    let source_qname: String = row.get(1).unwrap();
    let target_qname: Option<String> = row.get(2).unwrap();
    let unresolved: Option<String> = row.get(3).unwrap();

    let target_str = target_qname.or(unresolved).unwrap_or_default();
    println!(
      "IMPLEMENTS EDGE: source={}, target={}",
      source_qname, target_str
    );

    if source_qname == "src::main::User" && target_str.contains("Serialize") {
      has_serialize = true;
    }
    if source_qname == "src::main::User" && target_str.contains("Deserialize") {
      has_deserialize = true;
    }
    if source_qname == "src::main::CliArgs" && target_str.contains("Parser") {
      has_parser = true;
    }
    if source_qname == "src::main::ConfigError" && target_str.contains("Error") {
      has_error = true;
    }
  }

  assert!(has_serialize, "User should implement Serialize!");
  assert!(has_deserialize, "User should implement Deserialize!");
  assert!(has_parser, "CliArgs should implement Parser!");
  assert!(has_error, "ConfigError should implement Error!");

  // 5. Index repository using the --expand CLI option to verify it completes gracefully
  let mut index_expand_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_expand_cmd
    .current_dir(temp_dir.path())
    .args(["index", "--force", "--expand"]);

  // Should succeed gracefully even if cargo expand is not installed on the system (as it outputs a warning and proceeds)
  index_expand_cmd.assert().success();
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
