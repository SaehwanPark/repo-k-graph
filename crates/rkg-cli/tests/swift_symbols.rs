use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_swift_symbols_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create Swift source file
  let swift_content = r#"
    import Foundation

    struct Patient {
      var name: String
      func displayInfo() {
        print("Patient: \(name)")
      }
    }

    protocol Validatable {
      func isValid() -> Bool
    }

    class Document: Validatable {
      var title: String
      init(title: String) {
        self.title = title
      }
      func isValid() -> Bool {
        return !title.isEmpty
      }
    }

    extension Document {
      func printTitle() {
        print(title)
      }
    }
  "#;

  write_file(temp_dir.path(), "src/Patient.swift", swift_content);

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

  // Verify Module, Structs, Classes, Protocols, Extensions, Methods, and Properties are indexed
  assert!(sym_stdout.contains("src.Patient [Module]"));
  assert!(sym_stdout.contains("src.Patient::Patient [Struct]"));
  assert!(sym_stdout.contains("src.Patient::Patient.name [Unknown]"));
  assert!(sym_stdout.contains("src.Patient::Patient.displayInfo [Method]"));
  assert!(sym_stdout.contains("src.Patient::Validatable [Interface]"));
  assert!(sym_stdout.contains("src.Patient::Document [Class]"));
  assert!(sym_stdout.contains("src.Patient::Document.title [Unknown]"));
  assert!(sym_stdout.contains("src.Patient::Document.init [Method]"));
  assert!(sym_stdout.contains("src.Patient::Document.isValid [Method]"));
  assert!(sym_stdout.contains("src.Patient::Document.printTitle [Method]"));

  // 5. Test rkg find Patient
  let mut find_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  find_cmd
    .current_dir(temp_dir.path())
    .args(["find", "Patient"]);
  let find_output = find_cmd.assert().success().get_output().stdout.clone();
  let find_stdout = String::from_utf8(find_output).expect("stdout should be valid utf8");
  assert!(find_stdout.contains("src.Patient::Patient [Struct]"));

  // 6. Test rkg show src.Patient::Patient.displayInfo
  let mut show_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  show_cmd
    .current_dir(temp_dir.path())
    .args(["show", "src.Patient::Patient.displayInfo"]);
  let show_output = show_cmd.assert().success().get_output().stdout.clone();
  let show_stdout = String::from_utf8(show_output).expect("stdout should be valid utf8");
  assert!(show_stdout.contains("Symbol: src.Patient::Patient.displayInfo [Method]"));
  assert!(show_stdout.contains("File: src/Patient.swift"));
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
