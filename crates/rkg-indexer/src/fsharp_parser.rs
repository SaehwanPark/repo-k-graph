use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFSharpDependency {
  pub name: String,
  pub dependency_type: String, // "package" or "project"
  pub version_requirement: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFSharpProject {
  pub name: String,
  pub project_path: String,
  pub target_framework: Option<String>,
  pub dependencies: Vec<ParsedFSharpDependency>,
}

/// Parses an .fsproj XML file using a robust string scanner.
pub fn parse_fsproj(content: &str, project_path: &str) -> ParsedFSharpProject {
  let project_name = Path::new(project_path)
    .file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or(project_path)
    .to_string();

  let mut target_framework = None;
  let mut dependencies = Vec::new();

  for line in content.lines() {
    let line = line.trim();
    if line.is_empty() {
      continue;
    }

    // 1. Extract TargetFramework or TargetFrameworks
    if target_framework.is_none() {
      if let Some(tf) = extract_tag_value(line, "TargetFramework") {
        target_framework = Some(tf);
      } else if let Some(tf) = extract_tag_value(line, "TargetFrameworks") {
        // Just take the first one or raw list
        let first = tf.split(';').next().unwrap_or("").to_string();
        target_framework = Some(first);
      }
    }

    // 2. Extract PackageReference (NuGet)
    if line.contains("<PackageReference") {
      let name_opt = extract_attribute_value(line, "Include")
        .or_else(|| extract_attribute_value(line, "Update"));
      if let Some(name) = name_opt {
        let version = extract_attribute_value(line, "Version");
        dependencies.push(ParsedFSharpDependency {
          name,
          dependency_type: "package".to_string(),
          version_requirement: version,
        });
      }
    }

    // 3. Extract ProjectReference
    if line.contains("<ProjectReference") {
      let path_opt = extract_attribute_value(line, "Include");
      if let Some(path) = path_opt {
        // Canonicalize relative paths with forward slashes first
        let canonical_path = path.replace('\\', "/");

        // Get name of reference project
        let name = Path::new(&canonical_path)
          .file_stem()
          .and_then(|s| s.to_str())
          .unwrap_or(&canonical_path)
          .to_string();

        dependencies.push(ParsedFSharpDependency {
          name,
          dependency_type: "project".to_string(),
          version_requirement: Some(canonical_path),
        });
      }
    }
  }

  ParsedFSharpProject {
    name: project_name,
    project_path: project_path.to_string(),
    target_framework,
    dependencies,
  }
}

/// Parses a solution (.sln) file and returns listed .fsproj paths.
pub fn parse_sln(content: &str) -> Vec<String> {
  let mut projects = Vec::new();

  for line in content.lines() {
    let line = line.trim();
    // Typical line: Project("{F2A71F9B-5D33-465A-A702-920D77279786}") = "MyLib", "src\MyLib\MyLib.fsproj", "{...}"
    if line.starts_with("Project") && line.contains(".fsproj") {
      // Extract parts by split on comma
      let parts: Vec<&str> = line.split(',').collect();
      if parts.len() >= 2 {
        let path_part = parts[1].trim();
        // Strip quotes
        let clean_path = path_part.replace('"', "").replace('\\', "/");
        if !clean_path.trim().is_empty() {
          projects.push(clean_path.trim().to_string());
        }
      }
    }
  }

  projects
}

/// Parses a paket.dependencies file to map package name -> version range.
pub fn parse_paket_dependencies(content: &str) -> HashMap<String, String> {
  let mut map = HashMap::new();

  for line in content.lines() {
    let line = line.trim();
    if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
      continue;
    }

    // Typical line: nuget Newtonsoft.Json ~> 13.0.1
    if line.starts_with("nuget") {
      let parts: Vec<&str> = line.split_whitespace().collect();
      if parts.len() >= 3 {
        let name = parts[1].to_string();
        let version = parts[2..].join(" ");
        map.insert(name, version);
      } else if parts.len() == 2 {
        let name = parts[1].to_string();
        map.insert(name, "*".to_string());
      }
    }
  }

  map
}

