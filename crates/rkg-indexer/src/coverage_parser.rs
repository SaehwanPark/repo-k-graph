use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileCoverage {
  pub file_path: String,
  pub line_hits: HashMap<usize, usize>,
  pub line_branches: HashMap<usize, (usize, usize)>, // line -> (covered, total)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportCoverage {
  pub test_suite: Option<String>,
  pub file_coverages: Vec<FileCoverage>,
}

pub fn parse_coverage_report(content: &str) -> Result<ReportCoverage, String> {
  let trimmed = content.trim_start();
  if trimmed.starts_with('<') || trimmed.contains("<?xml") || trimmed.contains("<coverage") {
    parse_cobertura_xml(content)
  } else {
    parse_lcov(content)
  }
}

#[allow(clippy::collapsible_if)]
pub fn parse_lcov(content: &str) -> Result<ReportCoverage, String> {
  let mut test_suite = None;
  let mut file_coverages = Vec::new();
  let mut current_file = None;
  let mut line_hits = HashMap::new();
  let mut line_branches = HashMap::new();

  for line in content.lines() {
    let line = line.trim();
    if line.is_empty() {
      continue;
    }

    if let Some(tn) = line.strip_prefix("TN:") {
      let name = tn.trim().to_string();
      if !name.is_empty() {
        test_suite = Some(name);
      }
    } else if let Some(sf) = line.strip_prefix("SF:") {
      current_file = Some(sf.trim().to_string());
      line_hits.clear();
      line_branches.clear();
    } else if let Some(da) = line.strip_prefix("DA:") {
      // Format: DA:<line_number>,<hits>[,<checksum>]
      let parts: Vec<&str> = da.split(',').collect();
      if parts.len() >= 2 {
        if let (Ok(ln), Ok(hits)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
          line_hits.insert(ln, hits);
        }
      }
    } else if let Some(brda) = line.strip_prefix("BRDA:") {
      // Format: BRDA:<line_number>,<block>,<branch>,<taken>
      let parts: Vec<&str> = brda.split(',').collect();
      if parts.len() >= 4 {
        if let Ok(ln) = parts[0].parse::<usize>() {
          let taken = parts[3].trim();
          let covered = if taken != "-" && taken != "0" { 1 } else { 0 };
          let entry = line_branches.entry(ln).or_insert((0, 0));
          entry.0 += covered;
          entry.1 += 1;
        }
      }
    } else if line == "end_of_record" {
      if let Some(file_path) = current_file.take() {
        file_coverages.push(FileCoverage {
          file_path,
          line_hits: line_hits.clone(),
          line_branches: line_branches.clone(),
        });
      }
    }
  }

  Ok(ReportCoverage {
    test_suite,
    file_coverages,
  })
}

