use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn test_cargo_workspace_indexing_and_commands() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Write workspace root Cargo.toml
  let root_toml = r#"
[workspace]
members = ["crates/rkg-core", "crates/rkg-cli"]

[workspace.dependencies]
serde = "1.0.152"
"#;
  write_file(temp_dir.path(), "Cargo.toml", root_toml);

  // 2. Write Cargo.lock
  let lock_content = r#"
[[package]]
name = "serde"
version = "1.0.152"

[[package]]
name = "tokio"
version = "1.35.1"
"#;
  write_file(temp_dir.path(), "Cargo.lock", lock_content);

  // 3. Write crates/rkg-core/Cargo.toml
  let core_toml = r#"
[package]
name = "rkg-core"
version = "0.2.1"
edition = "2021"

[dependencies]
serde = { workspace = true }
"#;
  write_file(temp_dir.path(), "crates/rkg-core/Cargo.toml", core_toml);
  write_file(
    temp_dir.path(),
    "crates/rkg-core/src/lib.rs",
    "pub struct CoreDomain;",
  );

  // 4. Write crates/rkg-cli/Cargo.toml with custom features
  let cli_toml = r#"
[package]
name = "rkg-cli"
version = "0.5.0"
edition = "2021"

[dependencies]
rkg-core = { path = "../rkg-core" }
tokio = "1.0"

[features]
default = ["premium"]
premium = []
"#;
  write_file(temp_dir.path(), "crates/rkg-cli/Cargo.toml", cli_toml);

  // 5. Write crates/rkg-cli/src/main.rs with features and external imports
  let cli_main = r#"
use tokio::sync::mpsc;
use rkg_core::CoreDomain;

#[cfg(feature = "premium")]
pub fn premium_feature_active() {
  println!("Premium activated!");
}

#[cfg(not(feature = "premium"))]
pub fn premium_feature_inactive() {
  println!("Standard version");
}

pub fn main() {
  let _ = mpsc::channel::<i32>(10);
  let _ = CoreDomain;
}
"#;
  write_file(temp_dir.path(), "crates/rkg-cli/src/main.rs", cli_main);

  // 6. Initialize database
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 7. Index the workspace repo
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let index_output = index_cmd.assert().success().get_output().stdout.clone();
  let index_stdout = String::from_utf8(index_output).expect("stdout should be valid UTF8");
  assert!(index_stdout.contains("files scanned: 6"));

  // 8. Test rkg workspace command
  let mut ws_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  ws_cmd.current_dir(temp_dir.path()).arg("workspace");
  let ws_output = ws_cmd.assert().success().get_output().stdout.clone();
  let ws_stdout = String::from_utf8(ws_output).expect("stdout should be valid UTF8");

  assert!(ws_stdout.contains("rkg-core"));
  assert!(ws_stdout.contains("rkg-cli"));
  assert!(ws_stdout.contains("crates/rkg-core/Cargo.toml"));
  assert!(ws_stdout.contains("crates/rkg-cli/Cargo.toml"));
  assert!(ws_stdout.contains("0.2.1"));
  assert!(ws_stdout.contains("0.5.0"));
  assert!(ws_stdout.contains("default, premium"));

  // 9. Test rkg deps rkg-cli command
  let mut deps_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  deps_cmd
    .current_dir(temp_dir.path())
    .args(["deps", "rkg-cli"]);
  let deps_output = deps_cmd.assert().success().get_output().stdout.clone();
  let deps_stdout = String::from_utf8(deps_output).expect("stdout should be valid UTF8");

  assert!(deps_stdout.contains("Package: rkg-cli (0.5.0)"));
  assert!(deps_stdout.contains("rkg-core"));
  assert!(deps_stdout.contains("tokio"));
  assert!(deps_stdout.contains("Internal"));
  assert!(deps_stdout.contains("External"));

  // 10. Test feature flag pruning: premium_feature_active should be indexed, premium_feature_inactive should NOT
  let mut syms_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  syms_cmd.current_dir(temp_dir.path()).arg("symbols");
  let syms_output = syms_cmd.assert().success().get_output().stdout.clone();
  let syms_stdout = String::from_utf8(syms_output).expect("stdout should be valid UTF8");

  assert!(syms_stdout.contains("crates::rkg-cli::src::main::premium_feature_active [Function]"));
  assert!(!syms_stdout.contains("crates::rkg-cli::src::main::premium_feature_inactive [Function]"));

  // 11. Test external import docs mapping: tokio resolved version from Cargo.lock is 1.35.1
  let mut docs_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  docs_cmd
    .current_dir(temp_dir.path())
    .args(["docs", "tokio::sync::mpsc"]);
  let docs_output = docs_cmd.assert().success().get_output().stdout.clone();
  let docs_stdout = String::from_utf8(docs_output).expect("stdout should be valid UTF8");

  assert!(docs_stdout.contains("https://docs.rs/tokio/1.35.1/tokio/sync/mpsc/"));

  // 12. Test clearing removed Cargo dependencies: remove tokio from Cargo.toml, re-index, and assert tokio is gone!
  let cli_toml_no_tokio = r#"
[package]
name = "rkg-cli"
version = "0.5.0"
edition = "2021"

[dependencies]
rkg-core = { path = "../rkg-core" }
"#;
  write_file(
    temp_dir.path(),
    "crates/rkg-cli/Cargo.toml",
    cli_toml_no_tokio,
  );

  let mut reindex_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  reindex_cmd
    .current_dir(temp_dir.path())
    .args(["index", "--force"]);
  reindex_cmd.assert().success();

  let mut deps_cmd_2 = Command::cargo_bin("rkg").expect("rkg binary should compile");
  deps_cmd_2
    .current_dir(temp_dir.path())
    .args(["deps", "rkg-cli"]);
  let deps_output_2 = deps_cmd_2.assert().success().get_output().stdout.clone();
  let deps_stdout_2 = String::from_utf8(deps_output_2).expect("stdout should be valid UTF8");

  assert!(deps_stdout_2.contains("rkg-core"));
  assert!(!deps_stdout_2.contains("tokio"));
}

fn setup_repo(root: &std::path::Path) {
  fs::create_dir_all(root.join(".git")).expect(".git directory should be created");
}

fn write_file(root: &std::path::Path, relative_path: &str, content: &str) {
  let path = root.join(relative_path);
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).expect("parent directories should be created");
  }
  fs::write(path, content).expect("file should be written");
}
