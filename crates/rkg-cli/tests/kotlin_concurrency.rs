use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_kotlin_concurrency_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  let source = r#"
    package com.example.concurrent

    import kotlinx.coroutines.CoroutineScope
    import kotlinx.coroutines.async
    import kotlinx.coroutines.channels.Channel
    import kotlinx.coroutines.launch
    import kotlinx.coroutines.selects.select

    fun orchestrate(scope: CoroutineScope) {
      val updates = Channel<Int>()

      scope.launch {
        publishUpdates(updates)
      }

      scope.async {
        readUpdates(updates)
      }

      select<Unit> {
        updates.onReceive { value ->
          println(value)
        }
      }
    }

    suspend fun publishUpdates(updates: Channel<Int>) {
      updates.send(42)
    }

    suspend fun readUpdates(updates: Channel<Int>) {
      updates.receive()
    }
  "#;
  write_file(
    temp_dir.path(),
    "src/main/kotlin/com/example/concurrent/App.kt",
    source,
  );

  let build_gradle = r#"
    plugins {
      kotlin("jvm") version "1.9.0"
    }

    dependencies {
      implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.7.3")
    }
  "#;
  write_file(temp_dir.path(), "build.gradle.kts", build_gradle);

  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let index_stdout = String::from_utf8(index_cmd.assert().success().get_output().stdout.clone())
    .expect("stdout should be valid utf8");
  assert!(index_stdout.contains("files scanned:"));

  let mut concurrency_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  concurrency_cmd
    .current_dir(temp_dir.path())
    .args(["concurrency", "orchestrate"]);
  let concurrency_stdout = String::from_utf8(
    concurrency_cmd
      .assert()
      .success()
      .get_output()
      .stdout
      .clone(),
  )
  .expect("stdout should be valid utf8");

  assert!(concurrency_stdout.contains("Concurrency Topology for symbol:"));
  assert!(concurrency_stdout.contains("publishUpdates"));
  assert!(concurrency_stdout.contains("launch"));
  assert!(concurrency_stdout.contains("readUpdates"));
  assert!(concurrency_stdout.contains("async"));
  assert!(concurrency_stdout.contains("Channel"));
  assert!(concurrency_stdout.contains("select"));

  let mut topology_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  topology_cmd.current_dir(temp_dir.path()).arg("topology");
  let topology_stdout =
    String::from_utf8(topology_cmd.assert().success().get_output().stdout.clone())
      .expect("stdout should be valid utf8");

  assert!(topology_stdout.contains("WORKSPACE CONCURRENCY TOPOLOGY"));
  assert!(topology_stdout.contains("com.example.concurrent::orchestrate"));
  assert!(topology_stdout.contains("==[Spawns]==>"));
  assert!(topology_stdout.contains("publishUpdates"));
  assert!(topology_stdout.contains("──[SendsTo]──>"));
  assert!(topology_stdout.contains("com.example.concurrent::readUpdates"));

  let mut impact_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  impact_cmd
    .current_dir(temp_dir.path())
    .args(["impact", "publishUpdates", "-d", "1"]);
  let impact_stdout = String::from_utf8(impact_cmd.assert().success().get_output().stdout.clone())
    .expect("stdout should be valid utf8");

  assert!(impact_stdout.contains("SendsTo"));
  assert!(impact_stdout.contains("com.example.concurrent::readUpdates"));
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
