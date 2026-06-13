use assert_cmd::Command;

#[test]
fn runs_benchmark_subcommand_successfully() {
  let mut cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  cmd.args(["bench"]);
  let assert = cmd.assert().success();
  let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("valid utf8");

  assert!(stdout.contains("rkg Benchmark Summary"));
  assert!(stdout.contains("py-01"));
  assert!(stdout.contains("rust-01"));
  assert!(stdout.contains("fs-01"));
  assert!(stdout.contains("mojo-01"));
  assert!(stdout.contains("kt-01"));
  assert!(stdout.contains("swift-01"));
}

#[test]
fn runs_benchmark_subcommand_json_successfully() {
  let mut cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  cmd.args(["bench", "--json"]);
  let assert = cmd.assert().success();
  let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("valid utf8");

  assert!(stdout.contains("\"total_tasks\""));
  assert!(stdout.contains("\"successful_tasks\""));
  assert!(stdout.contains("\"py-01\""));
  assert!(stdout.contains("\"rust-01\""));
}
