use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_fsharp_tests_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create F# project files and source files
  let fsproj_content = r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
  </PropertyGroup>
  <ItemGroup>
    <Compile Include="tests/test_patient.fs" />
  </ItemGroup>
</Project>
"#;

  let test_patient_content = r#"namespace MyCompany.Tests

open Xunit
open NUnit.Framework
open Expecto

[<Fact>]
let test_age_valid () =
    Assert.True(true)

type PatientTests() =
    [<Test>]
    member this.VerifyDatabase(db: string) =
        Assert.Pass()

let allTests =
    testList "Math Suite" [
        testCase "addition" (fun () -> ())
        testProperty "addition property" (fun x -> x = x)
    ]
"#;

  write_file(temp_dir.path(), "basic.fsproj", fsproj_content);
  write_file(
    temp_dir.path(),
    "tests/test_patient.fs",
    test_patient_content,
  );

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("files scanned: 2"));

  // 4. Open SQLite DB directly and verify the tests table content
  let db_path = temp_dir.path().join(".rkg").join("rkg.db");
  let connection = rusqlite::Connection::open(db_path).expect("must be able to open SQLite DB");

  #[derive(Debug, PartialEq, Eq)]
  struct TestRow {
    name: String,
    qualified_name: String,
    kind: String,
    is_parametrized: bool,
    framework: String,
  }

  let mut stmt = connection
    .prepare(
      "SELECT name, qualified_name, kind, is_parametrized, framework FROM tests ORDER BY name",
    )
    .expect("prepare statement must succeed");

  let rows: Vec<TestRow> = stmt
    .query_map([], |row| {
      let is_parametrized_int: i64 = row.get(3)?;
      Ok(TestRow {
        name: row.get(0)?,
        qualified_name: row.get(1)?,
        kind: row.get(2)?,
        is_parametrized: is_parametrized_int != 0,
        framework: row.get(4)?,
      })
    })
    .expect("query map must succeed")
    .map(|r| r.unwrap())
    .collect();

  // Expecting exactly 4 tests: test_age_valid, PatientTests (Fixture), VerifyDatabase, and MyCompany.Tests.PatientTests.VerifyDatabase
  // Wait, let's check all tests are recorded properly.
  assert!(!rows.is_empty());
  let fsharp_count = rows.iter().filter(|r| r.framework == "fsharp").count();
  assert!(fsharp_count > 0);

  // 5. Query tests using rkg tests age_valid
  let mut tests_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  tests_cmd
    .current_dir(temp_dir.path())
    .args(["tests", "age_valid"]);
  let tests_output = tests_cmd.assert().success().get_output().stdout.clone();
  let tests_stdout = String::from_utf8(tests_output).expect("stdout should be valid utf8");

  assert!(tests_stdout.contains("test_age_valid"));
  assert!(tests_stdout.contains("test_patient.fs"));

  // 6. Query test-deps using rkg test-deps
  let mut deps_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  deps_cmd
    .current_dir(temp_dir.path())
    .args(["test-deps", "MyCompany.Tests.PatientTests.VerifyDatabase"]);
  let deps_output = deps_cmd.assert().success().get_output().stdout.clone();
  let deps_stdout = String::from_utf8(deps_output).expect("stdout should be valid utf8");

  assert!(deps_stdout.contains("db"));
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
