const USER_MANUAL: &str = include_str!("../../../docs/user-manual.md");
const RELEASE_CHECKLIST: &str = include_str!("../../../docs/release-checklist.md");
const SCHEMA_REFERENCE: &str = include_str!("../../../docs/schema-reference.md");
const LANGUAGE_ADAPTER_GUIDE: &str = include_str!("../../../docs/language-adapter-guide.md");

#[test]
fn user_manual_links_phase_18_3_distribution_docs() {
  for expected_link in [
    "docs/release-checklist.md",
    "docs/schema-reference.md",
    "docs/language-adapter-guide.md",
  ] {
    assert!(
      USER_MANUAL.contains(expected_link),
      "user manual should link to {expected_link}"
    );
  }
}

#[test]
fn release_checklist_covers_install_channel_validation() {
  for expected in [
    "GitHub Release",
    "rkg-linux-x86_64",
    "rkg-macos-aarch64",
    "rkg-windows-x86_64.exe",
    "rkg.bash",
    "_rkg",
    "rkg.fish",
    "Homebrew",
    "sha256",
    "brew audit --new --formula",
    "brew install --build-from-source",
    "cargo package --workspace --allow-dirty --list",
  ] {
    assert!(
      RELEASE_CHECKLIST.contains(expected),
      "release checklist should include {expected:?}"
    );
  }
}

#[test]
fn schema_reference_documents_core_tables_and_population_sources() {
  for table in [
    "repositories",
    "files",
    "symbols",
    "edges",
    "docs",
    "tests",
    "index_runs",
    "git_commits",
    "git_commit_files",
    "git_commit_symbols",
    "routes",
    "pydantic_models",
    "pydantic_fields",
    "pydantic_validators",
    "cargo_packages",
    "cargo_dependencies",
    "concurrency_spawns",
    "concurrency_channels",
    "concurrency_selects",
    "rust_unsafe_blocks",
    "rust_unsafe_functions",
    "rust_ffi_bindings",
    "fsharp_projects",
    "fsharp_dependencies",
    "kotlin_projects",
    "kotlin_dependencies",
    "swift_projects",
    "swift_dependencies",
  ] {
    assert!(
      SCHEMA_REFERENCE.contains(table),
      "schema reference should document {table}"
    );
  }

  for expected in [
    "ON DELETE CASCADE",
    "`rkg init`",
    "`rkg db reset`",
    "`rkg index`",
    "`rkg search`",
    "`rkg workspace`",
    "`rkg git`",
    "`rkg cochange`",
    "`rkg concurrency`",
    "`rkg topology`",
    "`rkg safety`",
  ] {
    assert!(
      SCHEMA_REFERENCE.contains(expected),
      "schema reference should mention {expected:?}"
    );
  }
}

#[test]
fn language_adapter_guide_covers_required_adapter_steps() {
  for expected in [
    "rkg-lang-*",
    "file discovery",
    "language detection",
    "parser unit tests",
    "CLI indexing",
    "persistence",
    "query",
    "integration tests",
    "SPEC.md",
    "CHANGELOG.md",
  ] {
    assert!(
      LANGUAGE_ADAPTER_GUIDE.contains(expected),
      "language adapter guide should include {expected:?}"
    );
  }
}
