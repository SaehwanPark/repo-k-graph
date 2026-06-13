use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_kotlin_symbols_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create Kotlin source file with a package header
  let kt_content = r#"
    package com.example.patient

    class Patient(val id: String) {
      fun validate(): Boolean {
        return id.isNotEmpty()
      }

      companion object {
        fun createDefault(): Patient {
          return Patient("default")
        }
      }
    }

    interface Validatable {
      fun isValid(): Boolean
    }

    object PatientRegistry {
      val count = 0
    }

    fun String.isAlpha(): Boolean {
      return this.all { it.isLetter() }
    }
  "#;

  write_file(temp_dir.path(), "src/Patient.kt", kt_content);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("files scanned: 1"));

  // 4. Test rkg symbols
  let mut symbols_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  symbols_cmd.current_dir(temp_dir.path()).arg("symbols");
  let sym_output = symbols_cmd.assert().success().get_output().stdout.clone();
  let sym_stdout = String::from_utf8(sym_output).expect("stdout should be valid utf8");

  // Verify Module, Classes, Interfaces, Functions, and Properties are indexed with com.example.patient package path
  assert!(sym_stdout.contains("com.example.patient [Module]"));
  assert!(sym_stdout.contains("com.example.patient::Patient [Class]"));
  assert!(sym_stdout.contains("com.example.patient::Patient.validate [Method]"));
  assert!(sym_stdout.contains("com.example.patient::Patient.Companion [Class]"));
  assert!(sym_stdout.contains("com.example.patient::Patient.Companion.createDefault [Method]"));
  assert!(sym_stdout.contains("com.example.patient::Validatable [Interface]"));
  assert!(sym_stdout.contains("com.example.patient::PatientRegistry [Class]"));
  assert!(sym_stdout.contains("com.example.patient::PatientRegistry.count [Unknown]"));
  assert!(sym_stdout.contains("com.example.patient::String.isAlpha [Function]"));

  // 5. Test rkg find Patient
  let mut find_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  find_cmd
    .current_dir(temp_dir.path())
    .args(["find", "Patient"]);
  let find_output = find_cmd.assert().success().get_output().stdout.clone();
  let find_stdout = String::from_utf8(find_output).expect("stdout should be valid utf8");
  assert!(find_stdout.contains("com.example.patient::Patient [Class]"));

  // 6. Test rkg show com.example.patient::Patient.validate
  let mut show_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  show_cmd
    .current_dir(temp_dir.path())
    .args(["show", "com.example.patient::Patient.validate"]);
  let show_output = show_cmd.assert().success().get_output().stdout.clone();
  let show_stdout = String::from_utf8(show_output).expect("stdout should be valid utf8");
  assert!(show_stdout.contains("Symbol: com.example.patient::Patient.validate [Method]"));
  assert!(show_stdout.contains("File: src/Patient.kt"));
}

#[test]
fn test_kotlin_multiple_files_same_package() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  let file_1_content = r#"
    package com.example.shared
    class Alice {
      fun hello() {}
    }
  "#;
  let file_2_content = r#"
    package com.example.shared
    class Bob {
      fun greet() {}
    }
  "#;

  write_file(temp_dir.path(), "src/Alice.kt", file_1_content);
  write_file(temp_dir.path(), "src/Bob.kt", file_2_content);

  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("files scanned: 2"));

  let mut symbols_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  symbols_cmd.current_dir(temp_dir.path()).arg("symbols");
  let sym_output = symbols_cmd.assert().success().get_output().stdout.clone();
  let sym_stdout = String::from_utf8(sym_output).expect("stdout should be valid utf8");

  // Verify Bob and Alice exist under the same package namespace correctly
  assert!(sym_stdout.contains("com.example.shared::Alice [Class]"));
  assert!(sym_stdout.contains("com.example.shared::Alice.hello [Method]"));
  assert!(sym_stdout.contains("com.example.shared::Bob [Class]"));
  assert!(sym_stdout.contains("com.example.shared::Bob.greet [Method]"));
}

#[test]
fn test_kotlin_no_package_fallback() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  let kt_content = r#"
    class Utility {
      fun run() {}
    }
  "#;

  write_file(temp_dir.path(), "src/utils/Helper.kt", kt_content);

  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  index_cmd.assert().success();

  let mut symbols_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  symbols_cmd.current_dir(temp_dir.path()).arg("symbols");
  let sym_output = symbols_cmd.assert().success().get_output().stdout.clone();
  let sym_stdout = String::from_utf8(sym_output).expect("stdout should be valid utf8");

  // Verify fallback module name is src.utils.Helper (stripped relative dots)
  assert!(sym_stdout.contains("src.utils.Helper [Module]"));
  assert!(sym_stdout.contains("src.utils.Helper::Utility [Class]"));
  assert!(sym_stdout.contains("src.utils.Helper::Utility.run [Method]"));
}

#[test]
fn test_kotlin_package_comment() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  let kt_content = r#"
    package com.example.commented // This is a comment at package declaration
    class Dummy
  "#;

  write_file(temp_dir.path(), "src/Dummy.kt", kt_content);

  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  index_cmd.assert().success();

  let mut symbols_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  symbols_cmd.current_dir(temp_dir.path()).arg("symbols");
  let sym_output = symbols_cmd.assert().success().get_output().stdout.clone();
  let sym_stdout = String::from_utf8(sym_output).expect("stdout should be valid utf8");

  // Verify comment did not corrupt package name extraction
  assert!(sym_stdout.contains("com.example.commented [Module]"));
  assert!(sym_stdout.contains("com.example.commented::Dummy [Class]"));
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
