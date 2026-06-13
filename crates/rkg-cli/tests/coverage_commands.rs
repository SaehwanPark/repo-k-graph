use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn imports_and_queries_coverage_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create Python source file with function
  let src_code = r#"
def add(a, b):
    return a + b

def subtract(a, b):
    return a - b
"#;
  write_file(temp_dir.path(), "src/math.py", src_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  index_cmd.assert().success();

  // 4. Create Cobertura XML report
  let xml_report = r#"
<?xml version="1.0" ?>
<coverage line-rate="0.5" branch-rate="0.5" version="1.9">
  <packages>
    <package name="src">
      <classes>
        <class name="math" filename="src/math.py" line-rate="0.5" branch-rate="0.5">
          <methods/>
          <lines>
            <line number="2" hits="1" branch="false"/>
            <line number="3" hits="0" branch="false"/>
            <line number="5" hits="0" branch="false"/>
            <line number="6" hits="0" branch="false"/>
          </lines>
        </class>
      </classes>
    </package>
  </packages>
</coverage>
"#;
  write_file(temp_dir.path(), "coverage.xml", xml_report);

  // 5. Create LCOV report
  let lcov_report = r#"
TN:integration
SF:src/math.py
DA:2,1
DA:3,1
DA:5,1
DA:6,0
end_of_record
"#;
  write_file(temp_dir.path(), "coverage.info", lcov_report);

  // 6. Import XML report
  let mut import_xml = Command::cargo_bin("rkg").expect("rkg binary should compile");
  import_xml.current_dir(temp_dir.path()).args([
    "import-coverage",
    "coverage.xml",
    "--test-suite",
    "unit",
  ]);
  import_xml.assert().success();

  // 7. Verify coverage of `add` function (lines 2-3)
  // `add` starts at line 2 and ends at line 3.
  // XML has line 2 (hits=1) and line 3 (hits=0). So 1/2 covered.
  let mut cov_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  cov_cmd
    .current_dir(temp_dir.path())
    .args(["coverage", "src.math::add"]);
  let output = cov_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("Coverage Profile"));
  assert!(stdout.contains("Target:        src.math::add"));
  assert!(stdout.contains("Statement Coverage: 50.0% (1/2 lines)"));
  assert!(stdout.contains("[Uncovered Line Spans]"));
  assert!(stdout.contains("3"));
  assert!(stdout.contains("- [unit] via `coverage.xml`"));

  // 8. Import LCOV report (which covers lines 2, 3, 5 with hits=1, line 6 with hits=0)
  // `add` (lines 2-3) now has 2/2 covered.
  let mut import_lcov = Command::cargo_bin("rkg").expect("rkg binary should compile");
  import_lcov
    .current_dir(temp_dir.path())
    .args(["import-coverage", "coverage.info"]);
  import_lcov.assert().success();

  // 9. Verify combined coverage of `add` (lines 2-3)
  // From unit report: uncovered 3.
  // From integration report: uncovered none (hits=1 for 2 and 3).
  // Combined uncovered: intersection of {3} and {} = {}. So 100% covered!
  let mut cov_cmd2 = Command::cargo_bin("rkg").expect("rkg binary should compile");
  cov_cmd2
    .current_dir(temp_dir.path())
    .args(["coverage", "src.math::add"]);
  let output2 = cov_cmd2.assert().success().get_output().stdout.clone();
  let stdout2 = String::from_utf8(output2).expect("stdout should be valid utf8");
  assert!(stdout2.contains("Statement Coverage: 100.0% (2/2 lines)"));
  assert!(stdout2.contains("(No uncovered lines - 100% coverage!)"));
  assert!(stdout2.contains("- [integration] via `coverage.info`"));

  // 10. Verify file-level coverage query
  let mut cov_file_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  cov_file_cmd
    .current_dir(temp_dir.path())
    .args(["coverage", "src/math.py"]);
  let file_output = cov_file_cmd.assert().success().get_output().stdout.clone();
  let file_stdout = String::from_utf8(file_output).expect("stdout should be valid utf8");
  assert!(file_stdout.contains("Target:        src/math.py"));
  assert!(file_stdout.contains("Type:          File"));

  // 11. Test missing report error case
  let mut import_missing = Command::cargo_bin("rkg").expect("rkg binary should compile");
  import_missing
    .current_dir(temp_dir.path())
    .args(["import-coverage", "non_existent.xml"]);
  import_missing.assert().failure();
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
