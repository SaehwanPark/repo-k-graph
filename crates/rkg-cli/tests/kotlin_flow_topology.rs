use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_kotlin_flow_topology_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  let source = r#"
    package com.example.flow

    import kotlinx.coroutines.CoroutineScope
    import kotlinx.coroutines.flow.Flow
    import kotlinx.coroutines.flow.SharingStarted
    import kotlinx.coroutines.flow.catch
    import kotlinx.coroutines.flow.collect
    import kotlinx.coroutines.flow.flow
    import kotlinx.coroutines.flow.map
    import kotlinx.coroutines.flow.shareIn

    fun updatesFlow(): Flow<Int> = flow {
      emit(fetchUpdate())
    }

    fun sharedUpdates(scope: CoroutineScope): Flow<Int> {
      val mapped = updatesFlow()
        .map { normalize(it) }
        .catch { logFailure() }
      return mapped.shareIn(scope, SharingStarted.Eagerly, replay = 1)
    }

    suspend fun persistUpdates(scope: CoroutineScope) {
      sharedUpdates(scope).collect { persist(it) }
    }

    fun fetchUpdate(): Int = 1
    fun normalize(value: Int): Int = value
    fun logFailure() {}
    fun persist(value: Int) {}
  "#;
  write_file(
    temp_dir.path(),
    "src/main/kotlin/com/example/flow/App.kt",
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
    .args(["concurrency", "sharedUpdates"]);
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
  assert!(concurrency_stdout.contains("com.example.flow::sharedUpdates"));
  assert!(concurrency_stdout.contains("[Channels]"));
  assert!(concurrency_stdout.contains("Created `Flow` channel with tx `mapped` and rx `mapped`"));

  let mut topology_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  topology_cmd.current_dir(temp_dir.path()).arg("topology");
  let topology_stdout =
    String::from_utf8(topology_cmd.assert().success().get_output().stdout.clone())
      .expect("stdout should be valid utf8");
  assert!(topology_stdout.contains("WORKSPACE CONCURRENCY TOPOLOGY"));
  assert!(topology_stdout.contains("com.example.flow::updatesFlow"));
  assert!(topology_stdout.contains("com.example.flow::sharedUpdates"));
  assert!(topology_stdout.contains("com.example.flow::persistUpdates"));
  assert!(topology_stdout.contains("──[SendsTo]──>"));

  let mut impact_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  impact_cmd
    .current_dir(temp_dir.path())
    .args(["impact", "sharedUpdates", "-d", "1"]);
  let impact_stdout = String::from_utf8(impact_cmd.assert().success().get_output().stdout.clone())
    .expect("stdout should be valid utf8");
  assert!(impact_stdout.contains("SendsTo"));
  assert!(impact_stdout.contains("com.example.flow::persistUpdates"));

  let mut callees_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  callees_cmd
    .current_dir(temp_dir.path())
    .args(["callees", "sharedUpdates"]);
  let callees_stdout =
    String::from_utf8(callees_cmd.assert().success().get_output().stdout.clone())
      .expect("stdout should be valid utf8");
  assert!(callees_stdout.contains("com.example.flow::normalize"));
  assert!(callees_stdout.contains("com.example.flow::logFailure"));

  let mut callers_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  callers_cmd
    .current_dir(temp_dir.path())
    .args(["callers", "com.example.flow::persist"]);
  let callers_stdout =
    String::from_utf8(callers_cmd.assert().success().get_output().stdout.clone())
      .expect("stdout should be valid utf8");
  assert!(callers_stdout.contains("com.example.flow::persistUpdates"));
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
