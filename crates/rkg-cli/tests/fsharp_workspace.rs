use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn test_fsharp_workspace_indexing_and_commands() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Write F# Solution file
  let sln_content = r#"
Microsoft Visual Studio Solution File, Format Version 12.00
Project("{F2A71F9B-5D33-465A-A702-920D77279786}") = "MyLib", "src\MyLib\MyLib.fsproj", "{GUID}"
Project("{F2A71F9B-5D33-465A-A702-920D77279786}") = "Core", "src\Core\Core.fsproj", "{GUID2}"
  "#;
  write_file(temp_dir.path(), "MyWorkspace.sln", sln_content);

  // 2. Write global paket.dependencies
  let paket_deps = r#"
nuget Newtonsoft.Json ~> 13.0.1
nuget FSharp.Core >= 6.0.0
  "#;
  write_file(temp_dir.path(), "paket.dependencies", paket_deps);

  // 3. Write MyLib's local paket.references
  let paket_refs = r#"
Newtonsoft.Json
  "#;
  write_file(temp_dir.path(), "src/MyLib/paket.references", paket_refs);

  // 4. Write src/Core/Core.fsproj
  let core_fsproj = r#"
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
  </PropertyGroup>
  <ItemGroup>
    <Compile Include="Core.fs" />
  </ItemGroup>
</Project>
  "#;
  write_file(temp_dir.path(), "src/Core/Core.fsproj", core_fsproj);
  write_file(
    temp_dir.path(),
    "src/Core/Core.fs",
    "namespace Core\nlet value = 42",
  );

  // 5. Write src/MyLib/MyLib.fsproj
  let mylib_fsproj = r#"
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFrameworks>net8.0;netstandard2.1</TargetFrameworks>
  </PropertyGroup>
  <ItemGroup>
    <Compile Include="Library.fs" />
  </ItemGroup>
  <ItemGroup>
    <PackageReference Include="FSharp.Core" Version="8.0.100" />
    <ProjectReference Include="..\Core\Core.fsproj" />
  </ItemGroup>
</Project>
  "#;
  write_file(temp_dir.path(), "src/MyLib/MyLib.fsproj", mylib_fsproj);
  write_file(
    temp_dir.path(),
    "src/MyLib/Library.fs",
    "namespace MyLib\nlet greet = \"hello\"",
  );

  // 6. Initialize database
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 7. Index the workspace repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let index_output = index_cmd.assert().success().get_output().stdout.clone();
  let index_stdout = String::from_utf8(index_output).expect("stdout should be valid UTF8");
  assert!(index_stdout.contains("files scanned:"));

  // 8. Test rkg workspace command outputs
  let mut ws_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  ws_cmd.current_dir(temp_dir.path()).arg("workspace");
  let ws_output = ws_cmd.assert().success().get_output().stdout.clone();
  let ws_stdout = String::from_utf8(ws_output).expect("stdout should be valid UTF8");

  assert!(ws_stdout.contains("F# PROJECTS"));
  assert!(ws_stdout.contains("MyLib"));
  assert!(ws_stdout.contains("Core"));
  assert!(ws_stdout.contains("src/MyLib/MyLib.fsproj"));
  assert!(ws_stdout.contains("src/Core/Core.fsproj"));
  assert!(ws_stdout.contains("net8.0"));
  assert!(ws_stdout.contains("yes")); // both should be solution members

  // 9. Test rkg deps MyLib command outputs
  let mut deps_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  deps_cmd
    .current_dir(temp_dir.path())
    .args(["deps", "MyLib"]);
  let deps_output = deps_cmd.assert().success().get_output().stdout.clone();
  let deps_stdout = String::from_utf8(deps_output).expect("stdout should be valid UTF8");

  assert!(deps_stdout.contains("Project: MyLib"));
  assert!(deps_stdout.contains("src/MyLib/MyLib.fsproj"));
  assert!(deps_stdout.contains("Core"));
  assert!(deps_stdout.contains("Project Reference"));
  assert!(deps_stdout.contains("../Core/Core.fsproj"));
  assert!(deps_stdout.contains("FSharp.Core"));
  assert!(deps_stdout.contains("NuGet Package"));
  assert!(deps_stdout.contains("8.0.100"));
  assert!(deps_stdout.contains("Newtonsoft.Json")); // Paket reference should be resolved!
  assert!(deps_stdout.contains("~> 13.0.1"));
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
