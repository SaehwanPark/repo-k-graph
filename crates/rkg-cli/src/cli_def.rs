//! Clap CLI argument definitions shared between the `rkg` and `rkg-completions` binaries.

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "rkg")]
#[command(about = "repo-k-graph CLI", long_about = None)]
pub struct Cli {
  #[command(subcommand)]
  pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
  /// Initialize the local knowledge store at `./.rkg/rkg.db`.
  Init,
  /// Index (or reindex) repository files into the knowledge store.
  Index {
    /// Force full reindex of all discovered files.
    #[arg(long)]
    force: bool,
    /// Alias for the default incremental index behaviour (changed files only).
    #[arg(long)]
    changed: bool,
    /// Run post-macro AST extraction via `cargo expand` after symbol indexing.
    #[arg(long)]
    expand: bool,
  },
  /// List all indexed files in the repository.
  Files,
  /// List all indexed symbols in the repository.
  Symbols,
  /// Find symbols by simple name.
  Find {
    /// Simple (unqualified) symbol name to look up.
    name: String,
  },
  /// Show definition snippet and metadata for a fully qualified symbol.
  Show {
    /// Fully qualified symbol name (e.g. `src/a/b.py::ClassName.method`).
    qualified_name: String,
  },
  /// List import edges for a file path.
  Imports {
    /// Repository-relative file path.
    path: String,
  },
  /// List files that import the given file path.
  #[command(name = "imported-by")]
  ImportedBy {
    /// Repository-relative file path.
    path: String,
  },
  /// List symbols called by the given symbol.
  Callees {
    /// Simple or qualified symbol name.
    name: String,
  },
  /// List symbols that call the given symbol.
  Callers {
    /// Simple or qualified symbol name.
    name: String,
  },
  /// List type references for the given symbol.
  Types {
    /// Simple or qualified symbol name.
    name: String,
  },
  /// List decorators applied to or by the given symbol.
  Decorators {
    /// Simple or qualified symbol name.
    name: String,
  },
  /// List tests linked to the given symbol.
  Tests {
    /// Simple or qualified symbol name.
    name: String,
  },
  /// List symbols that the given test depends on.
  #[command(name = "test-deps")]
  TestDeps {
    /// Simple or qualified test name.
    name: String,
  },
  /// List fixture dependencies for the given test.
  Fixtures {
    /// Simple or qualified test name.
    name: String,
  },
  /// Show documentation linked to the given symbol.
  Docs {
    /// Simple or qualified symbol name.
    name: String,
  },
  /// Search documentation blocks using BM25-ranked FTS5 (falls back to LIKE).
  #[command(name = "doc-search")]
  DocSearch {
    /// Free-text search query.
    query: String,
  },
  /// Search symbols and docs using BM25-ranked FTS5.
  #[command(name = "search")]
  Search {
    /// Free-text search query.
    query: String,
  },
  /// Show transitive impact analysis for a symbol.
  Impact {
    /// Simple or qualified symbol name.
    symbol: String,
    /// Maximum traversal depth.
    #[arg(long, short, default_value = "2")]
    depth: usize,
  },
  /// Pack context for a symbol into a token-budgeted representation.
  Context {
    /// Simple or qualified symbol name.
    symbol: String,
    /// Maximum token budget for the output.
    #[arg(long, short)]
    budget: Option<usize>,
    /// Output format: `markdown` or `json`.
    #[arg(long, short, default_value = "markdown")]
    format: String,
  },
  /// Show Git metadata (author frequency, churn, last commit) for a file.
  Git {
    /// Repository-relative file path.
    path: String,
  },
  /// Show co-change analysis for a symbol or file.
  Cochange {
    /// Simple or qualified symbol name, or repository-relative file path.
    name: String,
  },
  /// List indexed HTTP routes.
  Routes,
  /// Show Pydantic model fields, validators, and dependencies.
  Model {
    /// Model class name.
    name: String,
  },
  /// Render monadic/ROP pipeline execution sequence for a symbol.
  Pipeline {
    /// Simple or qualified symbol name.
    name: String,
  },
  /// Show concurrency topology for a symbol.
  Concurrency {
    /// Simple or qualified symbol name.
    name: String,
  },
  /// Show Rust FFI and memory-safety profile for a symbol or file.
  Safety {
    /// Simple or qualified symbol name, or repository-relative file path.
    target: String,
  },
  /// Show coverage profile for a symbol or file.
  Coverage {
    /// Simple or qualified symbol name, or repository-relative file path.
    target: String,
  },
  /// Import a coverage report (LCOV or Cobertura XML).
  #[command(name = "import-coverage")]
  ImportCoverage {
    /// Path to the coverage report file.
    path: String,
    /// Optional name of the test suite (overrides parsed TN from report).
    #[arg(long)]
    test_suite: Option<String>,
  },
  /// List workspace packages (Cargo, F#, Kotlin, Swift).
  Workspace,
  /// Show async spawn and channel topology for the workspace.
  Topology,
  /// Show dependency graph for a workspace package.
  Deps {
    /// Package name as declared in its manifest.
    package: String,
  },
  /// Show Android components and resources.
  Android {
    #[command(subcommand)]
    command: AndroidCommands,
  },
  /// Database lifecycle commands.
  Db {
    #[command(subcommand)]
    command: DbCommands,
  },
  /// Model Context Protocol server commands.
  Mcp {
    #[command(subcommand)]
    command: McpCommands,
  },
  /// Run evaluation and benchmarks on fixture repositories.
  Bench {
    /// Optional path to custom tasks JSON file.
    #[arg(long)]
    config: Option<String>,
    /// Print raw JSON results to stdout.
    #[arg(long)]
    json: bool,
    /// Save result output to the specified file path.
    #[arg(long, short)]
    output: Option<String>,
  },
}

/// Sub-commands for `rkg db`.
#[derive(Debug, Subcommand)]
pub enum DbCommands {
  /// Report database path, schema presence, and core-table row counts.
  Status,
  /// Recreate the local database from scratch (destructive).
  Reset,
}

/// Sub-commands for `rkg mcp`.
#[derive(Debug, Subcommand)]
pub enum McpCommands {
  /// Start the MCP stdio server.
  Serve,
}

/// Sub-commands for `rkg android`.
#[derive(Debug, Subcommand)]
pub enum AndroidCommands {
  /// List Android manifest components (Activities, Services, etc.) and their details.
  Components,
  /// List Android resources (layouts, strings, IDs) and their references.
  Resources,
}
