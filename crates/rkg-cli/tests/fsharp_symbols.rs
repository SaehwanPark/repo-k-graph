use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_fsharp_symbols_via_cli() {
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

type PatientValidator() =
    member this.Version = "1.0.0"
    interface IValidator with
        member this.Validate(patient: Patient) =
            patient.Age >= 0

module PatientValidation =
    let (|ValidAge|InvalidAge|) age =
        if age >= 0 then ValidAge else InvalidAge

    let validatePatient (patient: Patient) =
        match patient.Age with
        | ValidAge -> patient.Name <> ""
        | InvalidAge -> false
"#;

  let script_fsx_content = r#"// F# script
let testNum = 42
"#;

  write_file(temp_dir.path(), "basic.fsproj", fsproj_content);
  write_file(temp_dir.path(), "src/patient.fs", patient_fs_content);
  write_file(temp_dir.path(), "script.fsx", script_fsx_content);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("files scanned: 3"));

  // 4. Test rkg symbols
  let mut symbols_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  symbols_cmd.current_dir(temp_dir.path()).arg("symbols");
  let sym_output = symbols_cmd.assert().success().get_output().stdout.clone();
  let sym_stdout = String::from_utf8(sym_output).expect("stdout should be valid utf8");
  println!("EXTRACTED SYMBOLS:\n{}", sym_stdout);

  // Verify Namespace, Types, Nested Modules, Functions, and Members are indexed
  assert!(sym_stdout.contains("MyCompany.Core [Module]"));
  assert!(sym_stdout.contains("MyCompany.Core.PatientId [Class]"));
  assert!(sym_stdout.contains("MyCompany.Core.Patient [Class]"));
  assert!(sym_stdout.contains("MyCompany.Core.IValidator [Interface]"));
  assert!(sym_stdout.contains("MyCompany.Core.IValidator.Validate [Method]"));
  assert!(sym_stdout.contains("MyCompany.Core.PatientValidator [Class]"));
  assert!(sym_stdout.contains("MyCompany.Core.PatientValidator.Version [Method]"));
  assert!(sym_stdout.contains("MyCompany.Core.PatientValidator.Validate [Method]"));
  assert!(sym_stdout.contains("MyCompany.Core.PatientValidation [Module]"));
  assert!(sym_stdout.contains("MyCompany.Core.PatientValidation.|ValidAge|InvalidAge| [Function]"));
  assert!(sym_stdout.contains("MyCompany.Core.PatientValidation.validatePatient [Function]"));
  assert!(sym_stdout.contains("script [Module]"));
  assert!(sym_stdout.contains("script.testNum [Function]"));

  // 5. Test rkg find Patient
  let mut find_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  find_cmd
    .current_dir(temp_dir.path())
    .args(["find", "Patient"]);
  let find_output = find_cmd.assert().success().get_output().stdout.clone();
  let find_stdout = String::from_utf8(find_output).expect("stdout should be valid utf8");
  assert!(find_stdout.contains("MyCompany.Core.Patient [Class] (src/patient.fs:5-9)"));

  // 6. Test rkg show MyCompany.Core.PatientValidation.validatePatient
  let mut show_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  show_cmd
    .current_dir(temp_dir.path())
    .args(["show", "MyCompany.Core.PatientValidation.validatePatient"]);
  let show_output = show_cmd.assert().success().get_output().stdout.clone();
  let show_stdout = String::from_utf8(show_output).expect("stdout should be valid utf8");
  assert!(
    show_stdout.contains("Symbol: MyCompany.Core.PatientValidation.validatePatient [Function]")
  );
  assert!(show_stdout.contains("File: src/patient.fs (lines 24-27)"));
  assert!(show_stdout.contains("    let validatePatient (patient: Patient) ="));
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
