use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSwiftDependency {
  pub name: String,
  pub dependency_type: String, // "package" or "project"
  pub version_requirement: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSwiftProject {
  pub name: String,
  pub project_path: String,
  pub target_framework: Option<String>,
  pub dependencies: Vec<ParsedSwiftDependency>,
}

/// Parses a Package.swift file using robust string scanning.
pub fn parse_package_swift(content: &str, project_path: &str) -> ParsedSwiftProject {
  let mut name = None;
  let mut dependencies = Vec::new();
  let mut tools_version: Option<String> = None;

  for line in content.lines() {
    let line = line.trim();

    // Parse "// swift-tools-version:" comment (must come before skipping comments)
    if line.starts_with("// swift-tools-version:") {
      let ver = line
        .trim_start_matches("// swift-tools-version:")
        .trim()
        .to_string();
      if !ver.is_empty() {
        tools_version = Some(ver);
      }
      continue;
    }

    if line.is_empty() || line.starts_with("//") || line.starts_with("/*") || line.starts_with('*')
    {
      continue;
    }

    // 1. Detect package name (first occurrence wins)
    if name.is_none()
      && let Some(idx) = line.find("name:")
      && let Some(val) = extract_string_literal(&line[idx + 5..])
    {
      name = Some(val);
    }

    // 2. Detect package dependencies
    if line.contains(".package(") {
      let is_url = line.contains("url:");
      let is_path = line.contains("path:");

      if is_url
        && let Some(url_idx) = line.find("url:")
        && let Some(url) = extract_string_literal(&line[url_idx + 4..])
      {
        let base = url.split('/').next_back().unwrap_or(&url);
        let pkg_name = base
          .strip_suffix(".git")
          .unwrap_or_else(|| url.split('/').next_back().unwrap_or(&url))
          .to_string();

        let version = if let Some(from_idx) = line.find("from:") {
          extract_string_literal(&line[from_idx + 5..])
        } else if let Some(up_idx) = line.find("upToNextMajor(from:") {
          extract_string_literal(&line[up_idx + 19..])
        } else {
          None
        };

        dependencies.push(ParsedSwiftDependency {
          name: pkg_name,
          dependency_type: "package".to_string(),
          version_requirement: version,
        });
      } else if is_path
        && let Some(path_idx) = line.find("path:")
        && let Some(path) = extract_string_literal(&line[path_idx + 5..])
      {
        let pkg_name = Path::new(&path)
          .file_name()
          .and_then(|s| s.to_str())
          .unwrap_or(&path)
          .to_string();

        dependencies.push(ParsedSwiftDependency {
          name: pkg_name,
          dependency_type: "project".to_string(),
          version_requirement: None,
        });
      }
    }
  }

  let fallback_name = Path::new(project_path)
    .parent()
    .and_then(|p| p.file_name())
    .and_then(|s| s.to_str())
    .unwrap_or("App")
    .to_string();

  ParsedSwiftProject {
    name: name.unwrap_or(fallback_name),
    project_path: project_path.to_string(),
    target_framework: tools_version,
    dependencies,
  }
}

fn extract_string_literal(s: &str) -> Option<String> {
  let mut start_idx = None;
  let mut quote_char = ' ';
  for (i, c) in s.char_indices() {
    if c == '"' || c == '\'' {
      if let Some(start) = start_idx {
        if c == quote_char {
          return Some(s[start..i].to_string());
        }
      } else {
        start_idx = Some(i + 1);
        quote_char = c;
      }
    }
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parsing_package_swift() {
    let content = r#"
      // swift-tools-version: 5.9
      import PackageDescription

      let package = Package(
          name: "MySwiftPackage",
          platforms: [
              .macOS(.v13)
          ],
          products: [
              .library(name: "MyLib", targets: ["MyLib"])
          ],
          dependencies: [
              .package(url: "https://github.com/apple/swift-algorithms.git", from: "1.2.0"),
              .package(path: "../LocalPackage")
          ],
          targets: [
              .target(
                  name: "MyLib",
                  dependencies: [
                      .product(name: "Algorithms", package: "swift-algorithms")
                  ]
              )
          ]
      )
    "#;

    let proj = parse_package_swift(content, "MySwiftPackage/Package.swift");
    assert_eq!(proj.name, "MySwiftPackage");
    assert_eq!(proj.target_framework.as_deref(), Some("5.9"));

    assert_eq!(proj.dependencies.len(), 2);

    assert_eq!(proj.dependencies[0].name, "swift-algorithms");
    assert_eq!(proj.dependencies[0].dependency_type, "package");
    assert_eq!(
      proj.dependencies[0].version_requirement.as_deref(),
      Some("1.2.0")
    );

    assert_eq!(proj.dependencies[1].name, "LocalPackage");
    assert_eq!(proj.dependencies[1].dependency_type, "project");
    assert_eq!(proj.dependencies[1].version_requirement, None);
  }
}