/// Parses a paket.references file to extract referenced package names.
pub fn parse_paket_references(content: &str) -> Vec<String> {
  let mut refs = Vec::new();

  for line in content.lines() {
    let line = line.trim();
    if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
      continue;
    }
    refs.push(line.to_string());
  }

  refs
}

// Helpers
fn extract_tag_value(line: &str, tag: &str) -> Option<String> {
  let open_tag = format!("<{}>", tag);
  let close_tag = format!("</{}>", tag);
  if let (Some(start_idx), Some(end_idx)) = (line.find(&open_tag), line.find(&close_tag)) {
    let val_start = start_idx + open_tag.len();
    if end_idx >= val_start {
      return Some(line[val_start..end_idx].trim().to_string());
    }
  }
  None
}

fn extract_attribute_value(line: &str, attr: &str) -> Option<String> {
  let prefix = format!("{}=", attr);
  if let Some(start_idx) = line.find(&prefix) {
    let search_part = &line[start_idx + prefix.len()..];
    let quote_char = search_part.chars().next()?;
    if quote_char == '"' || quote_char == '\'' {
      let end_idx_opt = search_part[1..].find(quote_char);
      if let Some(end_idx) = end_idx_opt {
        return Some(search_part[1..end_idx + 1].trim().to_string());
      }
    }
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parsing_fsproj() {
    let fsproj = r#"
      <Project Sdk="Microsoft.NET.Sdk">
        <PropertyGroup>
          <TargetFramework>net8.0</TargetFramework>
          <AssemblyName>MyLib</AssemblyName>
        </PropertyGroup>
        <ItemGroup>
          <Compile Include="Library.fs" />
        </ItemGroup>
        <ItemGroup>
          <PackageReference Include="Newtonsoft.Json" Version="13.0.3" />
          <ProjectReference Include="..\Core\Core.fsproj" />
        </ItemGroup>
      </Project>
    "#;

    let parsed = parse_fsproj(fsproj, "src/MyLib/MyLib.fsproj");
    assert_eq!(parsed.name, "MyLib");
    assert_eq!(parsed.target_framework, Some("net8.0".to_string()));
    assert_eq!(parsed.dependencies.len(), 2);

    let pkg_dep = parsed
      .dependencies
      .iter()
      .find(|d| d.dependency_type == "package")
      .unwrap();
    assert_eq!(pkg_dep.name, "Newtonsoft.Json");
    assert_eq!(pkg_dep.version_requirement, Some("13.0.3".to_string()));

    let proj_dep = parsed
      .dependencies
      .iter()
      .find(|d| d.dependency_type == "project")
      .unwrap();
    assert_eq!(proj_dep.name, "Core");
    assert_eq!(
      proj_dep.version_requirement,
      Some("../Core/Core.fsproj".to_string())
    );
  }

  #[test]
  fn test_parsing_sln() {
    let sln = r#"
Project("{F2A71F9B-5D33-465A-A702-920D77279786}") = "MyLib", "src\MyLib\MyLib.fsproj", "{GUID}"
Project("{F2A71F9B-5D33-465A-A702-920D77279786}") = "Core", "src\Core\Core.fsproj", "{GUID2}"
    "#;

    let projs = parse_sln(sln);
    assert_eq!(projs.len(), 2);
    assert_eq!(projs[0], "src/MyLib/MyLib.fsproj");
    assert_eq!(projs[1], "src/Core/Core.fsproj");
  }

  #[test]
  fn test_parsing_paket() {
    let dependencies = r#"
nuget Newtonsoft.Json ~> 13.0.1
nuget FSharp.Core >= 6.0.0
    "#;

    let dep_map = parse_paket_dependencies(dependencies);
    assert_eq!(dep_map.get("Newtonsoft.Json").unwrap(), "~> 13.0.1");
    assert_eq!(dep_map.get("FSharp.Core").unwrap(), ">= 6.0.0");

    let references = r#"
Newtonsoft.Json
FSharp.Core
    "#;

    let refs = parse_paket_references(references);
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0], "Newtonsoft.Json");
    assert_eq!(refs[1], "FSharp.Core");
  }
}
