use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_fsharp_relationships_via_cli() {
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
    <Compile Include="src/patient.fs" />
    <Compile Include="script.fsx" />
  </ItemGroup>
</Project>
"#;

  let patient_fs_content = r#"namespace MyCompany.Core

type PatientId = PatientId of string

type Patient = {
    Id: PatientId
    Name: string
    Age: int
}

type IValidator =
    abstract member Validate: Patient -> bool

type BaseValidator() =
    member this.Version = "1.0.0"

type PatientValidator() =
    inherit BaseValidator()
    interface IValidator with
        member this.Validate(patient: Patient) =
            patient.Age >= 0

type MyClass =
    // interface abstract
    class
    end

module PatientValidation =
    let (|ValidAge|InvalidAge|) age =
        if age >= 0 then ValidAge else InvalidAge

    let validatePatient (patient: Patient) =
        match patient.Age with
        | ValidAge -> patient.Name <> ""
        | InvalidAge -> false
"#;

  let script_fsx_content = r#"open MyCompany.Core

let testPatient = {
    Id = PatientId "123"
    Name = "Alice"
    Age = 30
}

let isValid = PatientValidation.validatePatient testPatient
printfn "Patient validity: %b" isValid
"#;

  write_file(temp_dir.path(), "basic.fsproj", fsproj_content);
  write_file(temp_dir.path(), "src/patient.fs", patient_fs_content);
  write_file(temp_dir.path(), "script.fsx", script_fsx_content);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  println!("INDEX STDOUT:\n{}", stdout);
  assert!(stdout.contains("files scanned: 3"));

  // 4. Test rkg imports
  let mut imports_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  imports_cmd
    .current_dir(temp_dir.path())
    .args(["imports", "script.fsx"]);
  let imp_output = imports_cmd.assert().success().get_output().stdout.clone();
  let imp_stdout = String::from_utf8(imp_output).expect("stdout should be valid utf8");
  println!("EXTRACTED IMPORTS:\n{}", imp_stdout);
  assert!(imp_stdout.contains("MyCompany.Core"));

  // 4.1. Test rkg imported-by (E2E Reviewer request)
  let mut imported_by_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  imported_by_cmd
    .current_dir(temp_dir.path())
    .args(["imported-by", "src/patient.fs"]);
  let imp_by_output = imported_by_cmd
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let imp_by_stdout = String::from_utf8(imp_by_output).expect("stdout should be valid utf8");
  println!("IMPORTED BY:\n{}", imp_by_stdout);
  assert!(imp_by_stdout.contains("script.fsx"));

  // 5. Test rkg callers (validatePatient)
  let mut callers_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  callers_cmd.current_dir(temp_dir.path()).args([
    "callers",
    "MyCompany.Core.PatientValidation.validatePatient",
  ]);
  let callers_output = callers_cmd.assert().success().get_output().stdout.clone();
  let callers_stdout = String::from_utf8(callers_output).expect("stdout should be valid utf8");
  println!("CALLERS:\n{}", callers_stdout);
  assert!(callers_stdout.contains("script.isValid [Function]"));

  // 5.1. Test rkg callees (E2E Reviewer request)
  let mut callees_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  callees_cmd
    .current_dir(temp_dir.path())
    .args(["callees", "script.isValid"]);
  let callees_output = callees_cmd.assert().success().get_output().stdout.clone();
  let callees_stdout = String::from_utf8(callees_output).expect("stdout should be valid utf8");
  println!("CALLEES:\n{}", callees_stdout);
  assert!(callees_stdout.contains("MyCompany.Core.PatientValidation.validatePatient"));

  // 5.2. Test comment-stripping E2E (verify comment-stripping request)
  let mut find_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  find_cmd
    .current_dir(temp_dir.path())
    .args(["find", "MyClass"]);
  let find_output = find_cmd.assert().success().get_output().stdout.clone();
  let find_stdout = String::from_utf8(find_output).expect("stdout should be valid utf8");
  println!("FIND MYCLASS:\n{}", find_stdout);
  // If comment stripping fails, MyClass is incorrectly classified as [Interface] instead of [Class]
  assert!(find_stdout.contains("MyCompany.Core.MyClass [Class]"));

  // 6. Test rkg types (Patient)
  let mut types_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  types_cmd
    .current_dir(temp_dir.path())
    .args(["types", "MyCompany.Core.Patient"]);
  let types_output = types_cmd.assert().success().get_output().stdout.clone();
  let types_stdout = String::from_utf8(types_output).expect("stdout should be valid utf8");
  println!("TYPES:\n{}", types_stdout);
  assert!(types_stdout.contains("MyCompany.Core.PatientValidator.Validate"));
  assert!(types_stdout.contains("MyCompany.Core.PatientValidation.validatePatient"));

  // 7. Test rkg pipeline (validatePatient)
  let mut pipeline_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  pipeline_cmd.current_dir(temp_dir.path()).args([
    "pipeline",
    "MyCompany.Core.PatientValidation.validatePatient",
  ]);
  let pipe_output = pipeline_cmd.assert().success().get_output().stdout.clone();
  let pipe_stdout = String::from_utf8(pipe_output).expect("stdout should be valid utf8");
  println!("PIPELINE FLOW:\n{}", pipe_stdout);
  assert!(pipe_stdout.contains("Pipeline: script.isValid"));
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