#[allow(clippy::collapsible_if)]
pub fn parse_cobertura_xml(content: &str) -> Result<ReportCoverage, String> {
  let mut file_coverages = Vec::new();

  let extract_attr = |tag: &str, attr: &str| -> Option<String> {
    let search = format!("{}=\"", attr);
    if let Some(start_idx) = tag.find(&search) {
      let val_start = start_idx + search.len();
      if let Some(end_idx) = tag[val_start..].find('"') {
        return Some(tag[val_start..val_start + end_idx].to_string());
      }
    }
    None
  };

  let mut current_file = None;
  let mut line_hits = HashMap::new();
  let mut line_branches = HashMap::new();
  let mut pos = 0;

  while let Some(start_idx) = content[pos..].find('<') {
    let abs_start = pos + start_idx;
    if let Some(end_idx) = content[abs_start..].find('>') {
      let abs_end = abs_start + end_idx;
      let tag = &content[abs_start..=abs_end];
      pos = abs_end + 1;

      if tag.starts_with("<class") {
        if let Some(file_path) = current_file.take() {
          file_coverages.push(FileCoverage {
            file_path,
            line_hits: line_hits.clone(),
            line_branches: line_branches.clone(),
          });
        }
        line_hits.clear();
        line_branches.clear();

        if let Some(filename) = extract_attr(tag, "filename") {
          current_file = Some(filename);
        }
      } else if tag.starts_with("<line") {
        if let (Some(num_str), Some(hits_str)) =
          (extract_attr(tag, "number"), extract_attr(tag, "hits"))
        {
          if let (Ok(num), Ok(hits)) = (num_str.parse::<usize>(), hits_str.parse::<usize>()) {
            line_hits.insert(num, hits);

            if let Some(branch_str) = extract_attr(tag, "branch") {
              if branch_str == "true" {
                if let Some(cc) = extract_attr(tag, "condition-coverage") {
                  if let Some(paren_start) = cc.find('(') {
                    if let Some(paren_end) = cc[paren_start..].find(')') {
                      let ratio_str = &cc[paren_start + 1..paren_start + paren_end];
                      let parts: Vec<&str> = ratio_str.split('/').collect();
                      if parts.len() == 2 {
                        if let (Ok(cov), Ok(tot)) =
                          (parts[0].parse::<usize>(), parts[1].parse::<usize>())
                        {
                          line_branches.insert(num, (cov, tot));
                        }
                      }
                    }
                  }
                } else {
                  let cov = if hits > 0 { 1 } else { 0 };
                  line_branches.insert(num, (cov, 1));
                }
              }
            }
          }
        }
      } else if tag.starts_with("</class>") {
        if let Some(file_path) = current_file.take() {
          file_coverages.push(FileCoverage {
            file_path,
            line_hits: line_hits.clone(),
            line_branches: line_branches.clone(),
          });
        }
        line_hits.clear();
        line_branches.clear();
      }
    } else {
      break;
    }
  }

  if let Some(file_path) = current_file {
    file_coverages.push(FileCoverage {
      file_path,
      line_hits,
      line_branches,
    });
  }

  Ok(ReportCoverage {
    test_suite: None,
    file_coverages,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_lcov_parsing() {
    let content = r#"
TN:my_test_run
SF:src/foo.rs
DA:10,5
DA:11,0
BRDA:12,0,0,1
BRDA:12,0,1,0
end_of_record
SF:src/bar.rs
DA:20,0
end_of_record
"#;
    let report = parse_coverage_report(content).unwrap();
    assert_eq!(report.test_suite, Some("my_test_run".to_string()));
    assert_eq!(report.file_coverages.len(), 2);

    let foo = &report.file_coverages[0];
    assert_eq!(foo.file_path, "src/foo.rs");
    assert_eq!(foo.line_hits.get(&10), Some(&5));
    assert_eq!(foo.line_hits.get(&11), Some(&0));
    assert_eq!(foo.line_branches.get(&12), Some(&(1, 2)));

    let bar = &report.file_coverages[1];
    assert_eq!(bar.file_path, "src/bar.rs");
    assert_eq!(bar.line_hits.get(&20), Some(&0));
  }

  #[test]
  fn test_cobertura_xml_parsing() {
    let content = r#"
<?xml version="1.0" ?>
<coverage line-rate="0.5" branch-rate="0.5" version="1.9">
  <packages>
    <package name="src">
      <classes>
        <class name="foo" filename="src/foo.rs" line-rate="0.5" branch-rate="0.5">
          <methods/>
          <lines>
            <line number="10" hits="5" branch="false"/>
            <line number="11" hits="0" branch="false"/>
            <line number="12" hits="1" branch="true" condition-coverage="50% (1/2)"/>
          </lines>
        </class>
      </classes>
    </package>
  </packages>
</coverage>
"#;
    let report = parse_coverage_report(content).unwrap();
    assert_eq!(report.test_suite, None);
    assert_eq!(report.file_coverages.len(), 1);

    let foo = &report.file_coverages[0];
    assert_eq!(foo.file_path, "src/foo.rs");
    assert_eq!(foo.line_hits.get(&10), Some(&5));
    assert_eq!(foo.line_hits.get(&11), Some(&0));
    assert_eq!(foo.line_branches.get(&12), Some(&(1, 2)));
  }
}
