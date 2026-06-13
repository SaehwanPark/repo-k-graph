use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_mojo_symbols_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create Mojo source file
  let main_mojo_content = r#"
fn global_fn(x: Int) -> Bool:
    return True

struct Patient:
    fn __init__(out self):
        pass

    fn get_age(read self) -> Int:
        return 42

trait Validator:
    fn validate(self) -> Bool:
        pass
"#;

  write_file(temp_dir.path(), "src/main.mojo", main_mojo_content);

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
  println!("EXTRACTED MOJO SYMBOLS:\n{}", sym_stdout);

  // Verify Module, Structs, Traits, Functions, and Methods are indexed
  assert!(sym_stdout.contains("src.main [Module]"));
  assert!(sym_stdout.contains("src.main::global_fn [Function]"));
  assert!(sym_stdout.contains("src.main::Patient [Struct]"));
  assert!(sym_stdout.contains("src.main::Patient.__init__ [Method]"));
  assert!(sym_stdout.contains("src.main::Patient.get_age [Method]"));
  assert!(sym_stdout.contains("src.main::Validator [Interface]"));
  assert!(sym_stdout.contains("src.main::Validator.validate [Method]"));

  // 5. Test rkg find Patient
  let mut find_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  find_cmd
    .current_dir(temp_dir.path())
    .args(["find", "Patient"]);
  let find_output = find_cmd.assert().success().get_output().stdout.clone();
  let find_stdout = String::from_utf8(find_output).expect("stdout should be valid utf8");
  assert!(find_stdout.contains("src.main::Patient [Struct] (src/main.mojo:5-10)"));

  // 6. Test rkg show src.main::Patient.get_age
  let mut show_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  show_cmd
    .current_dir(temp_dir.path())
    .args(["show", "src.main::Patient.get_age"]);
  let show_output = show_cmd.assert().success().get_output().stdout.clone();
  let show_stdout = String::from_utf8(show_output).expect("stdout should be valid utf8");
  assert!(show_stdout.contains("Symbol: src.main::Patient.get_age [Method]"));
  assert!(show_stdout.contains("File: src/main.mojo (lines 9-10)"));
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
