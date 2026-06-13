use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_pipeline_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create python source file with a monadic pipeline
  let src_code = r#"def step1(x):
  return x + 1

def step2(x):
  return x * 2

def run_pipeline(val):
  # Chaining step1 and step2
  res = val.bind(step1).then(step2)
  return res
"#;
  write_file(temp_dir.path(), "src/main.py", src_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  index_cmd.assert().success();

  // 4. Test rkg pipeline step1
  let mut pipeline_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  pipeline_cmd
    .current_dir(temp_dir.path())
    .args(["pipeline", "step1"]);

  let output = pipeline_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");

  // Validate output contents
  assert!(stdout.contains("Pipeline / Functional Flow Analysis for: step1"));
  assert!(stdout.contains("Pipeline: src.main::run_pipeline (in src/main.py)"));
  assert!(stdout.contains("Step 1: src.main::step1 [TARGET]"));
  assert!(
    stdout.contains("Step 2: src.main::step2 [FAIL-FAST BLAST RADIUS (BYPASSED ON FAILURE)]")
  );
}

#[test]
fn extracts_and_queries_pipeline_with_currying_and_placeholders() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create python source file with a curried pipeline and partials
  let src_code = r#"from toolz import curry

@curry
def add(x, y, z):
  return x + y + z

def step1(x):
  return x + 1

def run_pipeline(val):
  # f1 is partially applied
  f1 = add(5) # Expect placeholders: y, z
  res = val.bind(step1).then(add(1, 2))
  return res
"#;
  write_file(temp_dir.path(), "src/main.py", src_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  index_cmd.assert().success();

  // 4. Test rkg pipeline add
  let mut pipeline_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  pipeline_cmd
    .current_dir(temp_dir.path())
    .args(["pipeline", "add"]);

  let output = pipeline_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");

  println!("PIPELINE CLI OUTPUT:\n{}", stdout);

  // Validate output contents:
  // Step 1: src.main::add [unresolved placeholders: y, z] is f1 = add(5)
  // Step 3: src.main::add is add(1, 2) which gets fully applied since it gets 1 extra arg from then
  assert!(stdout.contains("Pipeline / Functional Flow Analysis for: add"));
  assert!(stdout.contains("Step 1: src.main::add [unresolved placeholders: y, z]"));
  assert!(stdout.contains("Step 3: src.main::add"));
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
