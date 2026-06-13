use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_concurrency_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create Rust source file with async tokio concurrency and channels
  let src_code = r#"
    use tokio::sync::mpsc;

    pub async fn worker_task() {}

    pub async fn run_concurrency() {
      let (mut tx, rx) = mpsc::channel(32);

      tokio::spawn(async move {
        worker_task().await;
      });

      tokio::select! {
        _ = rx.recv() => {}
      }
    }
  "#;
  write_file(temp_dir.path(), "src/lib.rs", src_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  index_cmd.assert().success();

  // 4. Test rkg concurrency run_concurrency
  let mut concurrency_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  concurrency_cmd
    .current_dir(temp_dir.path())
    .args(["concurrency", "run_concurrency"]);

  let output = concurrency_cmd
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");

  // Validate output contents
  assert!(stdout.contains("Concurrency Topology for symbol: src::lib::run_concurrency"));
  assert!(stdout.contains("[Spawns]"));
  assert!(stdout.contains("Spawns task executing `worker_task` via `tokio::spawn`"));
  assert!(stdout.contains("[Channels]"));
  assert!(stdout.contains("Created `mpsc` channel with tx `tx` and rx `rx`"));
  assert!(stdout.contains("[Selects]"));
  assert!(stdout.contains("Concurrency select block at src/lib.rs:13"));
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
