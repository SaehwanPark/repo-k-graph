use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedKotlinDependency {
  pub name: String,
  pub dependency_type: String, // "package" or "project"
  pub version_requirement: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedKotlinProject {
  pub name: String,
  pub project_path: String,
  pub target_framework: Option<String>,
  pub dependencies: Vec<ParsedKotlinDependency>,
  pub generated_source_dirs: Vec<String>,
}

/// Parses a Gradle build file (`build.gradle` or `build.gradle.kts`) using robust string scanning.
pub fn parse_gradle(content: &str, project_path: &str) -> ParsedKotlinProject {
  let project_name = Path::new(project_path)
    .parent()
    .and_then(|p| p.file_name())
    .and_then(|s| s.to_str())
    .unwrap_or("app")
    .to_string();

  let mut target_framework = None;
  let mut dependencies = Vec::new();
  let mut generated_source_dirs = Vec::new();

  for line in content.lines() {
    let line = line.trim();
    if line.is_empty() || line.starts_with("//") || line.starts_with("/*") || line.starts_with('*')
    {
      continue;
    }

    // 1. Detect target framework
    if target_framework.is_none() {
      if line.contains("kotlin(\"jvm\")")
        || line.contains("kotlin 'jvm'")
        || line.contains("org.jetbrains.kotlin.jvm")
      {
        target_framework = Some("jvm".to_string());
      } else if line.contains("kotlin(\"android\")")
        || line.contains("kotlin 'android'")
        || line.contains("org.jetbrains.kotlin.android")
        || line.contains("com.android.application")
        || line.contains("com.android.library")
      {
        target_framework = Some("android".to_string());
      } else if line.contains("kotlin(\"multiplatform\")")
        || line.contains("kotlin 'multiplatform'")
        || line.contains("org.jetbrains.kotlin.multiplatform")
      {
        target_framework = Some("multiplatform".to_string());
      }
    }

    // 2. Parse generated source directories
    if line.contains("srcDir") || line.contains("srcDirs") {
      let paths = extract_quoted_paths_from_line(line);
      for p in paths {
        if p.contains("generated") || p.contains("ksp") || p.contains("kapt") {
          generated_source_dirs.push(p);
        }
      }
    }

    // 3. Parse dependencies
    let is_dep_line = line.contains("implementation")
      || line.contains("testImplementation")
      || line.contains("api")
      || line.contains("compileOnly")
      || line.contains("runtimeOnly")
      || line.contains("kapt")
      || line.contains("annotationProcessor");

    if is_dep_line {
      if line.contains("project(") || line.contains("project ") {
        // Project reference
        if let Some(subproj) = extract_project_name(line) {
          dependencies.push(ParsedKotlinDependency {
            name: subproj,
            dependency_type: "project".to_string(),
            version_requirement: None,
          });
        }
      } else {
        // Package dependency
        if let Some((name, version)) = extract_package_dependency(line) {
          dependencies.push(ParsedKotlinDependency {
            name,
            dependency_type: "package".to_string(),
            version_requirement: version,
          });
        }
      }
    }
  }

  // Fallback target_framework to jvm if none detected
  let tf = target_framework.or_else(|| {
    if project_path.ends_with(".kts") || project_path.ends_with(".gradle") {
      Some("jvm".to_string())
    } else {
      None
    }
  });

  ParsedKotlinProject {
    name: project_name,
    project_path: project_path.to_string(),
    target_framework: tf,
    dependencies,
    generated_source_dirs,
  }
}

/// Parses a Maven `pom.xml` file using robust string scanning.
pub fn parse_pom(content: &str, project_path: &str) -> ParsedKotlinProject {
  let fallback_name = Path::new(project_path)
    .parent()
    .and_then(|p| p.file_name())
    .and_then(|s| s.to_str())
    .unwrap_or("app")
    .to_string();

  let mut project_name = None;
  let mut dependencies = Vec::new();

  // Simple state machine for pom XML parsing
  let mut in_dependencies = false;
  let mut in_dependency = false;
  let mut current_group = String::new();
  let mut current_artifact = String::new();
  let mut current_version = String::new();

  for line in content.lines() {
    let line = line.trim();
    if line.is_empty() {
      continue;
    }

    if line.contains("<dependencies>") {
      in_dependencies = true;
      continue;
    }
    if line.contains("</dependencies>") {
      in_dependencies = false;
      continue;
    }

    if in_dependencies {
      if line.contains("<dependency>") {
        in_dependency = true;
        current_group.clear();
        current_artifact.clear();
        current_version.clear();
        continue;
      }
      if line.contains("</dependency>") {
        in_dependency = false;
        if !current_group.is_empty() && !current_artifact.is_empty() {
          let name = format!("{}:{}", current_group, current_artifact);
          let version = if current_version.is_empty() {
            None
          } else {
            Some(current_version.clone())
          };
          dependencies.push(ParsedKotlinDependency {
            name,
            dependency_type: "package".to_string(),
            version_requirement: version,
          });
        }
        continue;
      }

      if in_dependency {
        if let Some(g) = extract_xml_tag(line, "groupId") {
          current_group = g;
        } else if let Some(a) = extract_xml_tag(line, "artifactId") {
          current_artifact = a;
        } else if let Some(v) = extract_xml_tag(line, "version") {
          current_version = v;
        }
      }
    } else {
      // Main project attributes
      project_name = project_name.or_else(|| extract_xml_tag(line, "artifactId"));
    }
  }

  ParsedKotlinProject {
    name: project_name.unwrap_or(fallback_name),
    project_path: project_path.to_string(),
    target_framework: Some("pom".to_string()),
    dependencies,
    generated_source_dirs: Vec::new(),
  }
}

// Helpers
fn extract_project_name(line: &str) -> Option<String> {
  // e.g., implementation(project(":my-subproject"))
  // or implementation project(':my-subproject')
  let start_pat = if line.contains("project(") {
    "project("
  } else {
    "project"
  };
  if let Some(start_idx) = line.find(start_pat) {
    let sub = &line[start_idx + start_pat.len()..];
    let mut clean = String::new();
    let mut in_quotes = false;
    for c in sub.chars() {
      if c == '"' || c == '\'' {
        in_quotes = !in_quotes;
        continue;
      }
      if in_quotes {
        if c != ':' {
          // strip leading Gradle project colon
          clean.push(c);
        }
      } else if c == ')' && start_pat == "project(" {
        break;
      }
    }
    let clean = clean.trim().to_string();
    if !clean.is_empty() {
      return Some(clean);
    }
  }
  None
}

fn extract_package_dependency(line: &str) -> Option<(String, Option<String>)> {
  // e.g., implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.7.3")
  // or implementation 'org.jetbrains.kotlinx:kotlinx-coroutines-core:1.7.3'
  // or implementation(group = "org.jetbrains.kotlinx", name = "kotlinx-coroutines-core", version = "1.7.3")

  if line.contains("group =") || line.contains("group:") {
    let group = extract_named_attr(line, "group");
    let name = extract_named_attr(line, "name").or_else(|| extract_named_attr(line, "module"));
    let version = extract_named_attr(line, "version");

    if let (Some(g), Some(n)) = (group, name) {
      return Some((format!("{}:{}", g, n), version));
    }
  }

  // Look for quotes containing colon-delimited string
  let mut quotes = Vec::new();
  let mut in_quote = false;
  let mut quote_char = ' ';
  let mut current = String::new();

  for c in line.chars() {
    if c == '"' || c == '\'' {
      if in_quote {
        if c == quote_char {
          in_quote = false;
          quotes.push(current.clone());
          current.clear();
        } else {
          current.push(c);
        }
      } else {
        in_quote = true;
        quote_char = c;
      }
    } else if in_quote {
      current.push(c);
    }
  }

  for q in quotes {
    let parts: Vec<&str> = q.split(':').collect();
    if parts.len() >= 2 {
      let group = parts[0].trim();
      let artifact = parts[1].trim();
      if !group.is_empty() && !artifact.is_empty() {
        let name = format!("{}:{}", group, artifact);
        let version = if parts.len() >= 3 {
          let v = parts[2].trim().to_string();
          if v.is_empty() { None } else { Some(v) }
        } else {
          None
        };
        return Some((name, version));
      }
    }
  }

  None
}

fn extract_named_attr(line: &str, attr: &str) -> Option<String> {
  // Looks for attr = "value" or attr: "value"
  for pattern in &[
    format!("{} =", attr),
    format!("{}=", attr),
    format!("{}:", attr),
  ] {
    if let Some(idx) = line.find(pattern) {
      let sub = &line[idx + pattern.len()..];
      let mut val = String::new();
      let mut in_quotes = false;
      for c in sub.chars() {
        if c == '"' || c == '\'' {
          if in_quotes {
            break;
          }
          in_quotes = true;
          continue;
        }
        if in_quotes {
          val.push(c);
        } else if c == ',' || c == ')' {
          break;
        }
      }
      let val = val.trim().to_string();
      if !val.is_empty() {
        return Some(val);
      }
    }
  }
  None
}

fn extract_xml_tag(line: &str, tag: &str) -> Option<String> {
  let open = format!("<{}>", tag);
  let close = format!("</{}>", tag);
  if let (Some(s_idx), Some(e_idx)) = (line.find(&open), line.find(&close)) {
    let val_start = s_idx + open.len();
    if e_idx >= val_start {
      return Some(line[val_start..e_idx].trim().to_string());
    }
  }
  None
}

fn extract_quoted_paths_from_line(line: &str) -> Vec<String> {
  let mut paths = Vec::new();
  let mut current = String::new();
  let mut in_quote = false;
  let mut quote_char = ' ';
  for c in line.chars() {
    if c == '"' || c == '\'' {
      if in_quote {
        if c == quote_char {
          in_quote = false;
          let path = current.trim().to_string();
          if !path.is_empty() {
            paths.push(path);
          }
          current.clear();
        } else {
          current.push(c);
        }
      } else {
        in_quote = true;
        quote_char = c;
      }
    } else if in_quote {
      current.push(c);
    }
  }
  paths
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parsing_gradle_kts() {
    let content = r#"
      plugins {
          kotlin("jvm") version "1.9.0"
      }
      dependencies {
          implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.7.3")
          testImplementation 'org.junit.jupiter:junit-jupiter:5.10.0'
          implementation(project(":shared-module"))
          api(group = "org.slf4j", name = "slf4j-api", version = "2.0.9")
      }
    "#;

    let proj = parse_gradle(content, "my-app/build.gradle.kts");
    assert_eq!(proj.name, "my-app");
    assert_eq!(proj.target_framework.as_deref(), Some("jvm"));

    assert_eq!(proj.dependencies.len(), 4);

    assert_eq!(
      proj.dependencies[0].name,
      "org.jetbrains.kotlinx:kotlinx-coroutines-core"
    );
    assert_eq!(proj.dependencies[0].dependency_type, "package");
    assert_eq!(
      proj.dependencies[0].version_requirement.as_deref(),
      Some("1.7.3")
    );

    assert_eq!(proj.dependencies[1].name, "org.junit.jupiter:junit-jupiter");
    assert_eq!(proj.dependencies[1].dependency_type, "package");
    assert_eq!(
      proj.dependencies[1].version_requirement.as_deref(),
      Some("5.10.0")
    );

    assert_eq!(proj.dependencies[2].name, "shared-module");
    assert_eq!(proj.dependencies[2].dependency_type, "project");
    assert_eq!(proj.dependencies[2].version_requirement, None);

    assert_eq!(proj.dependencies[3].name, "org.slf4j:slf4j-api");
    assert_eq!(proj.dependencies[3].dependency_type, "package");
    assert_eq!(
      proj.dependencies[3].version_requirement.as_deref(),
      Some("2.0.9")
    );
  }

  #[test]
  fn test_parsing_pom_xml() {
    let content = r#"
      <project>
          <modelVersion>4.0.0</modelVersion>
          <groupId>com.example</groupId>
          <artifactId>my-maven-app</artifactId>
          <version>1.0.0</version>
          <dependencies>
              <dependency>
                  <groupId>org.jetbrains.kotlin</groupId>
                  <artifactId>kotlin-stdlib</artifactId>
                  <version>1.9.0</version>
              </dependency>
              <dependency>
                  <groupId>com.google.guava</groupId>
                  <artifactId>guava</artifactId>
              </dependency>
          </dependencies>
      </project>
    "#;

    let proj = parse_pom(content, "pom.xml");
    assert_eq!(proj.name, "my-maven-app");
    assert_eq!(proj.target_framework.as_deref(), Some("pom"));

    assert_eq!(proj.dependencies.len(), 2);

    assert_eq!(
      proj.dependencies[0].name,
      "org.jetbrains.kotlin:kotlin-stdlib"
    );
    assert_eq!(proj.dependencies[0].dependency_type, "package");
    assert_eq!(
      proj.dependencies[0].version_requirement.as_deref(),
      Some("1.9.0")
    );

    assert_eq!(proj.dependencies[1].name, "com.google.guava:guava");
    assert_eq!(proj.dependencies[1].dependency_type, "package");
    assert_eq!(proj.dependencies[1].version_requirement, None);
  }

  #[test]
  fn test_parsing_gradle_generated_sources() {
    let content = r#"
      kotlin {
          sourceSets {
              main {
                  kotlin.srcDir("build/generated/ksp/main/kotlin")
                  kotlin.srcDirs("build/generated/source/kapt/main")
              }
          }
      }
    "#;
    let proj = parse_gradle(content, "app/build.gradle.kts");
    assert!(
      proj
        .generated_source_dirs
        .contains(&"build/generated/ksp/main/kotlin".to_string())
    );
    assert!(
      proj
        .generated_source_dirs
        .contains(&"build/generated/source/kapt/main".to_string())
    );
  }
}
