use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDependency {
  pub name: String,
  pub version_requirement: Option<String>,
  pub is_workspace_dependency: bool,
  pub inherits_workspace: bool,
  pub features: Vec<String>,
  pub is_dev: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPackage {
  pub name: String,
  pub version: String,
  pub manifest_path: String,
  pub dependencies: Vec<ParsedDependency>,
  pub features: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedWorkspace {
  pub members: Vec<String>,
  pub workspace_dependencies: HashMap<String, ParsedDependency>,
}

/// Parses a Cargo.toml file using a robust line-by-line parser.
pub fn parse_cargo_toml(
  content: &str,
  manifest_path: &str,
) -> (Option<ParsedPackage>, Option<ParsedWorkspace>) {
  let mut current_section = String::new();

  let mut pkg_name = String::new();
  let mut pkg_version = String::new();
  let mut dependencies = Vec::new();
  let mut workspace_members = Vec::new();
  let mut workspace_dependencies = HashMap::new();
  let mut features = HashMap::new();

  for line in content.lines() {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
      continue;
    }

    if line.starts_with('[') && line.ends_with(']') {
      current_section = line[1..line.len() - 1].trim().to_string();
      continue;
    }

    if let Some(pos) = line.find('=') {
      let key = line[..pos].trim();
      let value = line[pos + 1..].trim();

      match current_section.as_str() {
        "package" => {
          if key == "name" {
            pkg_name = clean_quotes(value);
          } else if key == "version" {
            pkg_version = clean_quotes(value);
          }
        }
        "workspace" if key == "members" => {
          workspace_members = parse_array(value);
        }
        "dependencies" | "dev-dependencies" | "build-dependencies" => {
          let is_dev = current_section == "dev-dependencies";
          if let Some(dep) = parse_dependency_line(key, value, is_dev) {
            dependencies.push(dep);
          }
        }
        "workspace.dependencies" => {
          if let Some(dep) = parse_dependency_line(key, value, false) {
            workspace_dependencies.insert(dep.name.clone(), dep);
          }
        }
        "features" => {
          let list = parse_array(value);
          features.insert(clean_quotes(key), list);
        }
        _ => {}
      }
    }
  }

  // Inherit workspace dependencies if configured
  for dep in &mut dependencies {
    if dep.inherits_workspace {
      let ws_deps_ref = &workspace_dependencies;
      if let Some(ws_dep) = ws_deps_ref.get(&dep.name) {
        dep.is_workspace_dependency = ws_dep.is_workspace_dependency;
        if dep.version_requirement.is_none() {
          dep.version_requirement = ws_dep.version_requirement.clone();
        }
        for f in &ws_dep.features {
          if !dep.features.contains(f) {
            dep.features.push(f.clone());
          }
        }
      }
    }
  }

  let package = if !pkg_name.is_empty() {
    Some(ParsedPackage {
      name: pkg_name,
      version: if pkg_version.is_empty() {
        "0.1.0".to_string()
      } else {
        pkg_version
      },
      manifest_path: manifest_path.to_string(),
      dependencies,
      features,
    })
  } else {
    None
  };

  let workspace = if !workspace_members.is_empty() || !workspace_dependencies.is_empty() {
    Some(ParsedWorkspace {
      members: workspace_members,
      workspace_dependencies,
    })
  } else {
    None
  };

  (package, workspace)
}

/// Parses Cargo.lock and returns a map of external package name -> exact resolved version.
pub fn parse_cargo_lock(content: &str) -> HashMap<String, String> {
  let mut resolved = HashMap::new();
  let mut current_pkg_name = String::new();
  let mut current_pkg_version = String::new();
  let mut inside_package = false;

  for line in content.lines() {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
      continue;
    }

    if line == "[[package]]" {
      if !current_pkg_name.is_empty() && !current_pkg_version.is_empty() {
        resolved.insert(current_pkg_name.clone(), current_pkg_version.clone());
      }
      current_pkg_name.clear();
      current_pkg_version.clear();
      inside_package = true;
      continue;
    }

    if line.starts_with('[') && line.ends_with(']') {
      if !current_pkg_name.is_empty() && !current_pkg_version.is_empty() {
        resolved.insert(current_pkg_name.clone(), current_pkg_version.clone());
      }
      current_pkg_name.clear();
      current_pkg_version.clear();
      inside_package = false;
      continue;
    }

    if inside_package {
      let line_ref = line;
      if let Some(pos) = line_ref.find('=') {
        let key = line[..pos].trim();
        let value = line[pos + 1..].trim();
        if key == "name" {
          current_pkg_name = clean_quotes(value);
        } else if key == "version" {
          current_pkg_version = clean_quotes(value);
        }
      }
    }
  }

  // Push final package
  if !current_pkg_name.is_empty() && !current_pkg_version.is_empty() {
    resolved.insert(current_pkg_name, current_pkg_version);
  }

  resolved
}

/// Finds the closest matching Cargo package manifest for a given file path.
pub fn find_cargo_package_for_path<'a>(
  packages: &'a [ParsedPackage],
  file_path: &str,
) -> Option<&'a ParsedPackage> {
  let mut best_pkg = None;
  let mut best_len = 0;

  for pkg in packages {
    let manifest_dir = if let Some(idx) = pkg.manifest_path.rfind('/') {
      &pkg.manifest_path[..idx]
    } else {
      ""
    };

    if manifest_dir.is_empty() || file_path.starts_with(&(manifest_dir.to_string() + "/")) {
      let len = manifest_dir.len();
      if len >= best_len {
        best_len = len;
        best_pkg = Some(pkg);
      }
    }
  }

  best_pkg
}

