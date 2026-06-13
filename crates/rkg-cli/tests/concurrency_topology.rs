use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_concurrency_topology_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create Rust source file containing async spawns and mpsc channels
  let rust_code = r#"
    use tokio::sync::mpsc;

    pub async fn main_task() {
      let (tx, rx) = mpsc::channel(32);
      
      tokio::spawn(async move {
        worker_task(tx).await;
      });

      tokio::spawn(worker_task_direct);

      tokio::select! {
        val = rx.recv() => {
          println!("Got: {:?}", val);
        }
      }
    }

    pub async fn worker_task(tx: mpsc::Sender<i32>) {
      tx.send(42).await.unwrap();
    }

    pub fn worker_task_direct() {}
  "#;
  write_file(temp_dir.path(), "src/lib.rs", rust_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  println!("INDEX OUTPUT: {}", stdout);
  assert!(stdout.contains("rust files parsed: 1"));

  // 4. Test rkg topology CLI command output
  let mut topo_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  topo_cmd.current_dir(temp_dir.path()).arg("topology");
  let topo_output = topo_cmd.assert().success().get_output().stdout.clone();
  let topo_stdout = String::from_utf8(topo_output).expect("stdout should be valid utf8");

  // Assert elegant headers are printed
  assert!(topo_stdout.contains("WORKSPACE CONCURRENCY TOPOLOGY"));
  assert!(topo_stdout.contains("SPAWNING TOPOLOGY (Tasks & Threads)"));
  assert!(topo_stdout.contains("CHANNEL PIPELINES & DATAFLOW PATHWAYS"));

  // Assert Spawns edges are rendered
  assert!(
    topo_stdout.contains("src::lib::main_task")
      && topo_stdout.contains("==[Spawns]==>")
      && topo_stdout.contains("worker_task")
  );
  assert!(
    topo_stdout.contains("src::lib::main_task")
      && topo_stdout.contains("==[Spawns]==>")
      && topo_stdout.contains("worker_task_direct")
  );

  // Assert SendsTo channel pathways are rendered
  assert!(
    topo_stdout.contains("src::lib::worker_task")
      && topo_stdout.contains("──[SendsTo]──>")
      && topo_stdout.contains("src::lib::main_task")
  );

  // 5. Verify that impact analysis propagates across concurrency edges
  let mut impact_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  impact_cmd
    .current_dir(temp_dir.path())
    .args(["impact", "worker_task", "-d", "1"]);
  let impact_output = impact_cmd.assert().success().get_output().stdout.clone();
  let impact_stdout = String::from_utf8(impact_output).expect("stdout should be valid utf8");

  // Impact on worker_task should show main_task via SendsTo channel dataflow
  assert!(impact_stdout.contains("SendsTo"));
  assert!(impact_stdout.contains("src::lib::main_task"));
}

fn setup_repo(root: &std::path::Path) {
  fs::create_dir(root.join(".git")).expect(".git directory should be created");
  fs::create_dir(root.join("src")).expect("src directory should be created");
  fs::write(
    root.join("Cargo.toml"),
    r#"
    [package]
    name = "concurrency-test"
    version = "0.1.0"
    edition = "2021"
  "#,
  )
  .expect("Cargo.toml should be written");
}

fn write_file(root: &std::path::Path, relative_path: &str, content: &str) {
  let path = root.join(relative_path);
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).expect("parent directories should be created");
  }
  fs::write(path, content).expect("file should be written");
}
