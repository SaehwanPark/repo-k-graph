use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAndroidComponent {
  pub name: String,
  pub component_type: String, // "activity", "service", "receiver", "provider", "application"
  pub class_name: String,
  pub permission: Option<String>,
  pub intent_actions: Vec<String>,
  pub intent_categories: Vec<String>,
  pub start_line: usize,
  pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAndroidResource {
  pub name: String,
  pub resource_type: String, // "layout", "string", "id", "navigation", "dimen", "color", "drawable"
  pub value: Option<String>,
  pub start_line: usize,
  pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAndroidLink {
  pub source: String,
  pub target: String,
  pub kind: String,
}

/// Helper to count line number from char index
fn index_to_line(content: &str, index: usize) -> usize {
  content[..index].chars().filter(|&c| c == '\n').count() + 1
}

/// Helper to replace comments in XML content with space/newline equivalents
/// to avoid parsing commented-out elements while preserving exact character
/// offsets and line counts.
fn strip_comments(content: &str) -> String {
  let mut result = String::with_capacity(content.len());
  let mut cur = content;
  while let Some(start_idx) = cur.find("<!--") {
    result.push_str(&cur[..start_idx]);
    if let Some(end_idx) = cur[start_idx..].find("-->") {
      let comment_len = end_idx + 3;
      let comment_str = &cur[start_idx..start_idx + comment_len];
      for c in comment_str.chars() {
        if c == '\n' {
          result.push('\n');
        } else {
          result.push(' ');
        }
      }
      cur = &cur[start_idx + comment_len..];
    } else {
      for c in cur.chars() {
        if c == '\n' {
          result.push('\n');
        } else {
          result.push(' ');
        }
      }
      cur = "";
      break;
    }
  }
  result.push_str(cur);
  result
}

/// Helper to resolve relative class name based on manifest package
fn resolve_class_name(class_name: &str, package: &str) -> String {
  let trimmed = class_name.trim();
  if trimmed.starts_with('.') {
    format!("{}{}", package, trimmed)
  } else if !trimmed.contains('.') {
    if package.is_empty() {
      trimmed.to_string()
    } else {
      format!("{}.{}", package, trimmed)
    }
  } else {
    trimmed.to_string()
  }
}

/// Helper to parse attribute value from raw element string without regex
fn extract_attribute(element_str: &str, attr_name: &str) -> Option<String> {
  let mut cur = element_str;
  while let Some(idx) = cur.find(attr_name) {
    let after = &cur[idx + attr_name.len()..];

    // Check if the preceding character is valid (colon or whitespace)
    let preceding_valid = if idx == 0 {
      true
    } else {
      let prec = cur[..idx].chars().next_back().unwrap();
      prec.is_whitespace() || prec == ':'
    };

    if preceding_valid && after.starts_with('=') {
      let val_part = &after[1..];
      let quote_char = val_part.chars().next()?;
      if let Some(end_idx) = Some(quote_char)
        .filter(|&c| c == '"' || c == '\'')
        .and_then(|c| val_part[1..].find(c))
      {
        return Some(val_part[1..end_idx + 1].to_string());
      }
    }
    if after.is_empty() {
      break;
    }
    let offset = after.chars().next().map(|c| c.len_utf8()).unwrap_or(0);
    if offset == 0 {
      break;
    }
    cur = &after[offset..];
  }
  None
}

/// Parse AndroidManifest.xml
pub fn parse_manifest(content: &str) -> (Vec<ParsedAndroidComponent>, Vec<String>) {
  let mut components = Vec::new();
  let mut permissions = Vec::new();

  let content_clean = strip_comments(content);

  // Extract package
  let package = extract_attribute(&content_clean, "package").unwrap_or_default();

  // 1. Extract uses-permissions
  for line in content_clean.lines() {
    let line = line.trim();
    let perm_opt = if line.contains("<uses-permission") {
      extract_attribute(line, "name")
    } else {
      None
    };
    if let Some(perm) = perm_opt {
      permissions.push(perm);
    }
  }

  // 2. Extract components
  let component_tags = ["activity", "service", "receiver", "provider", "application"];
  for tag in &component_tags {
    let open_tag = format!("<{}", tag);
    let mut cur_idx = 0;
    while let Some(start_offset) = content_clean[cur_idx..].find(&open_tag) {
      let start_idx = cur_idx + start_offset;
      let start_line = index_to_line(&content_clean, start_idx);

      // Verify prefix-overlapping tags (e.g. <activity-alias>) are not matched
      let next_char = content_clean[start_idx + open_tag.len()..].chars().next();
      if next_char.is_some_and(|c| !c.is_whitespace() && c != '>' && c != '/') {
        cur_idx = start_idx + open_tag.len();
        continue;
      }

      // Find closing tag or self-closing boundary
      let remaining = &content_clean[start_idx..];
      let mut end_idx = start_idx;
      let mut element_content = String::new();
      let mut is_self_closing = false;

      if let Some(open_tag_end) = remaining.find('>') {
        let open_tag_str = &remaining[..=open_tag_end];
        if open_tag_str.trim().ends_with("/>") {
          is_self_closing = true;
          end_idx = start_idx + open_tag_end + 1;
          element_content = open_tag_str.to_string();
        } else {
          // Look for matching closing tag
          let close_tag = format!("</{}>", tag);
          if let Some(close_tag_idx) = remaining.find(&close_tag) {
            end_idx = start_idx + close_tag_idx + close_tag.len();
            element_content = remaining[..close_tag_idx + close_tag.len()].to_string();
          } else {
            // Fallback
            end_idx = start_idx + open_tag_end + 1;
            element_content = open_tag_str.to_string();
          }
        }
      }

      let end_line = index_to_line(&content_clean, end_idx);
      cur_idx = end_idx;

      // Extract attributes
      let name_val = extract_attribute(&element_content, "name");
      let permission_val = extract_attribute(&element_content, "permission");

      let (name, class_name) = if *tag == "application" {
        let name_str = name_val
          .clone()
          .unwrap_or_else(|| "Application".to_string());
        let name_clean = name_str.strip_prefix('.').unwrap_or(&name_str).to_string();
        let class_resolved = if let Some(ref n) = name_val {
          resolve_class_name(n, &package)
        } else {
          "".to_string()
        };
        (name_clean, class_resolved)
      } else {
        if let Some(ref raw_name) = name_val {
          let resolved = resolve_class_name(raw_name, &package);
          let short_name = raw_name
            .split('.')
            .next_back()
            .unwrap_or(raw_name)
            .to_string();
          let short_name_clean = short_name
            .strip_prefix('.')
            .unwrap_or(&short_name)
            .to_string();
          (short_name_clean, resolved)
        } else {
          continue; // skip if no name
        }
      };

      // Intent filters
      let mut intent_actions = Vec::new();
      let mut intent_categories = Vec::new();
      if !is_self_closing && *tag != "application" {
        for line in element_content.lines() {
          let line = line.trim();
          let act_opt = if line.contains("<action") {
            extract_attribute(line, "name")
          } else {
            None
          };
          if let Some(act) = act_opt {
            intent_actions.push(act);
          }

          let cat_opt = if line.contains("<category") {
            extract_attribute(line, "name")
          } else {
            None
          };
          if let Some(cat) = cat_opt {
            intent_categories.push(cat);
          }
        }
      }

      components.push(ParsedAndroidComponent {
        name,
        component_type: tag.to_string(),
        class_name,
        permission: permission_val,
        intent_actions,
        intent_categories,
        start_line,
        end_line,
      });
    }
  }

  (components, permissions)
}

/// Helper to scan all resource references of format "@type/name"
fn extract_all_resource_references(line: &str) -> Vec<(String, String)> {
  let mut refs = Vec::new();

  // Double quotes
  let mut cur = line;
  while let Some(idx) = cur.find("\"@") {
    let after = &cur[idx + 2..];
    if let Some(end_idx) = after.find('"') {
      let val = &after[..end_idx];
      if let Some(slash_idx) = val.find('/') {
        let res_type = &val[..slash_idx];
        let res_name = &val[slash_idx + 1..];
        if matches!(
          res_type,
          "layout" | "string" | "drawable" | "color" | "dimen" | "id"
        ) {
          refs.push((res_type.to_string(), res_name.to_string()));
        }
      }
      cur = &after[end_idx + 1..];
    } else {
      break;
    }
  }

  // Single quotes
  let mut cur = line;
  while let Some(idx) = cur.find("'@") {
    let after = &cur[idx + 2..];
    if let Some(end_idx) = after.find('\'') {
      let val = &after[..end_idx];
      if let Some(slash_idx) = val.find('/') {
        let res_type = &val[..slash_idx];
        let res_name = &val[slash_idx + 1..];
        if matches!(
          res_type,
          "layout" | "string" | "drawable" | "color" | "dimen" | "id"
        ) {
          refs.push((res_type.to_string(), res_name.to_string()));
        }
      }
      cur = &after[end_idx + 1..];
    } else {
      break;
    }
  }

  refs
}

/// Parse Android Layout XML
pub fn parse_layout(
  content: &str,
  file_path: &str,
) -> (Vec<ParsedAndroidResource>, Vec<ParsedAndroidLink>) {
  let mut resources = Vec::new();
  let mut links = Vec::new();

  let file_name = Path::new(file_path)
    .file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or(file_path);

  // 1. Implicit layout resource definition
  let layout_qname = format!("R.layout.{}", file_name);
  resources.push(ParsedAndroidResource {
    name: file_name.to_string(),
    resource_type: "layout".to_string(),
    value: None,
    start_line: 1,
    end_line: 1,
  });

  let content_clean = strip_comments(content);
  let file_lines: Vec<&str> = content_clean.lines().collect();

  // 2. Find all android:id="@+id/..." or android:id="@id/..." and references
  for (idx, line) in file_lines.iter().enumerate() {
    let line_num = idx + 1;
    let line = line.trim();
    if line.is_empty() {
      continue;
    }

    if let Some(id_val) = extract_attribute(line, "id") {
      let id_name = id_val.replace("@+id/", "").replace("@id/", "");
      resources.push(ParsedAndroidResource {
        name: id_name.clone(),
        resource_type: "id".to_string(),
        value: None,
        start_line: line_num,
        end_line: line_num,
      });

      links.push(ParsedAndroidLink {
        source: layout_qname.clone(),
        target: format!("R.id.{}", id_name),
        kind: "ConfiguredBy".to_string(),
      });
    }

    // Extract resources references in attributes
    let res_refs = extract_all_resource_references(line);
    for (res_type, res_name) in res_refs {
      links.push(ParsedAndroidLink {
        source: layout_qname.clone(),
        target: format!("R.{}.{}", res_type, res_name),
        kind: "ConfiguredBy".to_string(),
      });
    }
  }

  (resources, links)
}

/// Parse Android Navigation XML
pub fn parse_navigation(content: &str) -> (Vec<ParsedAndroidResource>, Vec<ParsedAndroidLink>) {
  let mut resources = Vec::new();
  let mut links = Vec::new();

  let content_clean = strip_comments(content);
  let dest_tags = ["fragment", "activity", "dialog", "navigation"];
  for tag in &dest_tags {
    let open_tag = format!("<{}", tag);
    let mut cur_idx = 0;
    while let Some(start_offset) = content_clean[cur_idx..].find(&open_tag) {
      let start_idx = cur_idx + start_offset;
      let line = index_to_line(&content_clean, start_idx);

      let remaining = &content_clean[start_idx..];
      if let Some(tag_end) = remaining.find('>') {
        let tag_str = &remaining[..=tag_end];
        cur_idx = start_idx + tag_end + 1;

        let id_val = extract_attribute(tag_str, "id");
        let name_val = extract_attribute(tag_str, "name");
        let layout_val = extract_attribute(tag_str, "layout");

        if let Some(id_raw) = id_val {
          let id_name = id_raw.replace("@+id/", "").replace("@id/", "");
          let dest_qname = format!("R.id.{}", id_name);

          resources.push(ParsedAndroidResource {
            name: id_name,
            resource_type: "id".to_string(),
            value: None,
            start_line: line,
            end_line: line,
          });

          if let Some(class_name) = name_val {
            links.push(ParsedAndroidLink {
              source: dest_qname.clone(),
              target: class_name,
              kind: "ConfiguredBy".to_string(),
            });
          }

          if let Some(layout_ref) = layout_val {
            let layout_name = layout_ref.replace("@layout/", "");
            links.push(ParsedAndroidLink {
              source: dest_qname.clone(),
              target: format!("R.layout.{}", layout_name),
              kind: "ConfiguredBy".to_string(),
            });
          }
        }
      } else {
        break;
      }
    }
  }

  (resources, links)
}

/// Parse Android Values XML (strings.xml, colors.xml, dimens.xml)
pub fn parse_values(content: &str) -> Vec<ParsedAndroidResource> {
  let mut resources = Vec::new();
  let content_clean = strip_comments(content);
  let tag_types = ["string", "color", "dimen"];

  for tag in &tag_types {
    let open_tag = format!("<{}", tag);
    let close_tag = format!("</{}>", tag);
    let mut cur_idx = 0;

    while let Some(start_offset) = content_clean[cur_idx..].find(&open_tag) {
      let start_idx = cur_idx + start_offset;
      let remaining = &content_clean[start_idx..];

      if let Some(open_tag_end) = remaining.find('>') {
        let open_tag_str = &remaining[..=open_tag_end];
        let name_opt = extract_attribute(open_tag_str, "name");
        let close_tag_idx_opt = remaining.find(&close_tag);

        if let (Some(name), Some(close_tag_idx)) = (name_opt, close_tag_idx_opt) {
          let val_start = open_tag_end + 1;
          let val = &remaining[val_start..close_tag_idx];

          let start_line = index_to_line(&content_clean, start_idx);
          let end_line = index_to_line(&content_clean, start_idx + close_tag_idx + close_tag.len());

          resources.push(ParsedAndroidResource {
            name,
            resource_type: tag.to_string(),
            value: Some(val.to_string()),
            start_line,
            end_line,
          });

          cur_idx = start_idx + close_tag_idx + close_tag.len();
          continue;
        }
        cur_idx = start_idx + open_tag_end + 1;
      } else {
        break;
      }
    }
  }

  resources
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_manifest() {
    let manifest = r#"
      <manifest xmlns:android="http://schemas.android.com/apk/res/android"
          package="com.example.app">
          <uses-permission android:name="android.permission.INTERNET" />
          <application
              android:name=".MyApplication"
              android:theme="@style/Theme.MyApp">
              <activity
                  android:name=".MainActivity"
                  android:exported="true">
                  <intent-filter>
                      <action android:name="android.intent.action.MAIN" />
                      <category android:name="android.intent.category.LAUNCHER" />
                  </intent-filter>
              </activity>
              <service android:name="com.example.app.MyService" android:permission="android.permission.BIND_JOB_SERVICE" />
          </application>
      </manifest>
    "#;

    let (components, permissions) = parse_manifest(manifest);
    assert_eq!(permissions.len(), 1);
    assert_eq!(permissions[0], "android.permission.INTERNET");

    assert_eq!(components.len(), 3); // application, activity, service

    let app = components
      .iter()
      .find(|c| c.component_type == "application")
      .unwrap();
    assert_eq!(app.name, "MyApplication");
    assert_eq!(app.class_name, "com.example.app.MyApplication");

    let act = components
      .iter()
      .find(|c| c.component_type == "activity")
      .unwrap();
    assert_eq!(act.name, "MainActivity");
    assert_eq!(act.class_name, "com.example.app.MainActivity");
    assert_eq!(act.intent_actions[0], "android.intent.action.MAIN");
    assert_eq!(act.intent_categories[0], "android.intent.category.LAUNCHER");

    let svc = components
      .iter()
      .find(|c| c.component_type == "service")
      .unwrap();
    assert_eq!(svc.name, "MyService");
    assert_eq!(svc.class_name, "com.example.app.MyService");
    assert_eq!(
      svc.permission.as_deref(),
      Some("android.permission.BIND_JOB_SERVICE")
    );
  }

  #[test]
  fn test_parse_layout() {
    let layout = r#"
      <LinearLayout xmlns:android="http://schemas.android.com/apk/res/android"
          android:layout_width="match_parent"
          android:layout_height="match_parent">
          <TextView
              android:id="@+id/text_title"
              android:layout_width="wrap_content"
              android:layout_height="wrap_content"
              android:text="@string/welcome_message" />
          <Button
              android:id="@id/btn_submit"
              android:layout_width="wrap_content"
              android:layout_height="wrap_content" />
          <include layout="@layout/common_footer" />
      </LinearLayout>
    "#;

    let (res, links) = parse_layout(layout, "app/src/main/res/layout/activity_main.xml");
    // activity_main (layout), text_title (id), btn_submit (id)
    assert_eq!(res.len(), 3);
    assert_eq!(res[0].name, "activity_main");
    assert_eq!(res[0].resource_type, "layout");
    assert_eq!(res[1].name, "text_title");
    assert_eq!(res[2].name, "btn_submit");

    // links: activity_main -> text_title, activity_main -> btn_submit, activity_main -> welcome_message, activity_main -> common_footer
    assert!(
      links
        .iter()
        .any(|l| l.source == "R.layout.activity_main" && l.target == "R.id.text_title")
    );
    assert!(
      links
        .iter()
        .any(|l| l.source == "R.layout.activity_main" && l.target == "R.string.welcome_message")
    );
    assert!(
      links
        .iter()
        .any(|l| l.source == "R.layout.activity_main" && l.target == "R.layout.common_footer")
    );
  }

  #[test]
  fn test_parse_navigation() {
    let nav = r#"
      <navigation xmlns:android="http://schemas.android.com/apk/res/android"
          xmlns:app="http://schemas.android.com/apk/res-auto"
          xmlns:tools="http://schemas.android.com/tools"
          android:id="@+id/nav_graph">
          <fragment
              android:id="@+id/navigation_home"
              android:name="com.example.app.ui.home.HomeFragment"
              tools:layout="@layout/fragment_home" />
      </navigation>
    "#;

    let (res, links) = parse_navigation(nav);
    assert!(res.iter().any(|r| r.name == "nav_graph"));
    assert!(res.iter().any(|r| r.name == "navigation_home"));

    assert!(
      links.iter().any(|l| l.source == "R.id.navigation_home"
        && l.target == "com.example.app.ui.home.HomeFragment")
    );
    assert!(
      links
        .iter()
        .any(|l| l.source == "R.id.navigation_home" && l.target == "R.layout.fragment_home")
    );
  }

  #[test]
  fn test_parse_values() {
    let values = r#"
      <resources>
          <string name="app_name">My App</string>
          <color name="purple_200">#FFBB86FC</color>
          <dimen name="margin_medium">16dp</dimen>
      </resources>
    "#;

    let res = parse_values(values);
    assert_eq!(res.len(), 3);
    assert_eq!(res[0].name, "app_name");
    assert_eq!(res[0].value.as_deref(), Some("My App"));
    assert_eq!(res[1].name, "purple_200");
    assert_eq!(res[2].name, "margin_medium");
  }

  #[test]
  fn test_parse_values_multiline_and_html() {
    let values = r#"
      <resources>
          <string name="multiline_str">
              Line 1
              Line 2
          </string>
          <string name="html_str">Welcome <b>User</b>!</string>
      </resources>
    "#;
    let res = parse_values(values);
    assert_eq!(res.len(), 2);
    assert_eq!(res[0].name, "multiline_str");
    assert!(res[0].value.as_ref().unwrap().contains("Line 1"));
    assert!(res[0].value.as_ref().unwrap().contains("Line 2"));
    assert_eq!(res[1].name, "html_str");
    assert_eq!(res[1].value.as_deref(), Some("Welcome <b>User</b>!"));
  }

  #[test]
  fn test_parse_with_comments() {
    let manifest = r#"
      <manifest xmlns:android="http://schemas.android.com/apk/res/android"
          package="com.example.app">
          <!-- <uses-permission android:name="android.permission.SEND_SMS" /> -->
          <application>
              <activity android:name=".MainActivity">
                  <intent-filter>
                      <action android:name="android.intent.action.MAIN" />
                  </intent-filter>
              </activity>
              <!--
              <service android:name=".MyService" />
              -->
          </application>
      </manifest>
    "#;
    let (components, permissions) = parse_manifest(manifest);
    assert_eq!(permissions.len(), 0);
    assert!(
      components
        .iter()
        .any(|c| c.component_type == "activity" && c.name == "MainActivity")
    );
    assert!(!components.iter().any(|c| c.component_type == "service"));
  }

  #[test]
  fn test_utf8_slicing_safety() {
    let manifest = r#"
      <manifest xmlns:android="http://schemas.android.com/apk/res/android"
          package="com.example.app">
          <!-- 📥 Emoji comment containing multi-byte characters -->
          <application android:label="한글앱">
              <activity android:name=".MainActivity" />
          </application>
      </manifest>
    "#;
    // This should not panic
    let (components, _) = parse_manifest(manifest);
    assert!(!components.is_empty());
  }

  #[test]
  fn test_overlapping_manifest_tags() {
    let manifest = r#"
      <manifest xmlns:android="http://schemas.android.com/apk/res/android"
          package="com.example.app">
          <application>
              <activity android:name=".MainActivity" />
              <activity-alias android:name=".AliasActivity" android:targetActivity=".MainActivity" />
          </application>
      </manifest>
    "#;
    let (components, _) = parse_manifest(manifest);
    assert!(
      components
        .iter()
        .any(|c| c.component_type == "activity" && c.name == "MainActivity")
    );
    assert!(
      !components
        .iter()
        .any(|c| c.component_type == "activity" && c.name == "AliasActivity")
    );
  }
}
