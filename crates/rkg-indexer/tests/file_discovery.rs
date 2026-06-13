use std::fs;
use std::path::Path;

use rkg_indexer::{
  ExistingFileSnapshot, analyze_python_parse, classify_incremental_diff, detect_repo_root,
  discover_files,
};
use tempfile::TempDir;

#[test]
fn detects_repo_root_from_nested_path() {
  let temp_dir = TempDir::new().expect("temp dir should be created");
  let root = temp_dir.path();
  fs::create_dir(root.join(".git")).expect(".git should be created");
  fs::create_dir_all(root.join("src/nested")).expect("nested dir should be created");

  let detected = detect_repo_root(root.join("src/nested")).expect("repo root should be detected");
  assert_eq!(
    detected,
    fs::canonicalize(root).expect("canonical path should resolve")
  );
}

#[test]
fn detects_repo_root_when_dotgit_is_a_file() {
  let temp_dir = TempDir::new().expect("temp dir should be created");
  let root = temp_dir.path();
  fs::write(root.join(".git"), "gitdir: /tmp/worktrees/repo/.git\n")
    .expect(".git file should be created");
  fs::create_dir_all(root.join("src/nested")).expect("nested dir should be created");

  let detected = detect_repo_root(root.join("src/nested")).expect("repo root should be detected");
  assert_eq!(
    detected,
    fs::canonicalize(root).expect("canonical path should resolve")
  );
}

#[test]
fn returns_error_when_repo_root_missing() {
  let temp_dir = TempDir::new().expect("temp dir should be created");
  let result = detect_repo_root(temp_dir.path());
  assert!(result.is_err());
}

#[test]
fn honors_gitignore_and_rkgignore_rules() {
  let temp_dir = TempDir::new().expect("temp dir should be created");
  let root = temp_dir.path();
  fs::create_dir(root.join(".git")).expect(".git should be created");
  fs::write(root.join(".gitignore"), "ignored.py\n").expect(".gitignore should be written");
  fs::write(root.join(".rkgignore"), "*.cache\n").expect(".rkgignore should be written");

  write_file(root, "included.py", "print('ok')\n");
  write_file(root, "ignored.py", "print('ignored')\n");
  write_file(root, "secret.cache", "cache\n");

  let files = discover_files(root).expect("file discovery should succeed");
  let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
  assert!(paths.contains(&"included.py"));
  assert!(!paths.contains(&"ignored.py"));
  assert!(!paths.contains(&"secret.cache"));
}

#[test]
fn excludes_git_and_rkg_internal_paths_but_keeps_other_dot_dirs() {
  let temp_dir = TempDir::new().expect("temp dir should be created");
  let root = temp_dir.path();
  fs::create_dir(root.join(".git")).expect(".git dir should be created");

  write_file(root, "src/included.py", "print('ok')\n");
  write_file(root, ".rkg/rkg.db", "internal-db\n");
  write_file(root, ".git/HEAD", "ref: refs/heads/main\n");
  write_file(root, ".github/workflows/ci.yml", "name: CI\n");

  let files = discover_files(root).expect("file discovery should succeed");
  let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
  assert!(paths.contains(&"src/included.py"));
  assert!(paths.contains(&".github/workflows/ci.yml"));
  assert!(!paths.contains(&".rkg/rkg.db"));
  assert!(!paths.contains(&".git/HEAD"));
}

#[test]
fn detects_known_and_unknown_languages() {
  let temp_dir = TempDir::new().expect("temp dir should be created");
  let root = temp_dir.path();
  fs::create_dir(root.join(".git")).expect(".git should be created");

  write_file(root, "src/module.py", "def f():\n  return 1\n");
  write_file(root, "notes.txt", "text\n");

  let files = discover_files(root).expect("file discovery should succeed");
  let py = files
    .iter()
    .find(|file| file.path == "src/module.py")
    .expect("python file should be present");
  assert_eq!(py.language.as_deref(), Some("python"));

  let txt = files
    .iter()
    .find(|file| file.path == "notes.txt")
    .expect("text file should be present");
  assert_eq!(txt.language, None);
}

#[test]
fn hash_changes_when_content_changes() {
  let temp_dir = TempDir::new().expect("temp dir should be created");
  let root = temp_dir.path();
  fs::create_dir(root.join(".git")).expect(".git should be created");

  write_file(root, "src/value.py", "x = 1\n");
  let before = discover_files(root).expect("first discovery should succeed");
  let before_hash = before
    .iter()
    .find(|file| file.path == "src/value.py")
    .expect("first file should exist")
    .content_hash
    .clone();

  write_file(root, "src/value.py", "x = 2\n");
  let after = discover_files(root).expect("second discovery should succeed");
  let after_hash = after
    .iter()
    .find(|file| file.path == "src/value.py")
    .expect("second file should exist")
    .content_hash
    .clone();

  assert_ne!(before_hash, after_hash);
}

