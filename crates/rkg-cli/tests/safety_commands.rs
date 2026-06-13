use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_safety_and_ffi_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create Rust source file with unsafe code and FFI declarations
  let src_code = r#"
    #[cxx::bridge]
    mod ffi_bridge {
      unsafe extern "C++" {
        fn cxx_func();
      }
    }

    pub unsafe fn do_raw_stuff() {
      let ptr = std::ptr::null::<i32>();
    }

    pub fn safe_wrapper() {
      // Safe wrapper around an unsafe block
      unsafe {
        let val = *std::ptr::null::<i32>();
      }
    }

    extern "C" {
      fn c_api_func(x: i32) -> i32;
    }
  "#;
  write_file(temp_dir.path(), "src/lib.rs", src_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  index_cmd.assert().success();

  // 4. Test rkg safety src/lib.rs
  let mut safety_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  safety_cmd
    .current_dir(temp_dir.path())
    .args(["safety", "src/lib.rs"]);

  let output = safety_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");

  // Validate output contents
  assert!(stdout.contains("Memory Safety & FFI Risk Profile"));
  assert!(stdout.contains("Target:        src/lib.rs"));
  assert!(stdout.contains("Safety Score:  43/100"));
  assert!(stdout.contains("Risk Level:    High"));
  assert!(stdout.contains("Safe Wrappers: 100.0%"));
  assert!(stdout.contains("[Unsafe Functions & Trait Boundaries]"));
  assert!(stdout.contains("src::lib::do_raw_stuff at src/lib.rs:9"));
  assert!(stdout.contains("[Unsafe Code Blocks]"));
  assert!(stdout.contains("[SAFELY WRAPPED] block in src::lib::safe_wrapper at src/lib.rs:15"));
  assert!(stdout.contains("[FFI / Foreign Interface Bindings]"));
  assert!(stdout.contains("[cxx] Foreign item `ffi_bridge` in src::lib at src/lib.rs:3"));
  assert!(stdout.contains("[C] Foreign item `c_api_func` in src::lib at src/lib.rs:21"));
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
