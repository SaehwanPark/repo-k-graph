use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_fsharp_routes_via_cli() {
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
    <Compile Include="src/api.fs" />
  </ItemGroup>
</Project>
"#;

  let api_content = r#"namespace MyCompany.Web

open Giraffe
open Saturn

let giraffeApp =
    choose [
        GET >=> route "/api/items" >=> text "items"
        POST >=> routef "/api/item/%d" (fun id -> text "item")
    ]

let saturnApp = router {
    get "/saturn/get" indexHandler
    post "/saturn/post" postHandler
}
"#;

  write_file(temp_dir.path(), "basic.fsproj", fsproj_content);
  write_file(temp_dir.path(), "src/api.fs", api_content);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("files scanned: 2"));

  // 4. Test routes query command: rkg routes
  let mut routes_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  routes_cmd.current_dir(temp_dir.path()).arg("routes");
  let routes_output = routes_cmd.assert().success().get_output().stdout.clone();
  let routes_stdout = String::from_utf8(routes_output).expect("stdout should be valid utf8");

  // Verify headers are present
  assert!(routes_stdout.contains("METHOD"));
  assert!(routes_stdout.contains("PATH"));
  assert!(routes_stdout.contains("HANDLER"));

  // Verify Giraffe route details
  assert!(routes_stdout.contains("GET"));
  assert!(routes_stdout.contains("/api/items"));
  assert!(routes_stdout.contains("MyCompany.Web.giraffeApp"));

  assert!(routes_stdout.contains("POST"));
  assert!(routes_stdout.contains("/api/item/%d"));

  // Verify Saturn route details
  assert!(routes_stdout.contains("/saturn/get"));
  assert!(routes_stdout.contains("/saturn/post"));
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
