use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn test_kotlin_workspace_indexing_and_commands() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Write settings.gradle.kts
  let settings_content = r#"
pluginManagement {
    repositories {
        mavenCentral()
    }
}
rootProject.name = "MyKotlinProject"
include(":app")
include(":shared-lib")
  "#;
  write_file(temp_dir.path(), "settings.gradle.kts", settings_content);

  // 2. Write app/build.gradle.kts with dependencies
  let app_build_content = r#"
plugins {
    kotlin("jvm") version "1.9.0"
}

dependencies {
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.7.3")
    implementation(project(":shared-lib"))
    api("com.squareup.okhttp3:okhttp:4.11.0")
}
  "#;
  write_file(temp_dir.path(), "app/build.gradle.kts", app_build_content);

  // 3. Write shared-lib/build.gradle.kts
  let lib_build_content = r#"
plugins {
    kotlin("jvm") version "1.9.0"
}
  "#;
  write_file(
    temp_dir.path(),
    "shared-lib/build.gradle.kts",
    lib_build_content,
  );

  // 4. Write app Kotlin file with a Ktor route
  let app_source_content = r#"
package com.example.app

import io.ktor.server.application.*
import io.ktor.server.routing.*
import io.ktor.server.response.*

fun Application.module() {
    routing {
        get("/users") {
            call.respondText("hello")
        }
        route("/api") {
            post("/items") {
                call.respondText("items")
            }
        }
    }
}
  "#;
  write_file(
    temp_dir.path(),
    "app/src/main/kotlin/com/example/app/App.kt",
    app_source_content,
  );

  // 5. Initialize database
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 6. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let index_output = index_cmd.assert().success().get_output().stdout.clone();
  let index_stdout = String::from_utf8(index_output).expect("stdout should be valid UTF8");
  assert!(index_stdout.contains("files scanned:"));

  // 7. Verify rkg workspace
  let mut ws_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  ws_cmd.current_dir(temp_dir.path()).arg("workspace");
  let ws_output = ws_cmd.assert().success().get_output().stdout.clone();
  let ws_stdout = String::from_utf8(ws_output).expect("stdout should be valid UTF8");

  assert!(ws_stdout.contains("KOTLIN PROJECTS"));
  assert!(ws_stdout.contains("app"));
  assert!(ws_stdout.contains("shared-lib"));
  assert!(ws_stdout.contains("app/build.gradle.kts"));
  assert!(ws_stdout.contains("shared-lib/build.gradle.kts"));
  assert!(ws_stdout.contains("jvm"));
  assert!(ws_stdout.contains("yes"));

  // 8. Verify rkg deps app
  let mut deps_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  deps_cmd.current_dir(temp_dir.path()).args(["deps", "app"]);
  let deps_output = deps_cmd.assert().success().get_output().stdout.clone();
  let deps_stdout = String::from_utf8(deps_output).expect("stdout should be valid UTF8");

  assert!(deps_stdout.contains("Project: app"));
  assert!(deps_stdout.contains("app/build.gradle.kts"));
  assert!(deps_stdout.contains("shared-lib"));
  assert!(deps_stdout.contains("Project Reference"));
  assert!(deps_stdout.contains("org.jetbrains.kotlinx:kotlinx-coroutines-core"));
  assert!(deps_stdout.contains("Maven/Gradle Dependency"));
  assert!(deps_stdout.contains("1.7.3"));
  assert!(deps_stdout.contains("com.squareup.okhttp3:okhttp"));
  assert!(deps_stdout.contains("4.11.0"));

  // 9. Verify rkg routes
  let mut routes_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  routes_cmd.current_dir(temp_dir.path()).arg("routes");
  let routes_output = routes_cmd.assert().success().get_output().stdout.clone();
  let routes_stdout = String::from_utf8(routes_output).expect("stdout should be valid UTF8");

  assert!(routes_stdout.contains("GET"));
  assert!(routes_stdout.contains("/users"));
  assert!(routes_stdout.contains("POST"));
  assert!(routes_stdout.contains("/api/items"));
  assert!(routes_stdout.contains("com.example.app::module"));
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