#[test]
fn returns_deterministically_sorted_paths_and_line_counts() {
  let temp_dir = TempDir::new().expect("temp dir should be created");
  let root = temp_dir.path();
  fs::create_dir(root.join(".git")).expect(".git should be created");

  write_file(root, "b.py", "print('b')\n");
  write_file(root, "a.py", "print('a')\nline2\n");

  let files_first = discover_files(root).expect("first discovery should succeed");
  let files_second = discover_files(root).expect("second discovery should succeed");

  let paths_first: Vec<&str> = files_first.iter().map(|file| file.path.as_str()).collect();
  assert_eq!(paths_first, vec!["a.py", "b.py"]);

  let a = files_first
    .iter()
    .find(|file| file.path == "a.py")
    .expect("a.py should exist");
  assert_eq!(a.line_count, 2);
  assert_eq!(files_first, files_second);
}

fn write_file(root: &Path, relative_path: &str, content: &str) {
  let absolute_path = root.join(relative_path);
  if let Some(parent) = absolute_path.parent() {
    fs::create_dir_all(parent).expect("parent directories should be created");
  }
  fs::write(absolute_path, content).expect("fixture file should be written");
}

#[test]
fn incremental_diff_marks_all_new_files_as_changed() {
  let discovered = vec![
    discovered_file("a.py", "hash-a"),
    discovered_file("b.py", "hash-b"),
  ];

  let diff = classify_incremental_diff(&discovered, &[]);
  assert_eq!(diff.changed, discovered);
  assert!(diff.unchanged.is_empty());
  assert!(diff.deleted.is_empty());
}

#[test]
fn incremental_diff_detects_changed_unchanged_and_deleted_files() {
  let discovered = vec![
    discovered_file("a.py", "hash-a"),
    discovered_file("b.py", "hash-b-new"),
    discovered_file("d.py", "hash-d"),
  ];
  let existing = vec![
    existing_file("a.py", Some("hash-a")),
    existing_file("b.py", Some("hash-b-old")),
    existing_file("c.py", Some("hash-c")),
  ];

  let diff = classify_incremental_diff(&discovered, &existing);
  let changed_paths: Vec<&str> = diff.changed.iter().map(|f| f.path.as_str()).collect();
  let unchanged_paths: Vec<&str> = diff.unchanged.iter().map(|f| f.path.as_str()).collect();
  let deleted_paths: Vec<&str> = diff.deleted.iter().map(|f| f.path.as_str()).collect();

  assert_eq!(changed_paths, vec!["b.py", "d.py"]);
  assert_eq!(unchanged_paths, vec!["a.py"]);
  assert_eq!(deleted_paths, vec!["c.py"]);
}

fn discovered_file(path: &str, hash: &str) -> rkg_indexer::DiscoveredFile {
  rkg_indexer::DiscoveredFile {
    path: path.to_string(),
    language: Some("python".to_string()),
    content_hash: hash.to_string(),
    line_count: 1,
  }
}

fn existing_file(path: &str, hash: Option<&str>) -> ExistingFileSnapshot {
  ExistingFileSnapshot {
    path: path.to_string(),
    content_hash: hash.map(str::to_string),
  }
}

#[test]
fn analyze_python_parse_reports_syntax_errors_deterministically() {
  let temp_dir = TempDir::new().expect("temp dir should be created");
  let root = temp_dir.path();
  fs::create_dir(root.join(".git")).expect(".git should be created");

  write_file(root, "src/good.py", "def ok():\n  return 1\n");
  write_file(root, "src/bad.py", "def broken(:\n  return 1\n");

  let discovered = discover_files(root).expect("file discovery should succeed");
  let summary_a = analyze_python_parse(root, &discovered).expect("parse analysis should succeed");
  let summary_b = analyze_python_parse(root, &discovered).expect("parse analysis should succeed");

  assert_eq!(summary_a.files_parsed, 2);
  assert_eq!(summary_a.files_with_syntax_errors, 1);
  assert!(summary_a.syntax_error_count >= 1);
  assert_eq!(summary_a.internal_error_count, 0);
  assert_eq!(summary_a, summary_b);
}

#[test]
fn analyze_python_parse_reports_non_utf8_file_without_failing() {
  let temp_dir = TempDir::new().expect("temp dir should be created");
  let root = temp_dir.path();
  fs::create_dir(root.join(".git")).expect(".git should be created");

  let path = root.join("src/non_utf8.py");
  fs::create_dir_all(path.parent().expect("parent should exist"))
    .expect("parent dir should be created");
  fs::write(
    &path,
    [
      0xff, 0xfe, b'd', b'e', b'f', b' ', b'x', b'(', b')', b':', b'\n',
    ],
  )
  .expect("non-utf8 file should be written");

  let discovered = discover_files(root).expect("file discovery should succeed");
  let summary = analyze_python_parse(root, &discovered).expect("parse analysis should succeed");

  assert_eq!(summary.files_parsed, 1);
  assert_eq!(summary.files_with_syntax_errors, 0);
  assert_eq!(summary.syntax_error_count, 0);
  assert_eq!(summary.internal_error_count, 1);
  assert_eq!(summary.issues.len(), 1);
  assert_eq!(summary.issues[0].path, "src/non_utf8.py");
  assert!(summary.issues[0].message.contains("non-utf8 python source"));
}
