use std::ffi::OsStr;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use rkg_lang_python::parse_python_source;
use sha2::{Digest, Sha256};

pub mod android_parser;
pub mod cargo_parser;
pub mod coverage_parser;
pub mod fsharp_parser;
pub mod kotlin_project_parser;
pub mod swift_project_parser;

pub const CRATE_NAME: &str = "rkg-indexer";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredFile {
  pub path: String,
  pub language: Option<String>,
  pub content_hash: String,
  pub line_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingFileSnapshot {
  pub path: String,
  pub content_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncrementalDiff {
  pub changed: Vec<DiscoveredFile>,
  pub unchanged: Vec<DiscoveredFile>,
  pub deleted: Vec<ExistingFileSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonParseIssue {
  pub path: String,
  pub message: String,
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonParseSummary {
  pub files_parsed: usize,
  pub files_with_syntax_errors: usize,
  pub syntax_error_count: usize,
  pub internal_error_count: usize,
  pub issues: Vec<PythonParseIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexerError {
  RepoRootNotFound { start: PathBuf },
  WalkError(String),
  Io(String),
}

impl Display for IndexerError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      IndexerError::RepoRootNotFound { start } => {
        write!(
          f,
          "failed to detect repository root from {}: no .git marker found",
          start.display()
        )
      }
      IndexerError::WalkError(message) => write!(f, "file walk failed: {message}"),
      IndexerError::Io(message) => write!(f, "{message}"),
    }
  }
}

impl std::error::Error for IndexerError {}

pub fn detect_repo_root(start: impl AsRef<Path>) -> Result<PathBuf, IndexerError> {
  let start = start.as_ref();
  let start_path = if start.is_absolute() {
    start.to_path_buf()
  } else {
    std::env::current_dir()
      .map_err(|e| IndexerError::Io(e.to_string()))?
      .join(start)
  };
  let canonical_start =
    fs::canonicalize(&start_path).map_err(|e| IndexerError::Io(e.to_string()))?;

  for candidate in canonical_start.ancestors() {
    if candidate.join(".git").exists() {
      return Ok(candidate.to_path_buf());
    }
  }

  Err(IndexerError::RepoRootNotFound {
    start: canonical_start,
  })
}

pub fn discover_files(repo_root: impl AsRef<Path>) -> Result<Vec<DiscoveredFile>, IndexerError> {
  let repo_root = repo_root.as_ref();
  let root = fs::canonicalize(repo_root).map_err(|e| IndexerError::Io(e.to_string()))?;
  let filter_root = root.clone();

  let walker = WalkBuilder::new(&root)
    .standard_filters(true)
    .git_ignore(true)
    .git_global(true)
    .git_exclude(true)
    .hidden(false)
    .add_custom_ignore_filename(".rkgignore")
    .filter_entry(move |entry| {
      let relative = entry
        .path()
        .strip_prefix(&filter_root)
        .unwrap_or_else(|_| entry.path());
      !is_internal_path(relative)
    })
    .build();

  let mut discovered = Vec::new();
  for entry in walker {
    let entry = entry.map_err(|e| IndexerError::WalkError(e.to_string()))?;
    let path = entry.path();
    let relative_path = path
      .strip_prefix(&root)
      .map_err(|e| IndexerError::Io(e.to_string()))?;
    if is_internal_path(relative_path) {
      continue;
    }
    if !entry
      .file_type()
      .is_some_and(|file_type| file_type.is_file())
    {
      continue;
    }

    let relative_path = relative_path.to_string_lossy().replace('\\', "/");
    let bytes = fs::read(path).map_err(|e| IndexerError::Io(e.to_string()))?;
    let content_hash = hex_sha256(&bytes);
    let line_count = count_lines(&bytes);
    let language = detect_language(path);

    discovered.push(DiscoveredFile {
      path: relative_path,
      language,
      content_hash,
      line_count,
    });
  }

  discovered.sort_by(|a, b| a.path.cmp(&b.path));
  Ok(discovered)
}

pub fn classify_incremental_diff(
  discovered_files: &[DiscoveredFile],
  existing_files: &[ExistingFileSnapshot],
) -> IncrementalDiff {
  let existing_by_path: std::collections::HashMap<&str, &ExistingFileSnapshot> = existing_files
    .iter()
    .map(|file| (file.path.as_str(), file))
    .collect();
  let discovered_paths: std::collections::HashSet<&str> = discovered_files
    .iter()
    .map(|file| file.path.as_str())
    .collect();

  let mut changed = Vec::new();
  let mut unchanged = Vec::new();
  for file in discovered_files {
    match existing_by_path.get(file.path.as_str()) {
      None => changed.push(file.clone()),
      Some(existing) => {
        if existing.content_hash.as_deref() == Some(file.content_hash.as_str()) {
          unchanged.push(file.clone());
        } else {
          changed.push(file.clone());
        }
      }
    }
  }

  let mut deleted: Vec<ExistingFileSnapshot> = existing_files
    .iter()
    .filter(|file| !discovered_paths.contains(file.path.as_str()))
    .cloned()
    .collect();

  changed.sort_by(|a, b| a.path.cmp(&b.path));
  unchanged.sort_by(|a, b| a.path.cmp(&b.path));
  deleted.sort_by(|a, b| a.path.cmp(&b.path));

  IncrementalDiff {
    changed,
    unchanged,
    deleted,
  }
}

pub fn analyze_python_parse(
  repo_root: impl AsRef<Path>,
  files: &[DiscoveredFile],
) -> Result<PythonParseSummary, IndexerError> {
  let root = repo_root.as_ref();
  let mut files_parsed = 0usize;
  let mut files_with_syntax_errors = 0usize;
  let mut syntax_error_count = 0usize;
  let mut internal_error_count = 0usize;
  let mut issues = Vec::new();

  for file in files {
    if file.language.as_deref() != Some("python") {
      continue;
    }

    files_parsed += 1;
    let file_path = root.join(&file.path);
    let source_bytes = fs::read(&file_path)
      .map_err(|e| IndexerError::Io(format!("failed to read {}: {e}", file_path.display())))?;
    let source = match String::from_utf8(source_bytes) {
      Ok(source) => source,
      Err(error) => {
        internal_error_count += 1;
        issues.push(PythonParseIssue {
          path: file.path.clone(),
          message: format!("non-utf8 python source: {error}"),
          start_line: 0,
          start_column: 0,
          end_line: 0,
          end_column: 0,
        });
        continue;
      }
    };

    match parse_python_source(&source) {
      Ok(report) => {
        if !report.syntax_errors.is_empty() {
          files_with_syntax_errors += 1;
          syntax_error_count += report.syntax_errors.len();
          for parse_error in report.syntax_errors {
            issues.push(PythonParseIssue {
              path: file.path.clone(),
              message: parse_error.message,
              start_line: parse_error.start_line,
              start_column: parse_error.start_column,
              end_line: parse_error.end_line,
              end_column: parse_error.end_column,
            });
          }
        }
      }
      Err(error) => {
        internal_error_count += 1;
        issues.push(PythonParseIssue {
          path: file.path.clone(),
          message: format!("internal parse error: {error}"),
          start_line: 0,
          start_column: 0,
          end_line: 0,
          end_column: 0,
        });
      }
    }
  }

  issues.sort_by(|a, b| {
    a.path
      .cmp(&b.path)
      .then(a.start_line.cmp(&b.start_line))
      .then(a.start_column.cmp(&b.start_column))
      .then(a.message.cmp(&b.message))
  });

  Ok(PythonParseSummary {
    files_parsed,
    files_with_syntax_errors,
    syntax_error_count,
    internal_error_count,
    issues,
  })
}

fn hex_sha256(bytes: &[u8]) -> String {
  let mut hasher = Sha256::new();
  hasher.update(bytes);
  let digest = hasher.finalize();
  format!("{digest:x}")
}

fn count_lines(bytes: &[u8]) -> usize {
  if bytes.is_empty() {
    return 0;
  }
  let newline_count = bytes.iter().filter(|byte| **byte == b'\n').count();
  if bytes.last() == Some(&b'\n') {
    newline_count
  } else {
    newline_count + 1
  }
}

fn detect_language(path: &Path) -> Option<String> {
  match path
    .extension()
    .and_then(OsStr::to_str)
    .map(|s| s.to_ascii_lowercase())
    .as_deref()
  {
    Some("py") => Some("python".to_string()),
    Some("rs") => Some("rust".to_string()),
    Some("fs") | Some("fsi") | Some("fsx") => Some("fsharp".to_string()),
    Some("mojo") | Some("🔥") => Some("mojo".to_string()),
    Some("kt") | Some("kts") => Some("kotlin".to_string()),
    Some("swift") => Some("swift".to_string()),
    Some("xml") => Some("xml".to_string()),
    Some("md") => Some("markdown".to_string()),
    Some("toml") => Some("toml".to_string()),
    Some("json") => Some("json".to_string()),
    Some("yml") | Some("yaml") => Some("yaml".to_string()),
    Some("sh") => Some("shell".to_string()),
    Some("ipynb") => Some("jupyter".to_string()),
    _ => None,
  }
}

fn is_internal_path(relative_path: &Path) -> bool {
  relative_path
    .components()
    .next()
    .and_then(|component| component.as_os_str().to_str())
    .is_some_and(|segment| segment == ".git" || segment == ".rkg")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolLineRange {
  pub qualified_name: String,
  pub start_line: usize,
  pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredCommit {
  pub commit: rkg_core::GitCommitInfo,
  pub modified_files: Vec<String>,
}

fn run_git_command(repo_root: &Path, args: &[&str]) -> Result<String, IndexerError> {
  let output = std::process::Command::new("git")
    .args(args)
    .current_dir(repo_root)
    .output()
    .map_err(|e| IndexerError::Io(format!("failed to execute git command: {e}")))?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    return Err(IndexerError::Io(format!("git command failed: {stderr}")));
  }

  Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn extract_git_history(
  repo_root: &Path,
  limit: usize,
) -> Result<Vec<DiscoveredCommit>, IndexerError> {
  let format_arg = "--pretty=format:::RKG_COMMIT::%H|%an|%ae|%ai|%s";
  let limit_str = limit.to_string();
  let args = if limit > 0 {
    vec!["log", format_arg, "--name-only", "-n", &limit_str]
  } else {
    vec!["log", format_arg, "--name-only"]
  };

  let output = run_git_command(repo_root, &args)?;
  let mut commits = Vec::new();

  for block in output.split("::RKG_COMMIT::") {
    let block = block.trim();
    if block.is_empty() {
      continue;
    }

    let mut lines = block.lines();
    let Some(meta_line) = lines.next() else {
      continue;
    };

    let parts: Vec<&str> = meta_line.split('|').collect();
    if parts.len() < 5 {
      continue;
    }

    let hash = parts[0].to_string();
    let author_name = parts[1].to_string();
    let author_email = parts[2].to_string();
    let date = parts[3].to_string();
    let message = parts[4..].join("|");

    let mut modified_files = Vec::new();
    for line in lines {
      let line = line.trim();
      if !line.is_empty() {
        modified_files.push(line.to_string());
      }
    }

    commits.push(DiscoveredCommit {
      commit: rkg_core::GitCommitInfo {
        hash,
        author_name,
        author_email,
        date,
        message,
      },
      modified_files,
    });
  }

  Ok(commits)
}

pub fn extract_symbol_changes_in_commit(
  repo_root: &Path,
  commit_hash: &str,
  file_path: &str,
  symbols: &[SymbolLineRange],
) -> Result<Vec<String>, IndexerError> {
  if symbols.is_empty() {
    return Ok(Vec::new());
  }

  let args = &["show", "--format=", commit_hash, "--", file_path];
  let diff_output = match run_git_command(repo_root, args) {
    Ok(out) => out,
    Err(_) => {
      return Ok(Vec::new());
    }
  };

  let mut changed_symbols = std::collections::HashSet::new();
  let mut current_line = 0;

  for line in diff_output.lines() {
    if line.starts_with("@@ ") {
      if let Some(plus_idx) = line.find(" +") {
        let rest = &line[plus_idx + 2..];
        let end_of_num = rest
          .find(',')
          .or_else(|| rest.find(' '))
          .unwrap_or(rest.len());
        if let Ok(l) = rest[..end_of_num].parse::<usize>() {
          current_line = l;
        }
      }
      continue;
    }

    if line.starts_with("+++ ") || line.starts_with("--- ") {
      continue;
    }

    let is_added = line.starts_with('+');
    let is_deleted = line.starts_with('-');

    if is_added || is_deleted {
      for sym in symbols {
        if current_line >= sym.start_line && current_line <= sym.end_line {
          changed_symbols.insert(sym.qualified_name.clone());
        }
      }
    }

    if !is_deleted {
      current_line += 1;
    }
  }

  let mut result: Vec<String> = changed_symbols.into_iter().collect();
  result.sort();
  Ok(result)
}