fn clean_quotes(s: &str) -> String {
  let s = s.trim();
  let s = if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\''))
  {
    &s[1..s.len() - 1]
  } else {
    s
  };
  s.trim().to_string()
}

fn parse_array(value: &str) -> Vec<String> {
  let mut list = Vec::new();
  let cleaned = value.trim();
  if !cleaned.starts_with('[') || !cleaned.ends_with(']') {
    return list;
  }
  let inner = &cleaned[1..cleaned.len() - 1];
  for item in inner.split(',') {
    let cleaned_item = clean_quotes(item);
    if !cleaned_item.is_empty() {
      list.push(cleaned_item);
    }
  }
  list
}

fn parse_dependency_line(key: &str, value: &str, is_dev: bool) -> Option<ParsedDependency> {
  let mut dep_name = clean_quotes(key);
  if dep_name.is_empty() {
    return None;
  }

  let mut inherits_workspace = false;
  if dep_name.ends_with(".workspace") {
    dep_name = dep_name[..dep_name.len() - 10].trim().to_string();
    if value.trim() == "true" {
      inherits_workspace = true;
    }
  }

  let value_trimmed = value.trim();
  if inherits_workspace {
    Some(ParsedDependency {
      name: dep_name,
      version_requirement: None,
      is_workspace_dependency: false,
      inherits_workspace: true,
      features: Vec::new(),
      is_dev,
    })
  } else if value_trimmed.starts_with('{') && value_trimmed.ends_with('}') {
    // Parsing inline table like: { path = "../rkg-core", features = ["foo"] }
    let inner = &value_trimmed[1..value_trimmed.len() - 1];
    let mut version_requirement = None;
    let mut is_workspace_dependency = false;
    let mut inherits_workspace_inline = false;
    let mut features = Vec::new();

    for part in inner.split(',') {
      if let Some(pos) = part.find('=') {
        let k = part[..pos].trim();
        let v = part[pos + 1..].trim();
        if k == "version" {
          version_requirement = Some(clean_quotes(v));
        } else if k == "path" {
          is_workspace_dependency = true;
        } else if k == "workspace" {
          inherits_workspace_inline = v == "true";
        } else if k == "features" {
          features = parse_array(v);
        }
      }
    }

    Some(ParsedDependency {
      name: dep_name,
      version_requirement,
      is_workspace_dependency,
      inherits_workspace: inherits_workspace_inline,
      features,
      is_dev,
    })
  } else {
    // Simple version string e.g. "1.0" or "workspace = true" (but simple check)
    let version = clean_quotes(value_trimmed);
    Some(ParsedDependency {
      name: dep_name,
      version_requirement: Some(version),
      is_workspace_dependency: false,
      inherits_workspace: false,
      features: Vec::new(),
      is_dev,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parsing_cargo_toml() {
    let toml = r#"
      [package]
      name = "rkg-cli"
      version = "0.1.0"

      [dependencies]
      rkg-core = { path = "../rkg-core", features = ["logging"] }
      tokio = "1.0"
      serde = { workspace = true, features = ["derive"] }

      [workspace.dependencies]
      serde = "1.0.100"

      [features]
      default = ["premium"]
      premium = []
    "#;

    let (pkg, _ws) = parse_cargo_toml(toml, "crates/rkg-cli/Cargo.toml");
    let pkg = pkg.expect("should parse package");
    assert_eq!(pkg.name, "rkg-cli");
    assert_eq!(pkg.version, "0.1.0");

    assert_eq!(pkg.dependencies.len(), 3);

    let core_dep = pkg
      .dependencies
      .iter()
      .find(|d| d.name == "rkg-core")
      .unwrap();
    assert!(core_dep.is_workspace_dependency);
    assert_eq!(core_dep.features, vec!["logging".to_string()]);

    let serde_dep = pkg.dependencies.iter().find(|d| d.name == "serde").unwrap();
    assert_eq!(serde_dep.version_requirement, Some("1.0.100".to_string()));
    assert!(serde_dep.features.contains(&"derive".to_string()));

    let default_features = pkg.features.get("default").unwrap();
    assert_eq!(default_features, &vec!["premium".to_string()]);
  }

  #[test]
  fn test_parsing_dotted_workspace_keys() {
    let toml = r#"
      [package]
      name = "rkg-db"
      version = "0.1.0"

      [dependencies]
      rkg-core.workspace = true
      serde.workspace = true

      [workspace.dependencies]
      rkg-core = { path = "../rkg-core" }
      serde = "1.0.152"
    "#;

    let (pkg, _ws) = parse_cargo_toml(toml, "crates/rkg-db/Cargo.toml");
    let pkg = pkg.expect("should parse package");
    assert_eq!(pkg.dependencies.len(), 2);

    let core_dep = pkg
      .dependencies
      .iter()
      .find(|d| d.name == "rkg-core")
      .unwrap();
    assert!(core_dep.inherits_workspace);
    assert_eq!(core_dep.version_requirement, None);

    let serde_dep = pkg.dependencies.iter().find(|d| d.name == "serde").unwrap();
    assert!(serde_dep.inherits_workspace);
  }

  #[test]
  fn test_parsing_cargo_lock() {
    let lock = r#"
      [[package]]
      name = "serde"
      version = "1.0.152"

      [[package]]
      name = "tokio"
      version = "1.35.1"
    "#;

    let resolved = parse_cargo_lock(lock);
    assert_eq!(resolved.get("serde").unwrap(), "1.0.152");
    assert_eq!(resolved.get("tokio").unwrap(), "1.35.1");
  }
}
