use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn test_swift_workspace_indexing_and_commands() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Write Package.swift
  let package_content = r#"
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "MySwiftPackage",
    dependencies: [
        .package(url: "https://github.com/apple/swift-algorithms.git", from: "1.2.0"),
        .package(path: "../LocalPackage")
    ]
)
  "#;
  write_file(temp_dir.path(), "Package.swift", package_content);

  // 2. Write Swift source file containing tests and concurrency
  let swift_source = r#"
import Foundation
import XCTest

// Production symbol under test
func validatePatient(age: Int) -> Bool {
  return age >= 0
}

class PatientTests: XCTestCase {
  func testValidation() {
    validatePatient(age: 42)
  }
}

@Test func testModernSwift() {
  Task {
    print("Spawn unstructured task")
  }
  let (stream, continuation) = AsyncStream.makeStream(of: Int.self)
  for await item in stream {
    print(item)
  }
}
  "#;
  write_file(
    temp_dir.path(),
    "Sources/MySwiftPackage/main.swift",
    swift_source,
  );

  // 3. Initialize database
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 4. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let index_output = index_cmd.assert().success().get_output().stdout.clone();
  let index_stdout = String::from_utf8(index_output).expect("stdout should be valid UTF8");
  assert!(index_stdout.contains("files scanned:"));

  // 5. Verify rkg workspace
  let mut ws_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  ws_cmd.current_dir(temp_dir.path()).arg("workspace");
  let ws_output = ws_cmd.assert().success().get_output().stdout.clone();
  let ws_stdout = String::from_utf8(ws_output).expect("stdout should be valid UTF8");

  assert!(ws_stdout.contains("SWIFT PROJECTS"));
  assert!(ws_stdout.contains("MySwiftPackage"));
  assert!(ws_stdout.contains("Package.swift"));
  assert!(ws_stdout.contains("5.9"));
  assert!(ws_stdout.contains("yes"));

  // 6. Verify rkg deps MySwiftPackage
  let mut deps_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  deps_cmd
    .current_dir(temp_dir.path())
    .args(["deps", "MySwiftPackage"]);
  let deps_output = deps_cmd.assert().success().get_output().stdout.clone();
  let deps_stdout = String::from_utf8(deps_output).expect("stdout should be valid UTF8");

  assert!(deps_stdout.contains("Project: MySwiftPackage"));
  assert!(deps_stdout.contains("swift-algorithms"));
  assert!(deps_stdout.contains("SPM Package"));
  assert!(deps_stdout.contains("1.2.0"));
  assert!(deps_stdout.contains("LocalPackage"));
  assert!(deps_stdout.contains("Project Reference"));

  // 7. Verify rkg tests — find tests calling validatePatient (production symbol)
  let mut tests_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  tests_cmd
    .current_dir(temp_dir.path())
    .args(["tests", "validatePatient"]);
  let tests_output = tests_cmd.assert().success().get_output().stdout.clone();
  let tests_stdout = String::from_utf8(tests_output).expect("stdout should be valid UTF8");
  assert!(
    tests_stdout.contains("PatientTests") || tests_stdout.contains("testValidation"),
    "expected PatientTests or testValidation in tests output: {tests_stdout}"
  );

  // Also verify test-level symbols are indexed via rkg find
  let mut find_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  find_cmd
    .current_dir(temp_dir.path())
    .args(["find", "testValidation"]);
  let find_output = find_cmd.assert().success().get_output().stdout.clone();
  let find_stdout = String::from_utf8(find_output).expect("stdout should be valid UTF8");
  assert!(
    find_stdout.contains("testValidation"),
    "expected testValidation to be indexed as a symbol: {find_stdout}"
  );

  // 8. Verify rkg concurrency — check header is present and at least one concurrency primitive
  let mut conc_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  conc_cmd
    .current_dir(temp_dir.path())
    .args(["concurrency", "testModernSwift"]);
  let conc_output = conc_cmd.assert().success().get_output().stdout.clone();
  let conc_stdout = String::from_utf8(conc_output).expect("stdout should be valid UTF8");
  assert!(
    conc_stdout.contains("Concurrency"),
    "expected Concurrency section header: {conc_stdout}"
  );
  // Verify at least one concrete concurrency primitive was recorded
  assert!(
    conc_stdout.contains("Task")
      || conc_stdout.contains("AsyncStream")
      || conc_stdout.contains("for await"),
    "expected Task, AsyncStream or for-await spawn in concurrency output: {conc_stdout}"
  );
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
