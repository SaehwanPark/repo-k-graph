use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Result};

pub const CRATE_NAME: &str = "rkg-db";
const CORE_TABLES: [&str; 26] = [
  "repositories",
  "files",
  "symbols",
  "edges",
  "docs",
  "tests",
  "index_runs",
  "routes",
  "pydantic_models",
  "pydantic_fields",
  "pydantic_validators",
  "cargo_packages",
  "cargo_dependencies",
  "rust_unsafe_blocks",
  "rust_unsafe_functions",
  "rust_ffi_bindings",
  "fsharp_projects",
  "fsharp_dependencies",
  "kotlin_projects",
  "kotlin_dependencies",
  "swift_projects",
  "swift_dependencies",
  "symbol_coverage",
  "generated_symbols",
  "android_components",
  "android_resources",
];

#[derive(Debug, Clone, PartialEq)]
pub struct DbStatus {
  pub schema_initialized: bool,
  pub repositories: i64,
  pub files: i64,
  pub symbols: i64,
  pub edges: i64,
  pub docs: i64,
  pub tests: i64,
  pub index_runs: i64,
  pub routes: i64,
  pub pydantic_models: i64,
  pub pydantic_fields: i64,
  pub pydantic_validators: i64,
  pub cargo_packages: i64,
  pub cargo_dependencies: i64,
  pub rust_unsafe_blocks: i64,
  pub rust_unsafe_functions: i64,
  pub rust_ffi_bindings: i64,
  pub fsharp_projects: i64,
  pub fsharp_dependencies: i64,
  pub kotlin_projects: i64,
  pub kotlin_dependencies: i64,
  pub swift_projects: i64,
  pub swift_dependencies: i64,
  pub symbol_coverage: i64,
  pub generated_symbols: i64,
  pub android_components: i64,
  pub android_resources: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewFileRecord {
  pub repository_id: i64,
  pub path: String,
  pub language: Option<String>,
  pub content_hash: Option<String>,
  pub line_count: Option<i64>,
  pub last_index_run_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewConcurrencySpawnRecord {
  pub file_id: i64,
  pub source_symbol_qualified_name: String,
  pub spawn_kind: String,
  pub target_name: Option<String>,
  pub start_line: i64,
  pub end_line: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConcurrencySpawnRecord {
  pub id: i64,
  pub file_id: i64,
  pub source_symbol_qualified_name: String,
  pub spawn_kind: String,
  pub target_name: Option<String>,
  pub start_line: i64,
  pub end_line: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewConcurrencyChannelRecord {
  pub file_id: i64,
  pub source_symbol_qualified_name: String,
  pub channel_kind: String,
  pub tx_name: String,
  pub rx_name: String,
  pub start_line: i64,
  pub end_line: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConcurrencyChannelRecord {
  pub id: i64,
  pub file_id: i64,
  pub source_symbol_qualified_name: String,
  pub channel_kind: String,
  pub tx_name: String,
  pub rx_name: String,
  pub start_line: i64,
  pub end_line: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewConcurrencySelectRecord {
  pub file_id: i64,
  pub source_symbol_qualified_name: String,
  pub start_line: i64,
  pub end_line: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConcurrencySelectRecord {
  pub id: i64,
  pub file_id: i64,
  pub source_symbol_qualified_name: String,
  pub start_line: i64,
  pub end_line: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewRustUnsafeBlockRecord {
  pub file_id: i64,
  pub source_symbol_qualified_name: String,
  pub start_line: i64,
  pub end_line: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RustUnsafeBlockRecord {
  pub id: i64,
  pub file_id: i64,
  pub source_symbol_qualified_name: String,
  pub start_line: i64,
  pub end_line: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewRustUnsafeFunctionRecord {
  pub file_id: i64,
  pub qualified_name: String,
  pub start_line: i64,
  pub end_line: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RustUnsafeFunctionRecord {
  pub id: i64,
  pub file_id: i64,
  pub qualified_name: String,
  pub start_line: i64,
  pub end_line: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewRustFFIBindingRecord {
  pub file_id: i64,
  pub source_symbol_qualified_name: String,
  pub foreign_item_name: String,
  pub abi: String,
  pub start_line: i64,
  pub end_line: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RustFFIBindingRecord {
  pub id: i64,
  pub file_id: i64,
  pub source_symbol_qualified_name: String,
  pub foreign_item_name: String,
  pub abi: String,
  pub start_line: i64,
  pub end_line: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileRecord {
  pub id: i64,
  pub repository_id: i64,
  pub path: String,
  pub language: Option<String>,
  pub content_hash: Option<String>,
  pub line_count: Option<i64>,
  pub last_index_run_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewSymbolRecord {
  pub file_id: i64,
  pub name: String,
  pub qualified_name: String,
  pub kind: String,
  pub start_line: i64,
  pub end_line: i64,
  pub start_column: Option<i64>,
  pub end_column: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SymbolRecord {
  pub id: i64,
  pub file_id: i64,
  pub name: String,
  pub qualified_name: String,
  pub kind: String,
  pub start_line: i64,
  pub end_line: i64,
  pub start_column: Option<i64>,
  pub end_column: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewSymbolCoverageRecord {
  pub file_id: i64,
  pub symbol_id: i64,
  pub report_path: String,
  pub test_suite: Option<String>,
  pub lines_valid: i64,
  pub lines_covered: i64,
  pub branches_valid: i64,
  pub branches_covered: i64,
  pub coverable_lines: String,
  pub uncovered_lines: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SymbolCoverageRecord {
  pub id: i64,
  pub file_id: i64,
  pub symbol_id: i64,
  pub report_path: String,
  pub test_suite: Option<String>,
  pub lines_valid: i64,
  pub lines_covered: i64,
  pub branches_valid: i64,
  pub branches_covered: i64,
  pub coverable_lines: String,
  pub uncovered_lines: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewEdgeRecord {
  pub source_symbol_id: i64,
  pub target_symbol_id: Option<i64>,
  pub unresolved_target: Option<String>,
  pub kind: String,
  pub confidence: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EdgeRecord {
  pub id: i64,
  pub source_symbol_id: i64,
  pub target_symbol_id: Option<i64>,
  pub unresolved_target: Option<String>,
  pub kind: String,
  pub confidence: Option<f64>,
  pub ordering: Option<i64>,
  pub placeholders: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTestRecord {
  pub file_id: i64,
  pub name: String,
  pub qualified_name: String,
  pub kind: String,
  pub is_parametrized: bool,
  pub framework: String,
  pub start_line: Option<i64>,
  pub end_line: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestRecord {
  pub id: i64,
  pub file_id: i64,
  pub name: String,
  pub qualified_name: String,
  pub kind: String,
  pub is_parametrized: bool,
  pub framework: String,
  pub start_line: Option<i64>,
  pub end_line: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRouteRecord {
  pub file_id: i64,
  pub symbol_id: Option<i64>,
  pub handler_name: String,
  pub qualified_name: String,
  pub method: String,
  pub path: String,
  pub response_model: Option<String>,
  pub start_line: Option<i64>,
  pub end_line: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteRecord {
  pub id: i64,
  pub file_id: i64,
  pub symbol_id: Option<i64>,
  pub handler_name: String,
  pub qualified_name: String,
  pub method: String,
  pub path: String,
  pub response_model: Option<String>,
  pub start_line: Option<i64>,
  pub end_line: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAndroidComponentRecord {
  pub file_id: i64,
  pub name: String,
  pub component_type: String,
  pub class_name: String,
  pub permission: Option<String>,
  pub intent_actions: Vec<String>,
  pub intent_categories: Vec<String>,
  pub start_line: Option<i64>,
  pub end_line: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndroidComponentRecord {
  pub id: i64,
  pub file_id: i64,
  pub name: String,
  pub component_type: String,
  pub class_name: String,
  pub permission: Option<String>,
  pub intent_actions: Vec<String>,
  pub intent_categories: Vec<String>,
  pub start_line: Option<i64>,
  pub end_line: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAndroidResourceRecord {
  pub file_id: i64,
  pub name: String,
  pub resource_type: String,
  pub value: Option<String>,
  pub start_line: Option<i64>,
  pub end_line: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndroidResourceRecord {
  pub id: i64,
  pub file_id: i64,
  pub name: String,
  pub resource_type: String,
  pub value: Option<String>,
  pub start_line: Option<i64>,
  pub end_line: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewDocRecord {
  pub file_id: i64,
  pub symbol_id: Option<i64>,
  pub title: Option<String>,
  pub body: String,
  pub start_line: Option<i64>,
  pub end_line: Option<i64>,
  pub source_kind: String, // "Markdown" or "Docstring"
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocRecord {
  pub id: i64,
  pub file_id: i64,
  pub symbol_id: Option<i64>,
  pub title: Option<String>,
  pub body: String,
  pub start_line: Option<i64>,
  pub end_line: Option<i64>,
  pub source_kind: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewPydanticModelRecord {
  pub file_id: i64,
  pub symbol_id: Option<i64>,
  pub name: String,
  pub qualified_name: String,
  pub start_line: i64,
  pub end_line: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PydanticModelRecord {
  pub id: i64,
  pub file_id: i64,
  pub symbol_id: Option<i64>,
  pub name: String,
  pub qualified_name: String,
  pub start_line: i64,
  pub end_line: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewPydanticFieldRecord {
  pub model_id: i64,
  pub name: String,
  pub type_annotation: String,
  pub default_value: Option<String>,
  pub is_required: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PydanticFieldRecord {
  pub id: i64,
  pub model_id: i64,
  pub name: String,
  pub type_annotation: String,
  pub default_value: Option<String>,
  pub is_required: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewPydanticValidatorRecord {
  pub model_id: i64,
  pub name: String,
  pub validator_type: String,
  pub target_fields: String, // comma-separated
}

#[derive(Debug, Clone, PartialEq)]
pub struct PydanticValidatorRecord {
  pub id: i64,
  pub model_id: i64,
  pub name: String,
  pub validator_type: String,
  pub target_fields: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewCargoPackageRecord {
  pub repository_id: i64,
  pub name: String,
  pub manifest_path: String,
  pub version: String,
  pub is_workspace_member: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CargoPackageRecord {
  pub id: i64,
  pub repository_id: i64,
  pub name: String,
  pub manifest_path: String,
  pub version: String,
  pub is_workspace_member: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewCargoDependencyRecord {
  pub package_id: i64,
  pub name: String,
  pub version_requirement: Option<String>,
  pub is_workspace_dependency: bool,
  pub features: String,
  pub is_dev: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CargoDependencyRecord {
  pub id: i64,
  pub package_id: i64,
  pub name: String,
  pub version_requirement: Option<String>,
  pub is_workspace_dependency: bool,
  pub features: String,
  pub is_dev: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewFSharpProjectRecord {
  pub repository_id: i64,
  pub name: String,
  pub project_path: String,
  pub target_framework: Option<String>,
  pub is_solution_member: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FSharpProjectRecord {
  pub id: i64,
  pub repository_id: i64,
  pub name: String,
  pub project_path: String,
  pub target_framework: Option<String>,
  pub is_solution_member: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewFSharpDependencyRecord {
  pub project_id: i64,
  pub name: String,
  pub dependency_type: String, // "package" or "project"
  pub version_requirement: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FSharpDependencyRecord {
  pub id: i64,
  pub project_id: i64,
  pub name: String,
  pub dependency_type: String,
  pub version_requirement: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewKotlinProjectRecord {
  pub repository_id: i64,
  pub name: String,
  pub project_path: String,
  pub target_framework: Option<String>,
  pub is_solution_member: bool,
  pub generated_source_dirs: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KotlinProjectRecord {
  pub id: i64,
  pub repository_id: i64,
  pub name: String,
  pub project_path: String,
  pub target_framework: Option<String>,
  pub is_solution_member: bool,
  pub generated_source_dirs: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewKotlinDependencyRecord {
  pub project_id: i64,
  pub name: String,
  pub dependency_type: String, // "package" or "project"
  pub version_requirement: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KotlinDependencyRecord {
  pub id: i64,
  pub project_id: i64,
  pub name: String,
  pub dependency_type: String,
  pub version_requirement: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewSwiftProjectRecord {
  pub repository_id: i64,
  pub name: String,
  pub project_path: String,
  pub target_framework: Option<String>,
  pub is_solution_member: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwiftProjectRecord {
  pub id: i64,
  pub repository_id: i64,
  pub name: String,
  pub project_path: String,
  pub target_framework: Option<String>,
  pub is_solution_member: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewSwiftDependencyRecord {
  pub project_id: i64,
  pub name: String,
  pub dependency_type: String, // "package" or "project"
  pub version_requirement: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwiftDependencyRecord {
  pub id: i64,
  pub project_id: i64,
  pub name: String,
  pub dependency_type: String,
  pub version_requirement: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryRecord {
  pub id: i64,
  pub root_path: String,
  pub vcs_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewIndexRunRecord {
  pub repository_id: i64,
  pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishedIndexRunRecord {
  pub files_scanned: i64,
  pub files_changed: i64,
  pub files_deleted: i64,
  pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexRunRecord {
  pub id: i64,
  pub repository_id: i64,
  pub status: String,
  pub files_scanned: i64,
  pub files_changed: i64,
  pub files_deleted: i64,
  pub error_message: Option<String>,
  pub finished_at: Option<String>,
}

pub fn open_or_create(path: impl AsRef<Path>) -> Result<Connection> {
  let connection = Connection::open(path)?;
  connection.execute_batch(
    "PRAGMA journal_mode = WAL;
     PRAGMA busy_timeout = 5000;",
  )?;
  initialize_schema(&connection)?;
  Ok(connection)
}

pub fn upsert_repository(connection: &Connection, root_path: &str) -> Result<RepositoryRecord> {
  connection.execute(
    "INSERT INTO repositories (root_path, vcs_type)
      VALUES (?1, 'git')
      ON CONFLICT(root_path) DO UPDATE SET
        vcs_type = excluded.vcs_type,
        updated_at = CURRENT_TIMESTAMP",
    [root_path],
  )?;

  lookup_repository_by_root_path(connection, root_path)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn lookup_repository_by_root_path(
  connection: &Connection,
  root_path: &str,
) -> Result<Option<RepositoryRecord>> {
  let mut statement = connection.prepare(
    "SELECT id, root_path, vcs_type
      FROM repositories
      WHERE root_path = ?1",
  )?;

  statement
    .query_row([root_path], |row| {
      Ok(RepositoryRecord {
        id: row.get(0)?,
        root_path: row.get(1)?,
        vcs_type: row.get(2)?,
      })
    })
    .optional()
}

pub fn list_files_for_repository(
  connection: &Connection,
  repository_id: i64,
) -> Result<Vec<FileRecord>> {
  let mut statement = connection.prepare(
    "SELECT id, repository_id, path, language, content_hash, line_count, last_index_run_id
      FROM files
      WHERE repository_id = ?1
      ORDER BY path ASC, id ASC",
  )?;
  let rows = statement.query_map([repository_id], |row| {
    Ok(FileRecord {
      id: row.get(0)?,
      repository_id: row.get(1)?,
      path: row.get(2)?,
      language: row.get(3)?,
      content_hash: row.get(4)?,
      line_count: row.get(5)?,
      last_index_run_id: row.get(6)?,
    })
  })?;
  rows.collect()
}

pub fn start_index_run(connection: &Connection, run: &NewIndexRunRecord) -> Result<IndexRunRecord> {
  connection.execute(
    "INSERT INTO index_runs (repository_id, status)
      VALUES (?1, ?2)",
    (run.repository_id, run.status.as_str()),
  )?;

  let id = connection.last_insert_rowid();
  get_index_run_by_id(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn finish_index_run(
  connection: &Connection,
  index_run_id: i64,
  status: &str,
  summary: &FinishedIndexRunRecord,
) -> Result<IndexRunRecord> {
  connection.execute(
    "UPDATE index_runs
      SET status = ?2,
          finished_at = CURRENT_TIMESTAMP,
          files_scanned = ?3,
          files_changed = ?4,
          files_deleted = ?5,
          error_message = ?6
      WHERE id = ?1",
    (
      index_run_id,
      status,
      summary.files_scanned,
      summary.files_changed,
      summary.files_deleted,
      summary.error_message.as_deref(),
    ),
  )?;

  get_index_run_by_id(connection, index_run_id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn insert_file(connection: &Connection, file: &NewFileRecord) -> Result<FileRecord> {
  connection.execute(
    "INSERT INTO files (repository_id, path, language, content_hash, line_count, last_index_run_id)
      VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    (
      file.repository_id,
      file.path.as_str(),
      file.language.as_deref(),
      file.content_hash.as_deref(),
      file.line_count,
      file.last_index_run_id,
    ),
  )?;

  let id = connection.last_insert_rowid();
  get_file_by_id(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn insert_symbol(connection: &Connection, symbol: &NewSymbolRecord) -> Result<SymbolRecord> {
  connection.execute(
    "INSERT INTO symbols (file_id, name, qualified_name, kind, start_line, end_line, start_column, end_column)
      VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
      ON CONFLICT(file_id, qualified_name) DO UPDATE SET
        name = excluded.name,
        kind = excluded.kind,
        start_line = excluded.start_line,
        end_line = excluded.end_line,
        start_column = excluded.start_column,
        end_column = excluded.end_column,
        updated_at = CURRENT_TIMESTAMP",
    (
      symbol.file_id,
      symbol.name.as_str(),
      symbol.qualified_name.as_str(),
      symbol.kind.as_str(),
      symbol.start_line,
      symbol.end_line,
      symbol.start_column,
      symbol.end_column,
    ),
  )?;

  let id: i64 = connection.query_row(
    "SELECT id FROM symbols WHERE file_id = ?1 AND qualified_name = ?2",
    (symbol.file_id, symbol.qualified_name.as_str()),
    |row| row.get(0),
  )?;

  get_symbol_by_id(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn insert_generated_symbol(
  connection: &Connection,
  symbol_id: i64,
  provenance: &str,
) -> Result<()> {
  connection.execute(
    "INSERT INTO generated_symbols (symbol_id, provenance)
     VALUES (?1, ?2)
     ON CONFLICT(symbol_id) DO UPDATE SET provenance = excluded.provenance",
    (symbol_id, provenance),
  )?;
  Ok(())
}

pub fn get_symbol_provenance(connection: &Connection, symbol_id: i64) -> Result<Option<String>> {
  let mut stmt =
    connection.prepare("SELECT provenance FROM generated_symbols WHERE symbol_id = ?1")?;
  stmt.query_row([symbol_id], |row| row.get(0)).optional()
}

pub fn get_test_by_id(connection: &Connection, id: i64) -> Result<Option<TestRecord>> {
  let mut statement = connection.prepare(
    "SELECT id, file_id, name, qualified_name, kind, is_parametrized, framework, start_line, end_line
      FROM tests
      WHERE id = ?1",
  )?;

  statement
    .query_row([id], |row| {
      let is_parametrized_int: i64 = row.get(5)?;
      Ok(TestRecord {
        id: row.get(0)?,
        file_id: row.get(1)?,
        name: row.get(2)?,
        qualified_name: row.get(3)?,
        kind: row.get(4)?,
        is_parametrized: is_parametrized_int != 0,
        framework: row.get(6)?,
        start_line: row.get(7)?,
        end_line: row.get(8)?,
      })
    })
    .optional()
}

pub fn insert_doc(connection: &Connection, doc: &NewDocRecord) -> Result<DocRecord> {
  connection.execute(
    "INSERT INTO docs (file_id, symbol_id, title, body, start_line, end_line, source_kind)
      VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    (
      doc.file_id,
      doc.symbol_id,
      doc.title.as_deref(),
      doc.body.as_str(),
      doc.start_line,
      doc.end_line,
      doc.source_kind.as_str(),
    ),
  )?;

  let id = connection.last_insert_rowid();
  get_doc_by_id(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn get_doc_by_id(connection: &Connection, doc_id: i64) -> Result<Option<DocRecord>> {
  let mut statement = connection.prepare(
    "SELECT id, file_id, symbol_id, title, body, start_line, end_line, source_kind
      FROM docs
      WHERE id = ?1",
  )?;

  statement
    .query_row([doc_id], |row| {
      Ok(DocRecord {
        id: row.get(0)?,
        file_id: row.get(1)?,
        symbol_id: row.get(2)?,
        title: row.get(3)?,
        body: row.get(4)?,
        start_line: row.get(5)?,
        end_line: row.get(6)?,
        source_kind: row.get(7)?,
      })
    })
    .optional()
}

pub fn insert_test(connection: &Connection, test: &NewTestRecord) -> Result<TestRecord> {
  connection.execute(
    "INSERT INTO tests (file_id, name, qualified_name, kind, is_parametrized, framework, start_line, end_line)
      VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
      ON CONFLICT(file_id, qualified_name) DO UPDATE SET
        name = excluded.name,
        kind = excluded.kind,
        is_parametrized = excluded.is_parametrized,
        framework = excluded.framework,
        start_line = excluded.start_line,
        end_line = excluded.end_line,
        updated_at = CURRENT_TIMESTAMP",
    (
      test.file_id,
      test.name.as_str(),
      test.qualified_name.as_str(),
      test.kind.as_str(),
      if test.is_parametrized { 1 } else { 0 },
      test.framework.as_str(),
      test.start_line,
      test.end_line,
    ),
  )?;

  let id: i64 = connection.query_row(
    "SELECT id FROM tests WHERE file_id = ?1 AND qualified_name = ?2",
    (test.file_id, test.qualified_name.as_str()),
    |row| row.get(0),
  )?;

  get_test_by_id(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn lookup_tests_by_file_id(connection: &Connection, file_id: i64) -> Result<Vec<TestRecord>> {
  let mut statement = connection.prepare(
    "SELECT id, file_id, name, qualified_name, kind, is_parametrized, framework, start_line, end_line
      FROM tests
      WHERE file_id = ?1
      ORDER BY name",
  )?;

  let rows = statement.query_map([file_id], |row| {
    let is_parametrized_int: i64 = row.get(5)?;
    Ok(TestRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      name: row.get(2)?,
      qualified_name: row.get(3)?,
      kind: row.get(4)?,
      is_parametrized: is_parametrized_int != 0,
      framework: row.get(6)?,
      start_line: row.get(7)?,
      end_line: row.get(8)?,
    })
  })?;

  rows.collect()
}

pub fn insert_edge(connection: &Connection, edge: &NewEdgeRecord) -> Result<EdgeRecord> {
  connection.execute(
    "INSERT INTO edges (source_symbol_id, target_symbol_id, unresolved_target, kind, confidence)
      VALUES (?1, ?2, ?3, ?4, ?5)",
    (
      edge.source_symbol_id,
      edge.target_symbol_id,
      edge.unresolved_target.as_deref(),
      edge.kind.as_str(),
      edge.confidence,
    ),
  )?;

  let id = connection.last_insert_rowid();
  get_edge_by_id(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn insert_edge_with_pipeline_metadata(
  connection: &Connection,
  edge: &NewEdgeRecord,
  ordering: Option<i64>,
  placeholders: Option<String>,
) -> Result<EdgeRecord> {
  connection.execute(
    "INSERT INTO edges (source_symbol_id, target_symbol_id, unresolved_target, kind, confidence, ordering, placeholders)
      VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    (
      edge.source_symbol_id,
      edge.target_symbol_id,
      edge.unresolved_target.as_deref(),
      edge.kind.as_str(),
      edge.confidence,
      ordering,
      placeholders.as_deref(),
    ),
  )?;

  let id = connection.last_insert_rowid();
  get_edge_by_id(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn lookup_file_by_path(
  connection: &Connection,
  repository_id: i64,
  path: &str,
) -> Result<Option<FileRecord>> {
  let mut statement = connection.prepare(
    "SELECT id, repository_id, path, language, content_hash, line_count, last_index_run_id
      FROM files
      WHERE repository_id = ?1 AND path = ?2",
  )?;

  statement
    .query_row((repository_id, path), |row| {
      Ok(FileRecord {
        id: row.get(0)?,
        repository_id: row.get(1)?,
        path: row.get(2)?,
        language: row.get(3)?,
        content_hash: row.get(4)?,
        line_count: row.get(5)?,
        last_index_run_id: row.get(6)?,
      })
    })
    .optional()
}

pub fn lookup_symbols_by_name(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> Result<Vec<SymbolRecord>> {
  let mut statement = connection.prepare(
    "SELECT s.id, s.file_id, s.name, s.qualified_name, s.kind, s.start_line, s.end_line, s.start_column, s.end_column
      FROM symbols s
      INNER JOIN files f ON f.id = s.file_id
      WHERE f.repository_id = ?1 AND s.name = ?2
      ORDER BY s.qualified_name ASC, s.id ASC",
  )?;

  let symbol_rows = statement.query_map((repository_id, name), |row| {
    Ok(SymbolRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      name: row.get(2)?,
      qualified_name: row.get(3)?,
      kind: row.get(4)?,
      start_line: row.get(5)?,
      end_line: row.get(6)?,
      start_column: row.get(7)?,
      end_column: row.get(8)?,
    })
  })?;

  symbol_rows.collect()
}

pub fn list_symbols_for_repository(
  connection: &Connection,
  repository_id: i64,
) -> Result<Vec<SymbolRecord>> {
  let mut statement = connection.prepare(
    "SELECT s.id, s.file_id, s.name, s.qualified_name, s.kind, s.start_line, s.end_line, s.start_column, s.end_column
      FROM symbols s
      INNER JOIN files f ON f.id = s.file_id
      WHERE f.repository_id = ?1
      ORDER BY s.qualified_name ASC, s.id ASC",
  )?;

  let symbol_rows = statement.query_map((repository_id,), |row| {
    Ok(SymbolRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      name: row.get(2)?,
      qualified_name: row.get(3)?,
      kind: row.get(4)?,
      start_line: row.get(5)?,
      end_line: row.get(6)?,
      start_column: row.get(7)?,
      end_column: row.get(8)?,
    })
  })?;

  symbol_rows.collect()
}

pub fn lookup_symbols_by_file_id(
  connection: &Connection,
  file_id: i64,
) -> Result<Vec<SymbolRecord>> {
  let mut statement = connection.prepare(
    "SELECT id, file_id, name, qualified_name, kind, start_line, end_line, start_column, end_column
      FROM symbols
      WHERE file_id = ?1
      ORDER BY qualified_name ASC, id ASC",
  )?;

  let symbol_rows = statement.query_map([file_id], |row| {
    Ok(SymbolRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      name: row.get(2)?,
      qualified_name: row.get(3)?,
      kind: row.get(4)?,
      start_line: row.get(5)?,
      end_line: row.get(6)?,
      start_column: row.get(7)?,
      end_column: row.get(8)?,
    })
  })?;

  symbol_rows.collect()
}

pub fn lookup_symbol_by_qualified_name(
  connection: &Connection,
  repository_id: i64,
  qualified_name: &str,
) -> Result<Option<SymbolRecord>> {
  let mut statement = connection.prepare(
    "SELECT s.id, s.file_id, s.name, s.qualified_name, s.kind, s.start_line, s.end_line, s.start_column, s.end_column
      FROM symbols s
      INNER JOIN files f ON f.id = s.file_id
      WHERE f.repository_id = ?1 AND s.qualified_name = ?2
      ORDER BY (CASE WHEN f.language = 'xml' THEN 1 ELSE 0 END) ASC, s.id ASC",
  )?;

  statement
    .query_row((repository_id, qualified_name), |row| {
      Ok(SymbolRecord {
        id: row.get(0)?,
        file_id: row.get(1)?,
        name: row.get(2)?,
        qualified_name: row.get(3)?,
        kind: row.get(4)?,
        start_line: row.get(5)?,
        end_line: row.get(6)?,
        start_column: row.get(7)?,
        end_column: row.get(8)?,
      })
    })
    .optional()
}

pub fn resolve_unresolved_edges(connection: &Connection, repository_id: i64) -> Result<usize> {
  let mut stmt = connection.prepare(
    "SELECT e.id, e.unresolved_target, e.kind, e.source_symbol_id
     FROM edges e
     INNER JOIN symbols s ON e.source_symbol_id = s.id
     INNER JOIN files f ON s.file_id = f.id
     WHERE f.repository_id = ?1 AND e.target_symbol_id IS NULL AND e.unresolved_target IS NOT NULL",
  )?;

  let rows = stmt.query_map([repository_id], |row| {
    Ok((
      row.get::<_, i64>(0)?,
      row.get::<_, String>(1)?,
      row.get::<_, String>(2)?,
      row.get::<_, i64>(3)?,
    ))
  })?;

  let unresolved: Vec<_> = rows.collect::<Result<Vec<_>, _>>()?;

  let mut resolved_count = 0;
  for (edge_id, target_qname, kind, source_symbol_id) in unresolved {
    let mut target_symbol =
      lookup_symbol_by_qualified_name(connection, repository_id, &target_qname)?;

    if target_symbol.is_none() {
      target_symbol = if let Some(last_dot_idx) = target_qname.rfind('.') {
        let parent = &target_qname[..last_dot_idx];
        let child = &target_qname[last_dot_idx + 1..];
        let qname_colons = format!("{parent}::{child}");
        lookup_symbol_by_qualified_name(connection, repository_id, &qname_colons)?
      } else if let Some(last_colons_idx) = target_qname.rfind("::") {
        let parent = &target_qname[..last_colons_idx];
        let child = &target_qname[last_colons_idx + 2..];
        let qname_colons = format!("{parent}::{child}");
        lookup_symbol_by_qualified_name(connection, repository_id, &qname_colons)?
      } else {
        None
      };
    }

    if target_symbol.is_none()
      && (kind == "Calls"
        || kind == "ReferencesType"
        || kind == "ModifiedWith"
        || kind == "ConfiguredBy"
        || kind == "TestedBy"
        || kind == "Spawns"
        || kind == "SendsTo"
        || kind == "Extends"
        || kind == "Implements")
    {
      // If it's a decorator and contains a dot or colons, avoid the simple basename fallback
      // to prevent corrupting external decorators to unrelated local symbols.
      if !(kind == "ModifiedWith" && (target_qname.contains('.') || target_qname.contains("::"))) {
        let simple_name = if target_qname.starts_with("self.") {
          target_qname.strip_prefix("self.").unwrap_or(&target_qname)
        } else if let Some(dot_idx) = target_qname.rfind('.') {
          &target_qname[dot_idx + 1..]
        } else if let Some(colons_idx) = target_qname.rfind("::") {
          &target_qname[colons_idx + 2..]
        } else {
          &target_qname
        };

        if simple_name != "*" {
          let mut matches = lookup_symbols_by_name(connection, repository_id, simple_name)?;
          if kind == "ReferencesType"
            || kind == "Calls"
            || kind == "Extends"
            || kind == "Implements"
          {
            matches.retain(|m| m.kind != "Module");
          }
          if matches.len() == 1 {
            target_symbol = Some(matches[0].clone());
          }
        }
      }
    }

    if target_symbol.is_none() {
      let file_path: String = connection
        .query_row(
          "SELECT f.path FROM files f
         INNER JOIN symbols s ON s.file_id = f.id
         WHERE s.id = ?1",
          [source_symbol_id],
          |row| row.get(0),
        )
        .unwrap_or_else(|_| "".to_string());

      if !file_path.is_empty() {
        let packages = list_cargo_packages(connection, repository_id).unwrap_or_default();
        let mut best_pkg: Option<CargoPackageRecord> = None;
        let mut best_len = 0;
        for pkg in &packages {
          let manifest_dir = if let Some(idx) = pkg.manifest_path.rfind('/') {
            &pkg.manifest_path[..idx]
          } else {
            ""
          };
          if manifest_dir.is_empty() || file_path.starts_with(&(manifest_dir.to_string() + "/")) {
            let len = manifest_dir.len();
            if len >= best_len {
              best_len = len;
              best_pkg = Some(pkg.clone());
            }
          }
        }

        if let Some(pkg) = best_pkg {
          let deps = list_cargo_dependencies(connection, pkg.id).unwrap_or_default();
          let first_segment = if let Some(idx) = target_qname.find("::") {
            &target_qname[..idx]
          } else if let Some(idx) = target_qname.find('.') {
            &target_qname[..idx]
          } else {
            &target_qname
          };

          if deps.iter().any(|d| d.name == first_segment) {
            let virtual_file_path = format!("external:{}", first_segment);
            connection.execute(
              "INSERT INTO files (repository_id, path, language)
               VALUES (?1, ?2, 'rust')
               ON CONFLICT(repository_id, path) DO UPDATE SET updated_at = CURRENT_TIMESTAMP",
              (repository_id, &virtual_file_path),
            )?;
            let virtual_file_id: i64 = connection.query_row(
              "SELECT id FROM files WHERE repository_id = ?1 AND path = ?2",
              (repository_id, &virtual_file_path),
              |row| row.get(0),
            )?;

            let simple_name = if let Some(idx) = target_qname.rfind("::") {
              &target_qname[idx + 2..]
            } else if let Some(idx) = target_qname.rfind('.') {
              &target_qname[idx + 1..]
            } else {
              &target_qname
            };

            connection.execute(
              "INSERT INTO symbols (file_id, name, qualified_name, kind, start_line, end_line)
               VALUES (?1, ?2, ?3, 'Unknown', 0, 0)
               ON CONFLICT(file_id, qualified_name) DO UPDATE SET updated_at = CURRENT_TIMESTAMP",
              (virtual_file_id, simple_name, &target_qname),
            )?;
            let virtual_symbol_id: i64 = connection.query_row(
              "SELECT id FROM symbols WHERE file_id = ?1 AND qualified_name = ?2",
              (virtual_file_id, &target_qname),
              |row| row.get(0),
            )?;

            target_symbol = Some(SymbolRecord {
              id: virtual_symbol_id,
              file_id: virtual_file_id,
              name: simple_name.to_string(),
              qualified_name: target_qname.clone(),
              kind: "Unknown".to_string(),
              start_line: 0,
              end_line: 0,
              start_column: None,
              end_column: None,
            });
          }
        }
      }
    }

    if let Some(sym) = target_symbol {
      let exists: bool = connection
        .query_row(
          "SELECT 1 FROM edges WHERE source_symbol_id = ?1 AND target_symbol_id = ?2 AND kind = ?3 LIMIT 1",
          (source_symbol_id, sym.id, &kind),
          |_| Ok(true),
        )
        .unwrap_or(false);

      if exists {
        connection.execute("DELETE FROM edges WHERE id = ?1", [edge_id])?;
      } else {
        connection.execute(
          "UPDATE edges SET target_symbol_id = ?1, unresolved_target = NULL WHERE id = ?2",
          (sym.id, edge_id),
        )?;
        resolved_count += 1;
      }
    }
  }

  Ok(resolved_count)
}

pub fn resolve_generated_symbols_linkages(
  connection: &Connection,
  repository_id: i64,
) -> Result<usize> {
  // Query all generated symbols for this repository
  let mut stmt = connection.prepare(
    "SELECT s.id, s.name, s.qualified_name
     FROM symbols s
     INNER JOIN files f ON f.id = s.file_id
     INNER JOIN generated_symbols gs ON gs.symbol_id = s.id
     WHERE f.repository_id = ?1",
  )?;

  let rows = stmt.query_map([repository_id], |row| {
    Ok(SymbolRecord {
      id: row.get(0)?,
      file_id: 0,
      name: row.get(1)?,
      qualified_name: row.get(2)?,
      kind: String::new(),
      start_line: 0,
      end_line: 0,
      start_column: None,
      end_column: None,
    })
  })?;

  let generated_symbols: Vec<SymbolRecord> = rows.collect::<Result<Vec<_>, _>>()?;
  let mut resolved_count = 0;

  for gen_sym in generated_symbols {
    let mut target_qname = None;
    let mut edge_kind = "ConfiguredBy";
    let mut annotation_check = "";

    let qname = &gen_sym.qualified_name;
    let name = &gen_sym.name;

    if qname.ends_with("Serializer") {
      let base = &qname[..qname.len() - "Serializer".len()];
      target_qname = Some(base.to_string());
      annotation_check = "Serializable";
    } else if qname.ends_with("_Impl") {
      let base = &qname[..qname.len() - "_Impl".len()];
      target_qname = Some(base.to_string());
      edge_kind = "Implements";
      annotation_check = "Dao";
    } else if let Some(base_name) = name.strip_prefix("Dagger") {
      if let Some(idx) = qname.rfind("::") {
        let package = &qname[..idx];
        target_qname = Some(format!("{}::{}", package, base_name));
      } else {
        target_qname = Some(base_name.to_string());
      }
      annotation_check = "Component";
    } else if qname.ends_with("JsonAdapter") {
      let base = &qname[..qname.len() - "JsonAdapter".len()];
      target_qname = Some(base.to_string());
      annotation_check = "JsonClass";
    }

    let Some(ref t_qname) = target_qname else {
      continue;
    };

    // Check if base symbol exists
    let target_symbol = lookup_symbol_by_qualified_name(connection, repository_id, t_qname)?;
    let Some(target_sym) = target_symbol else {
      continue;
    };

    let target_id = target_sym.id;
    let has_annotation = if annotation_check.is_empty() {
      false
    } else if annotation_check == "Component" {
      connection.query_row(
        "SELECT EXISTS (
          SELECT 1 FROM edges e
          WHERE e.source_symbol_id = ?1
            AND e.kind = 'ModifiedWith'
            AND (
              e.unresolved_target LIKE '%Component%' OR
              EXISTS (
                SELECT 1 FROM symbols s WHERE s.id = e.target_symbol_id AND s.name LIKE '%Component%'
              )
            )
        )",
        [target_id],
        |row| row.get::<_, bool>(0),
      ).unwrap_or(false)
    } else {
      connection
        .query_row(
          "SELECT EXISTS (
          SELECT 1 FROM edges e
          WHERE e.source_symbol_id = ?1
            AND e.kind = 'ModifiedWith'
            AND (
              e.unresolved_target = ?2 OR
              e.unresolved_target LIKE '%.' || ?2 OR
              EXISTS (
                SELECT 1 FROM symbols s WHERE s.id = e.target_symbol_id AND s.name = ?2
              )
            )
        )",
          (target_id, annotation_check),
          |row| row.get::<_, bool>(0),
        )
        .unwrap_or(false)
    };

    if has_annotation {
      // Check if this edge already exists
      let edge_exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM edges WHERE source_symbol_id = ?1 AND target_symbol_id = ?2 AND kind = ?3)",
        (gen_sym.id, target_id, edge_kind),
        |row| row.get(0),
      ).unwrap_or(false);

      if !edge_exists {
        insert_edge(
          connection,
          &NewEdgeRecord {
            source_symbol_id: gen_sym.id,
            target_symbol_id: Some(target_id),
            unresolved_target: None,
            kind: edge_kind.to_string(),
            confidence: Some(1.0),
          },
        )?;
        resolved_count += 1;
      }
    }
  }

  Ok(resolved_count)
}

#[allow(clippy::type_complexity)]
pub fn lookup_imports_by_file_path(
  connection: &Connection,
  repository_id: i64,
  path: &str,
) -> Result<Vec<(String, Option<String>, Option<String>)>> {
  let mut stmt = connection.prepare(
    "SELECT s.id 
     FROM symbols s
     INNER JOIN files f ON s.file_id = f.id
     WHERE f.repository_id = ?1 AND f.path = ?2 AND s.kind = 'Module'
     LIMIT 1",
  )?;
  let source_symbol_id: Option<i64> = stmt
    .query_row((repository_id, path), |row| row.get(0))
    .optional()?;

  let Some(source_id) = source_symbol_id else {
    return Ok(Vec::new());
  };

  let mut stmt = connection.prepare(
    "SELECT 
       COALESCE(s_target.qualified_name, e.unresolved_target) AS target_name,
       f_target.path AS target_file_path,
       e.unresolved_target
     FROM edges e
     LEFT JOIN symbols s_target ON e.target_symbol_id = s_target.id
     LEFT JOIN files f_target ON s_target.file_id = f_target.id
     WHERE e.source_symbol_id = ?1 AND e.kind = 'Imports'
     ORDER BY target_name ASC",
  )?;

  let rows = stmt.query_map([source_id], |row| {
    Ok((
      row.get::<_, String>(0)?,
      row.get::<_, Option<String>>(1)?,
      row.get::<_, Option<String>>(2)?,
    ))
  })?;

  rows.collect()
}

pub fn lookup_imported_by_file_path(
  connection: &Connection,
  repository_id: i64,
  path: &str,
) -> Result<Vec<(String, String)>> {
  let mut stmt = connection.prepare(
    "SELECT DISTINCT
       s_source.qualified_name,
       f_source.path
     FROM edges e
     INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
     INNER JOIN files f_source ON s_source.file_id = f_source.id
     INNER JOIN symbols s_target ON e.target_symbol_id = s_target.id
     INNER JOIN files f_target ON s_target.file_id = f_target.id
     WHERE f_target.repository_id = ?1 
       AND f_target.path = ?2 
       AND e.kind = 'Imports'
     ORDER BY s_source.qualified_name ASC",
  )?;

  let rows = stmt.query_map((repository_id, path), |row| {
    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
  })?;

  rows.collect()
}

#[allow(clippy::type_complexity)]
pub fn lookup_callers_by_symbol_name(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> Result<Vec<(String, String, String, Option<f64>)>> {
  let is_qualified = name.contains("::");

  let sql = if is_qualified {
    "SELECT DISTINCT
       s_source.qualified_name,
       s_source.kind,
       f_source.path,
       e.confidence
     FROM edges e
     INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
     INNER JOIN files f_source ON s_source.file_id = f_source.id
     LEFT JOIN symbols s_target ON e.target_symbol_id = s_target.id
     WHERE f_source.repository_id = ?1
       AND e.kind = 'Calls'
       AND (s_target.qualified_name = ?2 OR e.unresolved_target = ?2)
     ORDER BY s_source.qualified_name ASC"
  } else {
    "SELECT DISTINCT
       s_source.qualified_name,
       s_source.kind,
       f_source.path,
       e.confidence
     FROM edges e
     INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
     INNER JOIN files f_source ON s_source.file_id = f_source.id
     LEFT JOIN symbols s_target ON e.target_symbol_id = s_target.id
     WHERE f_source.repository_id = ?1
       AND e.kind = 'Calls'
       AND (
         s_target.name = ?2
         OR s_target.qualified_name = ?2
         OR e.unresolved_target = ?2
         OR e.unresolved_target LIKE '%::' || ?2
         OR e.unresolved_target LIKE '%.' || ?2
       )
     ORDER BY s_source.qualified_name ASC"
  };

  let mut stmt = connection.prepare(sql)?;
  let rows = stmt.query_map((repository_id, name), |row| {
    Ok((
      row.get::<_, String>(0)?,
      row.get::<_, String>(1)?,
      row.get::<_, String>(2)?,
      row.get::<_, Option<f64>>(3)?,
    ))
  })?;

  rows.collect()
}

#[allow(clippy::type_complexity)]
pub fn lookup_callees_by_symbol_name(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> Result<Vec<(String, Option<String>, Option<String>, Option<f64>)>> {
  let is_qualified = name.contains("::");

  let sql = if is_qualified {
    "SELECT 
       COALESCE(s_target.qualified_name, e.unresolved_target) AS target_name,
       s_target.kind AS target_kind,
       f_target.path AS target_file_path,
       e.confidence
     FROM edges e
     INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
     INNER JOIN files f_source ON s_source.file_id = f_source.id
     LEFT JOIN symbols s_target ON e.target_symbol_id = s_target.id
     LEFT JOIN files f_target ON s_target.file_id = f_target.id
     WHERE f_source.repository_id = ?1
       AND e.kind = 'Calls'
       AND s_source.qualified_name = ?2
     ORDER BY target_name ASC"
  } else {
    "SELECT 
       COALESCE(s_target.qualified_name, e.unresolved_target) AS target_name,
       s_target.kind AS target_kind,
       f_target.path AS target_file_path,
       e.confidence
     FROM edges e
     INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
     INNER JOIN files f_source ON s_source.file_id = f_source.id
     LEFT JOIN symbols s_target ON e.target_symbol_id = s_target.id
     LEFT JOIN files f_target ON s_target.file_id = f_target.id
     WHERE f_source.repository_id = ?1
       AND e.kind = 'Calls'
       AND (s_source.name = ?2 OR s_source.qualified_name = ?2)
     ORDER BY target_name ASC"
  };

  let mut stmt = connection.prepare(sql)?;
  let rows = stmt.query_map((repository_id, name), |row| {
    Ok((
      row.get::<_, String>(0)?,
      row.get::<_, Option<String>>(1)?,
      row.get::<_, Option<String>>(2)?,
      row.get::<_, Option<f64>>(3)?,
    ))
  })?;

  rows.collect()
}

#[allow(clippy::type_complexity)]
pub fn lookup_type_references_by_symbol_name(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> Result<Vec<(String, Option<String>, Option<String>, Option<f64>)>> {
  let is_qualified = name.contains("::");

  let sql = if is_qualified {
    "SELECT 
       COALESCE(s_target.qualified_name, e.unresolved_target) AS target_name,
       s_target.kind AS target_kind,
       f_target.path AS target_file_path,
       e.confidence
     FROM edges e
     INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
     INNER JOIN files f_source ON s_source.file_id = f_source.id
     LEFT JOIN symbols s_target ON e.target_symbol_id = s_target.id
     LEFT JOIN files f_target ON s_target.file_id = f_target.id
     WHERE f_source.repository_id = ?1
       AND e.kind = 'ReferencesType'
       AND s_source.qualified_name = ?2
     ORDER BY target_name ASC"
  } else {
    "SELECT 
       COALESCE(s_target.qualified_name, e.unresolved_target) AS target_name,
       s_target.kind AS target_kind,
       f_target.path AS target_file_path,
       e.confidence
     FROM edges e
     INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
     INNER JOIN files f_source ON s_source.file_id = f_source.id
     LEFT JOIN symbols s_target ON e.target_symbol_id = s_target.id
     LEFT JOIN files f_target ON s_target.file_id = f_target.id
     WHERE f_source.repository_id = ?1
       AND e.kind = 'ReferencesType'
       AND (s_source.name = ?2 OR s_source.qualified_name = ?2)
     ORDER BY target_name ASC"
  };

  let mut stmt = connection.prepare(sql)?;
  let rows = stmt.query_map((repository_id, name), |row| {
    Ok((
      row.get::<_, String>(0)?,
      row.get::<_, Option<String>>(1)?,
      row.get::<_, Option<String>>(2)?,
      row.get::<_, Option<f64>>(3)?,
    ))
  })?;

  rows.collect()
}

#[allow(clippy::type_complexity)]
pub fn lookup_type_referencers_by_symbol_name(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> Result<Vec<(String, String, String, Option<f64>)>> {
  let is_qualified = name.contains("::");

  let sql = if is_qualified {
    "SELECT DISTINCT
       s_source.qualified_name,
       s_source.kind,
       f_source.path,
       e.confidence
     FROM edges e
     INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
     INNER JOIN files f_source ON s_source.file_id = f_source.id
     LEFT JOIN symbols s_target ON e.target_symbol_id = s_target.id
     WHERE f_source.repository_id = ?1
       AND e.kind = 'ReferencesType'
       AND (
         s_target.qualified_name = ?2 
         OR e.unresolved_target = ?2 
         OR e.unresolved_target LIKE '%' || ?2 || '%'
       )
     ORDER BY s_source.qualified_name ASC"
  } else {
    "SELECT DISTINCT
       s_source.qualified_name,
       s_source.kind,
       f_source.path,
       e.confidence
     FROM edges e
     INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
     INNER JOIN files f_source ON s_source.file_id = f_source.id
     LEFT JOIN symbols s_target ON e.target_symbol_id = s_target.id
     WHERE f_source.repository_id = ?1
       AND e.kind = 'ReferencesType'
       AND (
         s_target.name = ?2
         OR s_target.qualified_name = ?2
         OR e.unresolved_target = ?2
         OR e.unresolved_target LIKE '%::' || ?2
         OR e.unresolved_target LIKE '%.' || ?2
         OR e.unresolved_target LIKE '%col::' || ?2 || '%'
         OR e.unresolved_target LIKE '%shape::' || ?2 || '%'
       )
     ORDER BY s_source.qualified_name ASC"
  };

  let mut stmt = connection.prepare(sql)?;
  let rows = stmt.query_map((repository_id, name), |row| {
    Ok((
      row.get::<_, String>(0)?,
      row.get::<_, String>(1)?,
      row.get::<_, String>(2)?,
      row.get::<_, Option<f64>>(3)?,
    ))
  })?;

  rows.collect()
}

#[allow(clippy::type_complexity)]
pub fn lookup_decorators_by_symbol_name(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> Result<Vec<(String, Option<String>, Option<String>, Option<f64>)>> {
  let is_qualified = name.contains("::");

  let sql = if is_qualified {
    "SELECT 
       COALESCE(s_target.qualified_name, e.unresolved_target) AS target_name,
       s_target.kind AS target_kind,
       f_target.path AS target_file_path,
       e.confidence
     FROM edges e
     INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
     INNER JOIN files f_source ON s_source.file_id = f_source.id
     LEFT JOIN symbols s_target ON e.target_symbol_id = s_target.id
     LEFT JOIN files f_target ON s_target.file_id = f_target.id
     WHERE f_source.repository_id = ?1
       AND e.kind = 'ModifiedWith'
       AND s_source.qualified_name = ?2
     ORDER BY target_name ASC"
  } else {
    "SELECT 
       COALESCE(s_target.qualified_name, e.unresolved_target) AS target_name,
       s_target.kind AS target_kind,
       f_target.path AS target_file_path,
       e.confidence
     FROM edges e
     INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
     INNER JOIN files f_source ON s_source.file_id = f_source.id
     LEFT JOIN symbols s_target ON e.target_symbol_id = s_target.id
     LEFT JOIN files f_target ON s_target.file_id = f_target.id
     WHERE f_source.repository_id = ?1
       AND e.kind = 'ModifiedWith'
       AND (s_source.name = ?2 OR s_source.qualified_name = ?2)
     ORDER BY target_name ASC"
  };

  let mut stmt = connection.prepare(sql)?;
  let rows = stmt.query_map((repository_id, name), |row| {
    Ok((
      row.get::<_, String>(0)?,
      row.get::<_, Option<String>>(1)?,
      row.get::<_, Option<String>>(2)?,
      row.get::<_, Option<f64>>(3)?,
    ))
  })?;

  rows.collect()
}

#[allow(clippy::type_complexity)]
pub fn lookup_decorated_symbols_by_symbol_name(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> Result<Vec<(String, String, String, Option<f64>)>> {
  let is_qualified = name.contains("::");

  let sql = if is_qualified {
    "SELECT DISTINCT
       s_source.qualified_name,
       s_source.kind,
       f_source.path,
       e.confidence
     FROM edges e
     INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
     INNER JOIN files f_source ON s_source.file_id = f_source.id
     LEFT JOIN symbols s_target ON e.target_symbol_id = s_target.id
     WHERE f_source.repository_id = ?1
       AND e.kind = 'ModifiedWith'
       AND (s_target.qualified_name = ?2 OR e.unresolved_target = ?2)
     ORDER BY s_source.qualified_name ASC"
  } else {
    "SELECT DISTINCT
       s_source.qualified_name,
       s_source.kind,
       f_source.path,
       e.confidence
     FROM edges e
     INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
     INNER JOIN files f_source ON s_source.file_id = f_source.id
     LEFT JOIN symbols s_target ON e.target_symbol_id = s_target.id
     WHERE f_source.repository_id = ?1
       AND e.kind = 'ModifiedWith'
       AND (
         s_target.name = ?2
         OR s_target.qualified_name = ?2
         OR e.unresolved_target = ?2
         OR e.unresolved_target LIKE '%::' || ?2
         OR e.unresolved_target LIKE '%.' || ?2
       )
     ORDER BY s_source.qualified_name ASC"
  };

  let mut stmt = connection.prepare(sql)?;
  let rows = stmt.query_map((repository_id, name), |row| {
    Ok((
      row.get::<_, String>(0)?,
      row.get::<_, String>(1)?,
      row.get::<_, String>(2)?,
      row.get::<_, Option<f64>>(3)?,
    ))
  })?;

  rows.collect()
}

pub fn resolve_direct_call_test_linkages(
  connection: &Connection,
  repository_id: i64,
) -> Result<usize> {
  let mut stmt = connection.prepare(
    "SELECT DISTINCT e.source_symbol_id, e.target_symbol_id
     FROM edges e
     INNER JOIN symbols s ON e.source_symbol_id = s.id
     INNER JOIN files f ON s.file_id = f.id
     INNER JOIN tests t_src ON t_src.file_id = f.id AND t_src.qualified_name = s.qualified_name
     WHERE f.repository_id = ?1
       AND t_src.kind IN ('Class', 'Function')
       AND e.kind = 'Calls'
       AND e.target_symbol_id IS NOT NULL
       AND e.target_symbol_id NOT IN (
         SELECT s2.id
         FROM symbols s2
         INNER JOIN files f2 ON s2.file_id = f2.id
         INNER JOIN tests t_tgt ON t_tgt.file_id = f2.id AND t_tgt.qualified_name = s2.qualified_name
         WHERE f2.repository_id = ?1
       )",
  )?;

  let rows = stmt.query_map([repository_id], |row| {
    Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
  })?;

  let calls: Vec<_> = rows.collect::<Result<Vec<_>, _>>()?;
  let mut inserted_count = 0;

  for (src_id, tgt_id) in calls {
    let rows_changed = connection.execute(
      "INSERT OR IGNORE INTO edges (source_symbol_id, target_symbol_id, kind, confidence)
       VALUES (?1, ?2, 'TestedBy', 1.0)",
      (src_id, tgt_id),
    )?;
    inserted_count += rows_changed;
  }

  Ok(inserted_count)
}

pub fn resolve_symbol_doc_linkages(connection: &Connection, repository_id: i64) -> Result<usize> {
  let mut sym_stmt = connection.prepare(
    "SELECT s.id, s.name, s.qualified_name
     FROM symbols s
     INNER JOIN files f ON s.file_id = f.id
     WHERE f.repository_id = ?1",
  )?;
  let sym_rows = sym_stmt.query_map([repository_id], |row| {
    Ok((
      row.get::<_, i64>(0)?,
      row.get::<_, String>(1)?,
      row.get::<_, String>(2)?,
    ))
  })?;
  let symbols: Vec<_> = sym_rows.collect::<Result<Vec<_>, _>>()?;

  let mut doc_stmt = connection.prepare(
    "SELECT d.id, d.file_id, d.title, d.body, d.start_line, d.end_line
     FROM docs d
     INNER JOIN files f ON d.file_id = f.id
     WHERE f.repository_id = ?1 AND d.source_kind = 'Markdown' AND d.symbol_id IS NULL",
  )?;
  let doc_rows = doc_stmt.query_map([repository_id], |row| {
    Ok((
      row.get::<_, i64>(0)?,
      row.get::<_, i64>(1)?,
      row.get::<_, Option<String>>(2)?,
      row.get::<_, String>(3)?,
      row.get::<_, Option<i64>>(4)?,
      row.get::<_, Option<i64>>(5)?,
    ))
  })?;
  let docs: Vec<_> = doc_rows.collect::<Result<Vec<_>, _>>()?;

  let mut links_created = 0;

  for (doc_id, file_id, doc_title, doc_body, start_line, end_line) in docs {
    let mut linked_symbols = std::collections::HashSet::new();

    if let Some(ref title) = doc_title {
      let normalized_title = title
        .to_lowercase()
        .replace([' ', '-'], "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>();

      for (sym_id, sym_name, _sym_qname) in &symbols {
        let normalized_sym_name = sym_name
          .to_lowercase()
          .replace([' ', '-'], "_")
          .chars()
          .filter(|c| c.is_alphanumeric() || *c == '_')
          .collect::<String>();

        if !normalized_title.is_empty() && normalized_title == normalized_sym_name {
          linked_symbols.insert(*sym_id);
        }
      }
    }

    for (sym_id, _sym_name, sym_qname) in &symbols {
      let dot_qname = sym_qname.replace("::", ".");
      if doc_body.contains(sym_qname) || doc_body.contains(&dot_qname) {
        linked_symbols.insert(*sym_id);
      }
    }

    let mut first = true;
    for sym_id in linked_symbols {
      if first {
        connection.execute(
          "UPDATE docs SET symbol_id = ?1 WHERE id = ?2",
          (sym_id, doc_id),
        )?;
        first = false;
      } else {
        connection.execute(
          "INSERT INTO docs (file_id, symbol_id, title, body, start_line, end_line, source_kind)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'Markdown')",
          (
            file_id,
            Some(sym_id),
            doc_title.as_deref(),
            doc_body.as_str(),
            start_line,
            end_line,
          ),
        )?;
      }
      links_created += 1;
    }
  }

  Ok(links_created)
}

#[allow(clippy::type_complexity)]
pub fn lookup_tests_for_symbol(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> Result<Vec<(String, String, Option<f64>)>> {
  let is_qualified = name.contains("::");

  let sql = if is_qualified {
    "SELECT DISTINCT
       s_source.qualified_name,
       f_source.path,
       e.confidence
     FROM edges e
     INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
     INNER JOIN files f_source ON s_source.file_id = f_source.id
     LEFT JOIN symbols s_target ON e.target_symbol_id = s_target.id
     WHERE f_source.repository_id = ?1
       AND e.kind = 'TestedBy'
       AND (s_target.qualified_name = ?2 OR e.unresolved_target = ?2)
     ORDER BY s_source.qualified_name ASC"
  } else {
    "SELECT DISTINCT
       s_source.qualified_name,
       f_source.path,
       e.confidence
     FROM edges e
     INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
     INNER JOIN files f_source ON s_source.file_id = f_source.id
     LEFT JOIN symbols s_target ON e.target_symbol_id = s_target.id
     WHERE f_source.repository_id = ?1
       AND e.kind = 'TestedBy'
       AND (
         s_target.name = ?2
         OR s_target.qualified_name = ?2
         OR e.unresolved_target = ?2
         OR e.unresolved_target LIKE '%::' || ?2
         OR e.unresolved_target LIKE '%.' || ?2
       )
     ORDER BY s_source.qualified_name ASC"
  };

  let mut stmt = connection.prepare(sql)?;
  let rows = stmt.query_map((repository_id, name), |row| {
    Ok((
      row.get::<_, String>(0)?,
      row.get::<_, String>(1)?,
      row.get::<_, Option<f64>>(2)?,
    ))
  })?;

  rows.collect()
}

pub fn lookup_docs_for_symbol(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> Result<Vec<DocRecord>> {
  let is_qualified = name.contains("::");

  let sql = if is_qualified {
    "SELECT DISTINCT
       d.id, d.file_id, d.symbol_id, d.title, d.body, d.start_line, d.end_line, d.source_kind
     FROM docs d
     INNER JOIN symbols s ON d.symbol_id = s.id
     INNER JOIN files f ON s.file_id = f.id
     WHERE f.repository_id = ?1 AND s.qualified_name = ?2
     ORDER BY d.id ASC"
  } else {
    "SELECT DISTINCT
       d.id, d.file_id, d.symbol_id, d.title, d.body, d.start_line, d.end_line, d.source_kind
     FROM docs d
     INNER JOIN symbols s ON d.symbol_id = s.id
     INNER JOIN files f ON s.file_id = f.id
     WHERE f.repository_id = ?1 AND (s.name = ?2 OR s.qualified_name = ?2)
     ORDER BY d.id ASC"
  };

  let mut stmt = connection.prepare(sql)?;
  let rows = stmt.query_map((repository_id, name), |row| {
    Ok(DocRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      symbol_id: row.get(2)?,
      title: row.get(3)?,
      body: row.get(4)?,
      start_line: row.get(5)?,
      end_line: row.get(6)?,
      source_kind: row.get(7)?,
    })
  })?;

  let mut results = rows.collect::<Result<Vec<_>, _>>()?;

  if results.is_empty() {
    let sym_opt = lookup_symbol_by_qualified_name(connection, repository_id, name)?;
    if let Some(sym) = sym_opt {
      let file_opt = get_file_by_id(connection, sym.file_id)?;
      if let Some(file) = file_opt.filter(|f| f.path.starts_with("external:")) {
        let crate_name = file.path.strip_prefix("external:").unwrap_or(&file.path);
        let mut version = "latest".to_string();
        let packages = list_cargo_packages(connection, repository_id).unwrap_or_default();
        'outer: for p in packages {
          let deps = list_cargo_dependencies(connection, p.id).unwrap_or_default();
          for d in deps {
            if d.name == crate_name {
              let req = &d.version_requirement;
              if let Some(v) = req.as_ref() {
                let clean_v = v.trim_start_matches('^').trim_start_matches('=').trim();
                if !clean_v.is_empty() {
                  version = clean_v.to_string();
                  break 'outer;
                }
              }
            }
          }
        }

        let doc_url = if name.contains("::") {
          format!(
            "https://docs.rs/{}/{}/{}/",
            crate_name,
            version,
            name.replace("::", "/")
          )
        } else {
          format!("https://docs.rs/{}/{}/{}/", crate_name, version, name)
        };

        let body = format!(
          "External documentation for `{}` is available on docs.rs:\n{}",
          name, doc_url
        );

        results.push(DocRecord {
          id: 0,
          file_id: file.id,
          symbol_id: Some(sym.id),
          title: Some("External Documentation".to_string()),
          body,
          start_line: Some(0),
          end_line: Some(0),
          source_kind: "external".to_string(),
        });
      }
    }
  }

  Ok(results)
}

fn sanitize_query_for_like(query: &str) -> String {
  // Retain '.', '_', '-', ':' so that dotted qualified names (e.g. `pkg.module.Class`)
  // and other common identifier separators survive the sanitization step intact.
  let cleaned: String = query
    .chars()
    .filter(|c| c.is_alphanumeric() || c.is_whitespace() || matches!(c, '_' | '-' | ':' | '.'))
    .collect();
  cleaned.trim().to_string()
}

pub fn search_docs(
  connection: &Connection,
  repository_id: i64,
  query: &str,
) -> Result<Vec<(DocRecord, String)>> {
  let sanitized = sanitize_query_for_like(query);
  let pattern = format!("%{sanitized}%");
  let sql = "SELECT d.id, d.file_id, d.symbol_id, d.title, d.body, d.start_line, d.end_line, d.source_kind, f.path
             FROM docs d
             INNER JOIN files f ON d.file_id = f.id
             WHERE f.repository_id = ?1 AND (d.body LIKE ?2 OR d.title LIKE ?2)
             ORDER BY f.path ASC, d.start_line ASC";

  let mut stmt = connection.prepare(sql)?;
  let rows = stmt.query_map((repository_id, &pattern), |row| {
    let doc = DocRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      symbol_id: row.get(2)?,
      title: row.get(3)?,
      body: row.get(4)?,
      start_line: row.get(5)?,
      end_line: row.get(6)?,
      source_kind: row.get(7)?,
    };
    let path = row.get::<_, String>(8)?;
    Ok((doc, path))
  })?;

  rows.collect()
}

pub fn search_docs_fts(
  connection: &Connection,
  repository_id: i64,
  query: &str,
) -> Result<Vec<(DocRecord, String)>> {
  let sql = "SELECT d.id, d.file_id, d.symbol_id, d.title, d.body, d.start_line, d.end_line, d.source_kind, f.path
             FROM docs d
             INNER JOIN files f ON d.file_id = f.id
             INNER JOIN docs_fts fts ON d.id = fts.rowid
             WHERE f.repository_id = ?1 AND docs_fts MATCH ?2
             ORDER BY rank ASC, f.path ASC, d.start_line ASC";

  let mut stmt = connection.prepare(sql)?;
  let rows = stmt.query_map((repository_id, query), |row| {
    let doc = DocRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      symbol_id: row.get(2)?,
      title: row.get(3)?,
      body: row.get(4)?,
      start_line: row.get(5)?,
      end_line: row.get(6)?,
      source_kind: row.get(7)?,
    };
    let path = row.get::<_, String>(8)?;
    Ok((doc, path))
  });

  match rows {
    Ok(rows_iter) => {
      let results: Result<Vec<(DocRecord, String)>> = rows_iter.collect();
      match results {
        Ok(res) => {
          if res.is_empty() {
            // Fallback to standard LIKE matching if no exact FTS MATCH results were returned
            search_docs(connection, repository_id, query)
          } else {
            Ok(res)
          }
        }
        Err(_) => search_docs(connection, repository_id, query),
      }
    }
    Err(_) => search_docs(connection, repository_id, query),
  }
}

pub fn search_symbols_fts(
  connection: &Connection,
  repository_id: i64,
  query: &str,
) -> Result<Vec<(SymbolRecord, String)>> {
  let sql = "SELECT s.id, s.file_id, s.name, s.qualified_name, s.kind, s.start_line, s.end_line, s.start_column, s.end_column, f.path
             FROM symbols s
             INNER JOIN files f ON s.file_id = f.id
             INNER JOIN symbols_fts fts ON s.id = fts.rowid
             WHERE f.repository_id = ?1 AND symbols_fts MATCH ?2
             ORDER BY rank ASC, f.path ASC, s.qualified_name ASC";

  let mut stmt = connection.prepare(sql)?;
  let rows = stmt.query_map((repository_id, query), |row| {
    let symbol = SymbolRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      name: row.get(2)?,
      qualified_name: row.get(3)?,
      kind: row.get(4)?,
      start_line: row.get(5)?,
      end_line: row.get(6)?,
      start_column: row.get(7)?,
      end_column: row.get(8)?,
    };
    let path = row.get::<_, String>(9)?;
    Ok((symbol, path))
  });

  match rows {
    Ok(rows_iter) => {
      let results: Result<Vec<(SymbolRecord, String)>> = rows_iter.collect();
      match results {
        Ok(res) => {
          if res.is_empty() {
            // Fallback to standard LIKE matching if no exact FTS MATCH results were returned
            fallback_search_symbols_like(connection, repository_id, query)
          } else {
            Ok(res)
          }
        }
        Err(_) => fallback_search_symbols_like(connection, repository_id, query),
      }
    }
    Err(_) => fallback_search_symbols_like(connection, repository_id, query),
  }
}

pub fn fallback_search_symbols_like(
  connection: &Connection,
  repository_id: i64,
  query: &str,
) -> Result<Vec<(SymbolRecord, String)>> {
  let sanitized = sanitize_query_for_like(query);
  let pattern = format!("%{sanitized}%");
  let sql = "SELECT s.id, s.file_id, s.name, s.qualified_name, s.kind, s.start_line, s.end_line, s.start_column, s.end_column, f.path
             FROM symbols s
             INNER JOIN files f ON s.file_id = f.id
             WHERE f.repository_id = ?1 AND (s.name LIKE ?2 OR s.qualified_name LIKE ?2)
             ORDER BY f.path ASC, s.qualified_name ASC";

  let mut stmt = connection.prepare(sql)?;
  let rows = stmt.query_map((repository_id, &pattern), |row| {
    let symbol = SymbolRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      name: row.get(2)?,
      qualified_name: row.get(3)?,
      kind: row.get(4)?,
      start_line: row.get(5)?,
      end_line: row.get(6)?,
      start_column: row.get(7)?,
      end_column: row.get(8)?,
    };
    let path = row.get::<_, String>(9)?;
    Ok((symbol, path))
  })?;

  rows.collect()
}

#[allow(clippy::type_complexity)]
pub fn lookup_test_deps(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> Result<Vec<(String, Option<String>, Option<String>, Option<f64>)>> {
  let is_qualified = name.contains("::");

  let sql = if is_qualified {
    "SELECT DISTINCT
       COALESCE(s_target.qualified_name, e.unresolved_target) AS target_name,
       s_target.kind AS target_kind,
       f_target.path AS target_file_path,
       e.confidence
     FROM edges e
     INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
     INNER JOIN files f_source ON s_source.file_id = f_source.id
     LEFT JOIN symbols s_target ON e.target_symbol_id = s_target.id
     LEFT JOIN files f_target ON s_target.file_id = f_target.id
     WHERE f_source.repository_id = ?1
       AND e.kind = 'TestedBy'
       AND s_source.qualified_name = ?2
     ORDER BY target_name ASC"
  } else {
    "SELECT DISTINCT
       COALESCE(s_target.qualified_name, e.unresolved_target) AS target_name,
       s_target.kind AS target_kind,
       f_target.path AS target_file_path,
       e.confidence
     FROM edges e
     INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
     INNER JOIN files f_source ON s_source.file_id = f_source.id
     LEFT JOIN symbols s_target ON e.target_symbol_id = s_target.id
     LEFT JOIN files f_target ON s_target.file_id = f_target.id
     WHERE f_source.repository_id = ?1
       AND e.kind = 'TestedBy'
       AND (s_source.name = ?2 OR s_source.qualified_name = ?2)
     ORDER BY target_name ASC"
  };

  let mut stmt = connection.prepare(sql)?;
  let rows = stmt.query_map((repository_id, name), |row| {
    Ok((
      row.get::<_, String>(0)?,
      row.get::<_, Option<String>>(1)?,
      row.get::<_, Option<String>>(2)?,
      row.get::<_, Option<f64>>(3)?,
    ))
  })?;

  rows.collect()
}

#[allow(clippy::type_complexity)]
pub fn lookup_fixtures_for_test(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> Result<Vec<(String, Option<String>)>> {
  let is_qualified = name.contains("::");

  let sql = if is_qualified {
    "SELECT DISTINCT
       COALESCE(s_target.qualified_name, e.unresolved_target) AS target_name,
       f_target.path AS target_file_path
     FROM edges e
     INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
     INNER JOIN files f_source ON s_source.file_id = f_source.id
     LEFT JOIN symbols s_target ON e.target_symbol_id = s_target.id
     LEFT JOIN files f_target ON s_target.file_id = f_target.id
     WHERE f_source.repository_id = ?1
       AND e.kind = 'ConfiguredBy'
       AND s_source.qualified_name = ?2
     ORDER BY target_name ASC"
  } else {
    "SELECT DISTINCT
       COALESCE(s_target.qualified_name, e.unresolved_target) AS target_name,
       f_target.path AS target_file_path
     FROM edges e
     INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
     INNER JOIN files f_source ON s_source.file_id = f_source.id
     LEFT JOIN symbols s_target ON e.target_symbol_id = s_target.id
     LEFT JOIN files f_target ON s_target.file_id = f_target.id
     WHERE f_source.repository_id = ?1
       AND e.kind = 'ConfiguredBy'
       AND (s_source.name = ?2 OR s_source.qualified_name = ?2)
     ORDER BY target_name ASC"
  };

  let mut stmt = connection.prepare(sql)?;
  let rows = stmt.query_map((repository_id, name), |row| {
    Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
  })?;

  rows.collect()
}

pub fn get_route_by_id(connection: &Connection, id: i64) -> Result<Option<RouteRecord>> {
  let mut stmt = connection.prepare(
    "SELECT id, file_id, symbol_id, handler_name, qualified_name, method, path, response_model, start_line, end_line
     FROM routes
     WHERE id = ?1",
  )?;
  stmt
    .query_row([id], |row| {
      Ok(RouteRecord {
        id: row.get(0)?,
        file_id: row.get(1)?,
        symbol_id: row.get(2)?,
        handler_name: row.get(3)?,
        qualified_name: row.get(4)?,
        method: row.get(5)?,
        path: row.get(6)?,
        response_model: row.get(7)?,
        start_line: row.get(8)?,
        end_line: row.get(9)?,
      })
    })
    .optional()
}

pub fn insert_route(connection: &Connection, route: &NewRouteRecord) -> Result<RouteRecord> {
  connection.execute(
    "INSERT INTO routes (file_id, symbol_id, handler_name, qualified_name, method, path, response_model, start_line, end_line)
      VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
      ON CONFLICT(file_id, qualified_name, method, path) DO UPDATE SET
        symbol_id = excluded.symbol_id,
        handler_name = excluded.handler_name,
        response_model = excluded.response_model,
        start_line = excluded.start_line,
        end_line = excluded.end_line,
        updated_at = CURRENT_TIMESTAMP",
    (
      route.file_id,
      route.symbol_id,
      route.handler_name.as_str(),
      route.qualified_name.as_str(),
      route.method.as_str(),
      route.path.as_str(),
      route.response_model.as_deref(),
      route.start_line,
      route.end_line,
    ),
  )?;

  let id: i64 = connection.query_row(
    "SELECT id FROM routes
     WHERE file_id = ?1 AND qualified_name = ?2 AND method = ?3 AND path = ?4",
    (
      route.file_id,
      route.qualified_name.as_str(),
      route.method.as_str(),
      route.path.as_str(),
    ),
    |row| row.get(0),
  )?;

  get_route_by_id(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn insert_android_component(
  connection: &Connection,
  comp: &NewAndroidComponentRecord,
) -> Result<AndroidComponentRecord> {
  let actions_str = comp.intent_actions.join(",");
  let categories_str = comp.intent_categories.join(",");
  connection.execute(
    "INSERT INTO android_components (file_id, name, component_type, class_name, permission, intent_actions, intent_categories, start_line, end_line)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
     ON CONFLICT(file_id, class_name) DO UPDATE SET
       name = excluded.name,
       component_type = excluded.component_type,
       permission = excluded.permission,
       intent_actions = excluded.intent_actions,
       intent_categories = excluded.intent_categories,
       start_line = excluded.start_line,
       end_line = excluded.end_line",
    (
      comp.file_id,
      &comp.name,
      &comp.component_type,
      &comp.class_name,
      comp.permission.as_deref(),
      actions_str,
      categories_str,
      comp.start_line,
      comp.end_line,
    ),
  )?;

  let id: i64 = connection.query_row(
    "SELECT id FROM android_components WHERE file_id = ?1 AND class_name = ?2",
    (comp.file_id, &comp.class_name),
    |row| row.get(0),
  )?;

  Ok(AndroidComponentRecord {
    id,
    file_id: comp.file_id,
    name: comp.name.clone(),
    component_type: comp.component_type.clone(),
    class_name: comp.class_name.clone(),
    permission: comp.permission.clone(),
    intent_actions: comp.intent_actions.clone(),
    intent_categories: comp.intent_categories.clone(),
    start_line: comp.start_line,
    end_line: comp.end_line,
  })
}

pub fn lookup_android_components_for_repository(
  connection: &Connection,
  repository_id: i64,
) -> Result<Vec<AndroidComponentRecord>> {
  let mut stmt = connection.prepare(
    "SELECT ac.id, ac.file_id, ac.name, ac.component_type, ac.class_name, ac.permission, ac.intent_actions, ac.intent_categories, ac.start_line, ac.end_line
     FROM android_components ac
     INNER JOIN files f ON ac.file_id = f.id
     WHERE f.repository_id = ?1
     ORDER BY ac.name ASC",
  )?;

  let rows = stmt.query_map([repository_id], |row| {
    let actions_str: String = row.get(6)?;
    let categories_str: String = row.get(7)?;
    let intent_actions = if actions_str.is_empty() {
      vec![]
    } else {
      actions_str.split(',').map(String::from).collect()
    };
    let intent_categories = if categories_str.is_empty() {
      vec![]
    } else {
      categories_str.split(',').map(String::from).collect()
    };

    Ok(AndroidComponentRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      name: row.get(2)?,
      component_type: row.get(3)?,
      class_name: row.get(4)?,
      permission: row.get(5)?,
      intent_actions,
      intent_categories,
      start_line: row.get(8)?,
      end_line: row.get(9)?,
    })
  })?;

  rows.collect()
}

pub fn insert_android_resource(
  connection: &Connection,
  res: &NewAndroidResourceRecord,
) -> Result<AndroidResourceRecord> {
  connection.execute(
    "INSERT INTO android_resources (file_id, name, resource_type, value, start_line, end_line)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
     ON CONFLICT(file_id, name, resource_type) DO UPDATE SET
       value = excluded.value,
       start_line = excluded.start_line,
       end_line = excluded.end_line",
    (
      res.file_id,
      &res.name,
      &res.resource_type,
      res.value.as_deref(),
      res.start_line,
      res.end_line,
    ),
  )?;

  let id: i64 = connection.query_row(
    "SELECT id FROM android_resources WHERE file_id = ?1 AND name = ?2 AND resource_type = ?3",
    (res.file_id, &res.name, &res.resource_type),
    |row| row.get(0),
  )?;

  Ok(AndroidResourceRecord {
    id,
    file_id: res.file_id,
    name: res.name.clone(),
    resource_type: res.resource_type.clone(),
    value: res.value.clone(),
    start_line: res.start_line,
    end_line: res.end_line,
  })
}

pub fn lookup_android_resources_for_repository(
  connection: &Connection,
  repository_id: i64,
) -> Result<Vec<AndroidResourceRecord>> {
  let mut stmt = connection.prepare(
    "SELECT ar.id, ar.file_id, ar.name, ar.resource_type, ar.value, ar.start_line, ar.end_line
     FROM android_resources ar
     INNER JOIN files f ON ar.file_id = f.id
     WHERE f.repository_id = ?1
     ORDER BY ar.name ASC",
  )?;

  let rows = stmt.query_map([repository_id], |row| {
    Ok(AndroidResourceRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      name: row.get(2)?,
      resource_type: row.get(3)?,
      value: row.get(4)?,
      start_line: row.get(5)?,
      end_line: row.get(6)?,
    })
  })?;

  rows.collect()
}

pub fn lookup_routes_by_file_id(connection: &Connection, file_id: i64) -> Result<Vec<RouteRecord>> {
  let mut stmt = connection.prepare(
    "SELECT id, file_id, symbol_id, handler_name, qualified_name, method, path, response_model, start_line, end_line
     FROM routes
     WHERE file_id = ?1
     ORDER BY path ASC, method ASC",
  )?;
  let rows = stmt.query_map([file_id], |row| {
    Ok(RouteRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      symbol_id: row.get(2)?,
      handler_name: row.get(3)?,
      qualified_name: row.get(4)?,
      method: row.get(5)?,
      path: row.get(6)?,
      response_model: row.get(7)?,
      start_line: row.get(8)?,
      end_line: row.get(9)?,
    })
  })?;
  rows.collect()
}

pub fn lookup_routes_for_repository(
  connection: &Connection,
  repository_id: i64,
) -> Result<Vec<RouteRecord>> {
  let mut stmt = connection.prepare(
    "SELECT r.id, r.file_id, r.symbol_id, r.handler_name, r.qualified_name, r.method, r.path, r.response_model, r.start_line, r.end_line
     FROM routes r
     INNER JOIN files f ON r.file_id = f.id
     WHERE f.repository_id = ?1
     ORDER BY r.path ASC, r.method ASC",
  )?;
  let rows = stmt.query_map([repository_id], |row| {
    Ok(RouteRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      symbol_id: row.get(2)?,
      handler_name: row.get(3)?,
      qualified_name: row.get(4)?,
      method: row.get(5)?,
      path: row.get(6)?,
      response_model: row.get(7)?,
      start_line: row.get(8)?,
      end_line: row.get(9)?,
    })
  })?;
  rows.collect()
}

pub fn insert_pydantic_model(
  connection: &Connection,
  model: &NewPydanticModelRecord,
) -> Result<PydanticModelRecord> {
  connection.execute(
    "INSERT INTO pydantic_models (file_id, symbol_id, name, qualified_name, start_line, end_line)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
     ON CONFLICT(file_id, qualified_name) DO UPDATE SET
       symbol_id = excluded.symbol_id,
       name = excluded.name,
       start_line = excluded.start_line,
       end_line = excluded.end_line,
       updated_at = CURRENT_TIMESTAMP",
    (
      model.file_id,
      model.symbol_id,
      &model.name,
      &model.qualified_name,
      model.start_line,
      model.end_line,
    ),
  )?;

  let id: i64 = connection.query_row(
    "SELECT id FROM pydantic_models WHERE file_id = ?1 AND qualified_name = ?2",
    (model.file_id, &model.qualified_name),
    |row| row.get(0),
  )?;

  Ok(PydanticModelRecord {
    id,
    file_id: model.file_id,
    symbol_id: model.symbol_id,
    name: model.name.clone(),
    qualified_name: model.qualified_name.clone(),
    start_line: model.start_line,
    end_line: model.end_line,
  })
}

pub fn insert_pydantic_field(
  connection: &Connection,
  field: &NewPydanticFieldRecord,
) -> Result<PydanticFieldRecord> {
  connection.execute(
    "INSERT INTO pydantic_fields (model_id, name, type_annotation, default_value, is_required)
     VALUES (?1, ?2, ?3, ?4, ?5)
     ON CONFLICT(model_id, name) DO UPDATE SET
       type_annotation = excluded.type_annotation,
       default_value = excluded.default_value,
       is_required = excluded.is_required",
    (
      field.model_id,
      &field.name,
      &field.type_annotation,
      &field.default_value,
      field.is_required as i32,
    ),
  )?;

  let id: i64 = connection.query_row(
    "SELECT id FROM pydantic_fields WHERE model_id = ?1 AND name = ?2",
    (field.model_id, &field.name),
    |row| row.get(0),
  )?;

  Ok(PydanticFieldRecord {
    id,
    model_id: field.model_id,
    name: field.name.clone(),
    type_annotation: field.type_annotation.clone(),
    default_value: field.default_value.clone(),
    is_required: field.is_required,
  })
}

pub fn insert_pydantic_validator(
  connection: &Connection,
  validator: &NewPydanticValidatorRecord,
) -> Result<PydanticValidatorRecord> {
  connection.execute(
    "INSERT INTO pydantic_validators (model_id, name, validator_type, target_fields)
     VALUES (?1, ?2, ?3, ?4)
     ON CONFLICT(model_id, name) DO UPDATE SET
       validator_type = excluded.validator_type,
       target_fields = excluded.target_fields",
    (
      validator.model_id,
      &validator.name,
      &validator.validator_type,
      &validator.target_fields,
    ),
  )?;

  let id: i64 = connection.query_row(
    "SELECT id FROM pydantic_validators WHERE model_id = ?1 AND name = ?2",
    (validator.model_id, &validator.name),
    |row| row.get(0),
  )?;

  Ok(PydanticValidatorRecord {
    id,
    model_id: validator.model_id,
    name: validator.name.clone(),
    validator_type: validator.validator_type.clone(),
    target_fields: validator.target_fields.clone(),
  })
}

pub fn insert_cargo_package(
  connection: &Connection,
  pkg: &NewCargoPackageRecord,
) -> Result<CargoPackageRecord> {
  connection.execute(
    "INSERT INTO cargo_packages (repository_id, name, manifest_path, version, is_workspace_member)
     VALUES (?1, ?2, ?3, ?4, ?5)
     ON CONFLICT(repository_id, name, manifest_path) DO UPDATE SET
       version = excluded.version,
       is_workspace_member = excluded.is_workspace_member",
    (
      pkg.repository_id,
      &pkg.name,
      &pkg.manifest_path,
      &pkg.version,
      if pkg.is_workspace_member { 1 } else { 0 },
    ),
  )?;

  let id: i64 = connection.query_row(
    "SELECT id FROM cargo_packages WHERE repository_id = ?1 AND name = ?2 AND manifest_path = ?3",
    (pkg.repository_id, &pkg.name, &pkg.manifest_path),
    |row| row.get(0),
  )?;

  Ok(CargoPackageRecord {
    id,
    repository_id: pkg.repository_id,
    name: pkg.name.clone(),
    manifest_path: pkg.manifest_path.clone(),
    version: pkg.version.clone(),
    is_workspace_member: pkg.is_workspace_member,
  })
}

pub fn insert_cargo_dependency(
  connection: &Connection,
  dep: &NewCargoDependencyRecord,
) -> Result<CargoDependencyRecord> {
  connection.execute(
    "INSERT INTO cargo_dependencies (package_id, name, version_requirement, is_workspace_dependency, features, is_dev)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
     ON CONFLICT(package_id, name) DO UPDATE SET
       version_requirement = excluded.version_requirement,
       is_workspace_dependency = excluded.is_workspace_dependency,
       features = excluded.features,
       is_dev = excluded.is_dev",
    (
      dep.package_id,
      &dep.name,
      &dep.version_requirement,
      if dep.is_workspace_dependency { 1 } else { 0 },
      &dep.features,
      if dep.is_dev { 1 } else { 0 },
    ),
  )?;

  let id: i64 = connection.query_row(
    "SELECT id FROM cargo_dependencies WHERE package_id = ?1 AND name = ?2",
    (dep.package_id, &dep.name),
    |row| row.get(0),
  )?;

  Ok(CargoDependencyRecord {
    id,
    package_id: dep.package_id,
    name: dep.name.clone(),
    version_requirement: dep.version_requirement.clone(),
    is_workspace_dependency: dep.is_workspace_dependency,
    features: dep.features.clone(),
    is_dev: dep.is_dev,
  })
}

pub fn delete_cargo_dependencies_for_package(
  connection: &Connection,
  package_id: i64,
) -> Result<()> {
  connection.execute(
    "DELETE FROM cargo_dependencies WHERE package_id = ?1",
    [package_id],
  )?;
  Ok(())
}

pub fn list_cargo_packages(
  connection: &Connection,
  repository_id: i64,
) -> Result<Vec<CargoPackageRecord>> {
  let mut stmt = connection.prepare(
    "SELECT id, repository_id, name, manifest_path, version, is_workspace_member
     FROM cargo_packages
     WHERE repository_id = ?1
     ORDER BY name ASC",
  )?;
  let rows = stmt.query_map([repository_id], |row| {
    let is_ws_int: i64 = row.get(5)?;
    Ok(CargoPackageRecord {
      id: row.get(0)?,
      repository_id: row.get(1)?,
      name: row.get(2)?,
      manifest_path: row.get(3)?,
      version: row.get(4)?,
      is_workspace_member: is_ws_int != 0,
    })
  })?;
  rows.collect()
}

pub fn list_cargo_dependencies(
  connection: &Connection,
  package_id: i64,
) -> Result<Vec<CargoDependencyRecord>> {
  let mut stmt = connection.prepare(
    "SELECT id, package_id, name, version_requirement, is_workspace_dependency, features, is_dev
     FROM cargo_dependencies
     WHERE package_id = ?1
     ORDER BY name ASC",
  )?;
  let rows = stmt.query_map([package_id], |row| {
    let is_ws_int: i64 = row.get(4)?;
    let is_dev_int: i64 = row.get(6)?;
    Ok(CargoDependencyRecord {
      id: row.get(0)?,
      package_id: row.get(1)?,
      name: row.get(2)?,
      version_requirement: row.get(3)?,
      is_workspace_dependency: is_ws_int != 0,
      features: row.get(5)?,
      is_dev: is_dev_int != 0,
    })
  })?;
  rows.collect()
}

pub fn insert_fsharp_project(
  connection: &Connection,
  proj: &NewFSharpProjectRecord,
) -> Result<FSharpProjectRecord> {
  connection.execute(
    "INSERT INTO fsharp_projects (repository_id, name, project_path, target_framework, is_solution_member)
     VALUES (?1, ?2, ?3, ?4, ?5)
     ON CONFLICT(repository_id, name, project_path) DO UPDATE SET
       target_framework = excluded.target_framework,
       is_solution_member = excluded.is_solution_member",
    (
      proj.repository_id,
      &proj.name,
      &proj.project_path,
      &proj.target_framework,
      if proj.is_solution_member { 1 } else { 0 },
    ),
  )?;

  let id: i64 = connection.query_row(
    "SELECT id FROM fsharp_projects WHERE repository_id = ?1 AND name = ?2 AND project_path = ?3",
    (proj.repository_id, &proj.name, &proj.project_path),
    |row| row.get(0),
  )?;

  Ok(FSharpProjectRecord {
    id,
    repository_id: proj.repository_id,
    name: proj.name.clone(),
    project_path: proj.project_path.clone(),
    target_framework: proj.target_framework.clone(),
    is_solution_member: proj.is_solution_member,
  })
}

pub fn insert_fsharp_dependency(
  connection: &Connection,
  dep: &NewFSharpDependencyRecord,
) -> Result<FSharpDependencyRecord> {
  connection.execute(
    "INSERT INTO fsharp_dependencies (project_id, name, dependency_type, version_requirement)
     VALUES (?1, ?2, ?3, ?4)
     ON CONFLICT(project_id, name, dependency_type) DO UPDATE SET
       version_requirement = excluded.version_requirement",
    (
      dep.project_id,
      &dep.name,
      &dep.dependency_type,
      &dep.version_requirement,
    ),
  )?;

  let id: i64 = connection.query_row(
    "SELECT id FROM fsharp_dependencies WHERE project_id = ?1 AND name = ?2 AND dependency_type = ?3",
    (dep.project_id, &dep.name, &dep.dependency_type),
    |row| row.get(0),
  )?;

  Ok(FSharpDependencyRecord {
    id,
    project_id: dep.project_id,
    name: dep.name.clone(),
    dependency_type: dep.dependency_type.clone(),
    version_requirement: dep.version_requirement.clone(),
  })
}

pub fn delete_fsharp_dependencies_for_project(
  connection: &Connection,
  project_id: i64,
) -> Result<()> {
  connection.execute(
    "DELETE FROM fsharp_dependencies WHERE project_id = ?1",
    [project_id],
  )?;
  Ok(())
}

/// Removes any `fsharp_projects` rows for `repository_id` whose `project_path` is
/// not present in `current_paths`. Dependencies cascade automatically.
pub fn delete_fsharp_projects_not_in_paths(
  connection: &Connection,
  repository_id: i64,
  current_paths: &[String],
) -> Result<()> {
  let existing: Vec<(i64, String)> = {
    let mut stmt = connection
      .prepare("SELECT id, project_path FROM fsharp_projects WHERE repository_id = ?1")?;
    let rows = stmt.query_map([repository_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect::<Result<Vec<_>>>()?
  };

  for (id, path) in existing {
    if !current_paths.contains(&path) {
      connection.execute("DELETE FROM fsharp_projects WHERE id = ?1", [id])?;
    }
  }
  Ok(())
}

pub fn list_fsharp_projects(
  connection: &Connection,
  repository_id: i64,
) -> Result<Vec<FSharpProjectRecord>> {
  let mut stmt = connection.prepare(
    "SELECT id, repository_id, name, project_path, target_framework, is_solution_member
     FROM fsharp_projects
     WHERE repository_id = ?1
     ORDER BY name ASC",
  )?;
  let rows = stmt.query_map([repository_id], |row| {
    let is_sol_int: i64 = row.get(5)?;
    Ok(FSharpProjectRecord {
      id: row.get(0)?,
      repository_id: row.get(1)?,
      name: row.get(2)?,
      project_path: row.get(3)?,
      target_framework: row.get(4)?,
      is_solution_member: is_sol_int != 0,
    })
  })?;
  rows.collect()
}

pub fn list_fsharp_dependencies(
  connection: &Connection,
  project_id: i64,
) -> Result<Vec<FSharpDependencyRecord>> {
  let mut stmt = connection.prepare(
    "SELECT id, project_id, name, dependency_type, version_requirement
     FROM fsharp_dependencies
     WHERE project_id = ?1
     ORDER BY name ASC",
  )?;
  let rows = stmt.query_map([project_id], |row| {
    Ok(FSharpDependencyRecord {
      id: row.get(0)?,
      project_id: row.get(1)?,
      name: row.get(2)?,
      dependency_type: row.get(3)?,
      version_requirement: row.get(4)?,
    })
  })?;
  rows.collect()
}

pub fn insert_kotlin_project(
  connection: &Connection,
  proj: &NewKotlinProjectRecord,
) -> Result<KotlinProjectRecord> {
  connection.execute(
    "INSERT INTO kotlin_projects (repository_id, name, project_path, target_framework, is_solution_member, generated_source_dirs)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
     ON CONFLICT(repository_id, name, project_path) DO UPDATE SET
       target_framework = excluded.target_framework,
       is_solution_member = excluded.is_solution_member,
       generated_source_dirs = excluded.generated_source_dirs",
    (
      proj.repository_id,
      &proj.name,
      &proj.project_path,
      &proj.target_framework,
      if proj.is_solution_member { 1 } else { 0 },
      &proj.generated_source_dirs,
    ),
  )?;

  let (id, generated_source_dirs): (i64, Option<String>) = connection.query_row(
    "SELECT id, generated_source_dirs FROM kotlin_projects WHERE repository_id = ?1 AND name = ?2 AND project_path = ?3",
    (proj.repository_id, &proj.name, &proj.project_path),
    |row| Ok((row.get(0)?, row.get(1)?)),
  )?;

  Ok(KotlinProjectRecord {
    id,
    repository_id: proj.repository_id,
    name: proj.name.clone(),
    project_path: proj.project_path.clone(),
    target_framework: proj.target_framework.clone(),
    is_solution_member: proj.is_solution_member,
    generated_source_dirs,
  })
}

pub fn insert_kotlin_dependency(
  connection: &Connection,
  dep: &NewKotlinDependencyRecord,
) -> Result<KotlinDependencyRecord> {
  connection.execute(
    "INSERT INTO kotlin_dependencies (project_id, name, dependency_type, version_requirement)
     VALUES (?1, ?2, ?3, ?4)
     ON CONFLICT(project_id, name, dependency_type) DO UPDATE SET
       version_requirement = excluded.version_requirement",
    (
      dep.project_id,
      &dep.name,
      &dep.dependency_type,
      &dep.version_requirement,
    ),
  )?;

  let id: i64 = connection.query_row(
    "SELECT id FROM kotlin_dependencies WHERE project_id = ?1 AND name = ?2 AND dependency_type = ?3",
    (dep.project_id, &dep.name, &dep.dependency_type),
    |row| row.get(0),
  )?;

  Ok(KotlinDependencyRecord {
    id,
    project_id: dep.project_id,
    name: dep.name.clone(),
    dependency_type: dep.dependency_type.clone(),
    version_requirement: dep.version_requirement.clone(),
  })
}

pub fn delete_kotlin_dependencies_for_project(
  connection: &Connection,
  project_id: i64,
) -> Result<()> {
  connection.execute(
    "DELETE FROM kotlin_dependencies WHERE project_id = ?1",
    [project_id],
  )?;
  Ok(())
}

pub fn delete_kotlin_projects_not_in_paths(
  connection: &Connection,
  repository_id: i64,
  current_paths: &[String],
) -> Result<()> {
  let existing: Vec<(i64, String)> = {
    let mut stmt = connection
      .prepare("SELECT id, project_path FROM kotlin_projects WHERE repository_id = ?1")?;
    let rows = stmt.query_map([repository_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect::<Result<Vec<_>>>()?
  };

  for (id, path) in existing {
    if !current_paths.contains(&path) {
      connection.execute("DELETE FROM kotlin_projects WHERE id = ?1", [id])?;
    }
  }
  Ok(())
}

pub fn list_kotlin_projects(
  connection: &Connection,
  repository_id: i64,
) -> Result<Vec<KotlinProjectRecord>> {
  let mut stmt = connection.prepare(
    "SELECT id, repository_id, name, project_path, target_framework, is_solution_member, generated_source_dirs
     FROM kotlin_projects
     WHERE repository_id = ?1
     ORDER BY name ASC",
  )?;
  let rows = stmt.query_map([repository_id], |row| {
    let is_sol_int: i64 = row.get(5)?;
    Ok(KotlinProjectRecord {
      id: row.get(0)?,
      repository_id: row.get(1)?,
      name: row.get(2)?,
      project_path: row.get(3)?,
      target_framework: row.get(4)?,
      is_solution_member: is_sol_int != 0,
      generated_source_dirs: row.get(6)?,
    })
  })?;
  rows.collect()
}

pub fn list_kotlin_dependencies(
  connection: &Connection,
  project_id: i64,
) -> Result<Vec<KotlinDependencyRecord>> {
  let mut stmt = connection.prepare(
    "SELECT id, project_id, name, dependency_type, version_requirement
     FROM kotlin_dependencies
     WHERE project_id = ?1
     ORDER BY name ASC",
  )?;
  let rows = stmt.query_map([project_id], |row| {
    Ok(KotlinDependencyRecord {
      id: row.get(0)?,
      project_id: row.get(1)?,
      name: row.get(2)?,
      dependency_type: row.get(3)?,
      version_requirement: row.get(4)?,
    })
  })?;
  rows.collect()
}

pub fn insert_swift_project(
  connection: &Connection,
  proj: &NewSwiftProjectRecord,
) -> Result<SwiftProjectRecord> {
  connection.execute(
    "INSERT INTO swift_projects (repository_id, name, project_path, target_framework, is_solution_member)
     VALUES (?1, ?2, ?3, ?4, ?5)
     ON CONFLICT(repository_id, name, project_path) DO UPDATE SET
       target_framework = excluded.target_framework,
       is_solution_member = excluded.is_solution_member",
    (
      proj.repository_id,
      &proj.name,
      &proj.project_path,
      &proj.target_framework,
      if proj.is_solution_member { 1 } else { 0 },
    ),
  )?;

  let id: i64 = connection.query_row(
    "SELECT id FROM swift_projects WHERE repository_id = ?1 AND name = ?2 AND project_path = ?3",
    (proj.repository_id, &proj.name, &proj.project_path),
    |row| row.get(0),
  )?;

  Ok(SwiftProjectRecord {
    id,
    repository_id: proj.repository_id,
    name: proj.name.clone(),
    project_path: proj.project_path.clone(),
    target_framework: proj.target_framework.clone(),
    is_solution_member: proj.is_solution_member,
  })
}

pub fn insert_swift_dependency(
  connection: &Connection,
  dep: &NewSwiftDependencyRecord,
) -> Result<SwiftDependencyRecord> {
  connection.execute(
    "INSERT INTO swift_dependencies (project_id, name, dependency_type, version_requirement)
     VALUES (?1, ?2, ?3, ?4)
     ON CONFLICT(project_id, name, dependency_type) DO UPDATE SET
       version_requirement = excluded.version_requirement",
    (
      dep.project_id,
      &dep.name,
      &dep.dependency_type,
      &dep.version_requirement,
    ),
  )?;

  let id: i64 = connection.query_row(
    "SELECT id FROM swift_dependencies WHERE project_id = ?1 AND name = ?2 AND dependency_type = ?3",
    (dep.project_id, &dep.name, &dep.dependency_type),
    |row| row.get(0),
  )?;

  Ok(SwiftDependencyRecord {
    id,
    project_id: dep.project_id,
    name: dep.name.clone(),
    dependency_type: dep.dependency_type.clone(),
    version_requirement: dep.version_requirement.clone(),
  })
}

pub fn delete_swift_dependencies_for_project(
  connection: &Connection,
  project_id: i64,
) -> Result<()> {
  connection.execute(
    "DELETE FROM swift_dependencies WHERE project_id = ?1",
    [project_id],
  )?;
  Ok(())
}

pub fn delete_swift_projects_not_in_paths(
  connection: &Connection,
  repository_id: i64,
  current_paths: &[String],
) -> Result<()> {
  let existing: Vec<(i64, String)> = {
    let mut stmt =
      connection.prepare("SELECT id, project_path FROM swift_projects WHERE repository_id = ?1")?;
    let rows = stmt.query_map([repository_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect::<Result<Vec<_>>>()?
  };

  for (id, path) in existing {
    if !current_paths.contains(&path) {
      connection.execute("DELETE FROM swift_projects WHERE id = ?1", [id])?;
    }
  }
  Ok(())
}

pub fn list_swift_projects(
  connection: &Connection,
  repository_id: i64,
) -> Result<Vec<SwiftProjectRecord>> {
  let mut stmt = connection.prepare(
    "SELECT id, repository_id, name, project_path, target_framework, is_solution_member
     FROM swift_projects
     WHERE repository_id = ?1
     ORDER BY name ASC",
  )?;
  let rows = stmt.query_map([repository_id], |row| {
    let is_sol_int: i64 = row.get(5)?;
    Ok(SwiftProjectRecord {
      id: row.get(0)?,
      repository_id: row.get(1)?,
      name: row.get(2)?,
      project_path: row.get(3)?,
      target_framework: row.get(4)?,
      is_solution_member: is_sol_int != 0,
    })
  })?;
  rows.collect()
}

pub fn list_swift_dependencies(
  connection: &Connection,
  project_id: i64,
) -> Result<Vec<SwiftDependencyRecord>> {
  let mut stmt = connection.prepare(
    "SELECT id, project_id, name, dependency_type, version_requirement
     FROM swift_dependencies
     WHERE project_id = ?1
     ORDER BY name ASC",
  )?;
  let rows = stmt.query_map([project_id], |row| {
    Ok(SwiftDependencyRecord {
      id: row.get(0)?,
      project_id: row.get(1)?,
      name: row.get(2)?,
      dependency_type: row.get(3)?,
      version_requirement: row.get(4)?,
    })
  })?;
  rows.collect()
}

pub fn lookup_pydantic_models_for_repository(
  connection: &Connection,
  repository_id: i64,
) -> Result<Vec<PydanticModelRecord>> {
  let mut stmt = connection.prepare(
    "SELECT m.id, m.file_id, m.symbol_id, m.name, m.qualified_name, m.start_line, m.end_line
     FROM pydantic_models m
     INNER JOIN files f ON m.file_id = f.id
     WHERE f.repository_id = ?1
     ORDER BY m.qualified_name ASC",
  )?;
  let rows = stmt.query_map([repository_id], |row| {
    Ok(PydanticModelRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      symbol_id: row.get(2)?,
      name: row.get(3)?,
      qualified_name: row.get(4)?,
      start_line: row.get(5)?,
      end_line: row.get(6)?,
    })
  })?;
  rows.collect()
}

pub fn lookup_pydantic_model_by_name(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> Result<Option<PydanticModelRecord>> {
  let mut stmt = connection.prepare(
    "SELECT m.id, m.file_id, m.symbol_id, m.name, m.qualified_name, m.start_line, m.end_line
     FROM pydantic_models m
     INNER JOIN files f ON m.file_id = f.id
     WHERE f.repository_id = ?1 AND (m.name = ?2 OR m.qualified_name = ?2)
     LIMIT 1",
  )?;
  stmt
    .query_row((repository_id, name), |row| {
      Ok(PydanticModelRecord {
        id: row.get(0)?,
        file_id: row.get(1)?,
        symbol_id: row.get(2)?,
        name: row.get(3)?,
        qualified_name: row.get(4)?,
        start_line: row.get(5)?,
        end_line: row.get(6)?,
      })
    })
    .optional()
}

pub fn lookup_pydantic_fields_for_model(
  connection: &Connection,
  model_id: i64,
) -> Result<Vec<PydanticFieldRecord>> {
  let mut stmt = connection.prepare(
    "SELECT id, model_id, name, type_annotation, default_value, is_required
     FROM pydantic_fields
     WHERE model_id = ?1
     ORDER BY id ASC",
  )?;
  let rows = stmt.query_map([model_id], |row| {
    let is_req_int: i32 = row.get(5)?;
    Ok(PydanticFieldRecord {
      id: row.get(0)?,
      model_id: row.get(1)?,
      name: row.get(2)?,
      type_annotation: row.get(3)?,
      default_value: row.get(4)?,
      is_required: is_req_int != 0,
    })
  })?;
  rows.collect()
}

pub fn lookup_pydantic_validators_for_model(
  connection: &Connection,
  model_id: i64,
) -> Result<Vec<PydanticValidatorRecord>> {
  let mut stmt = connection.prepare(
    "SELECT id, model_id, name, validator_type, target_fields
     FROM pydantic_validators
     WHERE model_id = ?1
     ORDER BY id ASC",
  )?;
  let rows = stmt.query_map([model_id], |row| {
    Ok(PydanticValidatorRecord {
      id: row.get(0)?,
      model_id: row.get(1)?,
      name: row.get(2)?,
      validator_type: row.get(3)?,
      target_fields: row.get(4)?,
    })
  })?;
  rows.collect()
}

pub fn insert_concurrency_spawn(
  connection: &Connection,
  spawn: &NewConcurrencySpawnRecord,
) -> Result<ConcurrencySpawnRecord> {
  connection.execute(
    "INSERT INTO concurrency_spawns (file_id, source_symbol_qualified_name, spawn_kind, target_name, start_line, end_line)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    (
      spawn.file_id,
      &spawn.source_symbol_qualified_name,
      &spawn.spawn_kind,
      &spawn.target_name,
      spawn.start_line,
      spawn.end_line,
    ),
  )?;

  let id = connection.last_insert_rowid();
  Ok(ConcurrencySpawnRecord {
    id,
    file_id: spawn.file_id,
    source_symbol_qualified_name: spawn.source_symbol_qualified_name.clone(),
    spawn_kind: spawn.spawn_kind.clone(),
    target_name: spawn.target_name.clone(),
    start_line: spawn.start_line,
    end_line: spawn.end_line,
  })
}

pub fn insert_concurrency_channel(
  connection: &Connection,
  channel: &NewConcurrencyChannelRecord,
) -> Result<ConcurrencyChannelRecord> {
  connection.execute(
    "INSERT INTO concurrency_channels (file_id, source_symbol_qualified_name, channel_kind, tx_name, rx_name, start_line, end_line)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    (
      channel.file_id,
      &channel.source_symbol_qualified_name,
      &channel.channel_kind,
      &channel.tx_name,
      &channel.rx_name,
      channel.start_line,
      channel.end_line,
    ),
  )?;

  let id = connection.last_insert_rowid();
  Ok(ConcurrencyChannelRecord {
    id,
    file_id: channel.file_id,
    source_symbol_qualified_name: channel.source_symbol_qualified_name.clone(),
    channel_kind: channel.channel_kind.clone(),
    tx_name: channel.tx_name.clone(),
    rx_name: channel.rx_name.clone(),
    start_line: channel.start_line,
    end_line: channel.end_line,
  })
}

pub fn insert_concurrency_select(
  connection: &Connection,
  select: &NewConcurrencySelectRecord,
) -> Result<ConcurrencySelectRecord> {
  connection.execute(
    "INSERT INTO concurrency_selects (file_id, source_symbol_qualified_name, start_line, end_line)
     VALUES (?1, ?2, ?3, ?4)",
    (
      select.file_id,
      &select.source_symbol_qualified_name,
      select.start_line,
      select.end_line,
    ),
  )?;

  let id = connection.last_insert_rowid();
  Ok(ConcurrencySelectRecord {
    id,
    file_id: select.file_id,
    source_symbol_qualified_name: select.source_symbol_qualified_name.clone(),
    start_line: select.start_line,
    end_line: select.end_line,
  })
}

pub fn insert_rust_unsafe_block(
  connection: &Connection,
  block: &NewRustUnsafeBlockRecord,
) -> Result<RustUnsafeBlockRecord> {
  connection.execute(
    "INSERT INTO rust_unsafe_blocks (file_id, source_symbol_qualified_name, start_line, end_line)
     VALUES (?1, ?2, ?3, ?4)",
    (
      block.file_id,
      &block.source_symbol_qualified_name,
      block.start_line,
      block.end_line,
    ),
  )?;

  let id = connection.last_insert_rowid();
  Ok(RustUnsafeBlockRecord {
    id,
    file_id: block.file_id,
    source_symbol_qualified_name: block.source_symbol_qualified_name.clone(),
    start_line: block.start_line,
    end_line: block.end_line,
  })
}

pub fn insert_rust_unsafe_function(
  connection: &Connection,
  func: &NewRustUnsafeFunctionRecord,
) -> Result<RustUnsafeFunctionRecord> {
  connection.execute(
    "INSERT INTO rust_unsafe_functions (file_id, qualified_name, start_line, end_line)
     VALUES (?1, ?2, ?3, ?4)",
    (
      func.file_id,
      &func.qualified_name,
      func.start_line,
      func.end_line,
    ),
  )?;

  let id = connection.last_insert_rowid();
  Ok(RustUnsafeFunctionRecord {
    id,
    file_id: func.file_id,
    qualified_name: func.qualified_name.clone(),
    start_line: func.start_line,
    end_line: func.end_line,
  })
}

pub fn insert_rust_ffi_binding(
  connection: &Connection,
  ffi: &NewRustFFIBindingRecord,
) -> Result<RustFFIBindingRecord> {
  connection.execute(
    "INSERT INTO rust_ffi_bindings (file_id, source_symbol_qualified_name, foreign_item_name, abi, start_line, end_line)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    (
      ffi.file_id,
      &ffi.source_symbol_qualified_name,
      &ffi.foreign_item_name,
      &ffi.abi,
      ffi.start_line,
      ffi.end_line,
    ),
  )?;

  let id = connection.last_insert_rowid();
  Ok(RustFFIBindingRecord {
    id,
    file_id: ffi.file_id,
    source_symbol_qualified_name: ffi.source_symbol_qualified_name.clone(),
    foreign_item_name: ffi.foreign_item_name.clone(),
    abi: ffi.abi.clone(),
    start_line: ffi.start_line,
    end_line: ffi.end_line,
  })
}

pub fn lookup_rust_unsafe_blocks_for_symbol(
  connection: &Connection,
  repository_id: i64,
  symbol_qname: &str,
) -> Result<Vec<RustUnsafeBlockRecord>> {
  let mut stmt = connection.prepare(
    r#"SELECT rub.id, rub.file_id, rub.source_symbol_qualified_name, rub.start_line, rub.end_line
     FROM rust_unsafe_blocks rub
     INNER JOIN files f ON rub.file_id = f.id
     WHERE f.repository_id = ?1 AND (rub.source_symbol_qualified_name = ?2 OR rub.source_symbol_qualified_name LIKE ?3 ESCAPE '\')
     ORDER BY rub.start_line ASC"#,
  )?;
  let escaped = escape_sqlite_like(symbol_qname);
  let pattern = format!("{}::%", escaped);
  let rows = stmt.query_map((repository_id, symbol_qname, &pattern), |row| {
    Ok(RustUnsafeBlockRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      source_symbol_qualified_name: row.get(2)?,
      start_line: row.get(3)?,
      end_line: row.get(4)?,
    })
  })?;
  rows.collect()
}

pub fn lookup_rust_unsafe_functions_for_symbol(
  connection: &Connection,
  repository_id: i64,
  symbol_qname: &str,
) -> Result<Vec<RustUnsafeFunctionRecord>> {
  let mut stmt = connection.prepare(
    r#"SELECT ruf.id, ruf.file_id, ruf.qualified_name, ruf.start_line, ruf.end_line
     FROM rust_unsafe_functions ruf
     INNER JOIN files f ON ruf.file_id = f.id
     WHERE f.repository_id = ?1 AND (ruf.qualified_name = ?2 OR ruf.qualified_name LIKE ?3 ESCAPE '\')
     ORDER BY ruf.start_line ASC"#,
  )?;
  let escaped = escape_sqlite_like(symbol_qname);
  let pattern = format!("{}::%", escaped);
  let rows = stmt.query_map((repository_id, symbol_qname, &pattern), |row| {
    Ok(RustUnsafeFunctionRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      qualified_name: row.get(2)?,
      start_line: row.get(3)?,
      end_line: row.get(4)?,
    })
  })?;
  rows.collect()
}

pub fn lookup_rust_ffi_bindings_for_symbol(
  connection: &Connection,
  repository_id: i64,
  symbol_qname: &str,
) -> Result<Vec<RustFFIBindingRecord>> {
  let mut stmt = connection.prepare(
    r#"SELECT rfb.id, rfb.file_id, rfb.source_symbol_qualified_name, rfb.foreign_item_name, rfb.abi, rfb.start_line, rfb.end_line
     FROM rust_ffi_bindings rfb
     INNER JOIN files f ON rfb.file_id = f.id
     WHERE f.repository_id = ?1 AND (rfb.source_symbol_qualified_name = ?2 OR rfb.source_symbol_qualified_name LIKE ?3 ESCAPE '\')
     ORDER BY rfb.start_line ASC"#,
  )?;
  let escaped = escape_sqlite_like(symbol_qname);
  let pattern = format!("{}::%", escaped);
  let rows = stmt.query_map((repository_id, symbol_qname, &pattern), |row| {
    Ok(RustFFIBindingRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      source_symbol_qualified_name: row.get(2)?,
      foreign_item_name: row.get(3)?,
      abi: row.get(4)?,
      start_line: row.get(5)?,
      end_line: row.get(6)?,
    })
  })?;
  rows.collect()
}

pub fn lookup_rust_unsafe_blocks_for_file(
  connection: &Connection,
  repository_id: i64,
  file_path: &str,
) -> Result<Vec<RustUnsafeBlockRecord>> {
  let mut stmt = connection.prepare(
    r#"SELECT rub.id, rub.file_id, rub.source_symbol_qualified_name, rub.start_line, rub.end_line
     FROM rust_unsafe_blocks rub
     INNER JOIN files f ON rub.file_id = f.id
     WHERE f.repository_id = ?1 AND f.path = ?2
     ORDER BY rub.start_line ASC"#,
  )?;
  let rows = stmt.query_map((repository_id, file_path), |row| {
    Ok(RustUnsafeBlockRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      source_symbol_qualified_name: row.get(2)?,
      start_line: row.get(3)?,
      end_line: row.get(4)?,
    })
  })?;
  rows.collect()
}

pub fn lookup_rust_unsafe_functions_for_file(
  connection: &Connection,
  repository_id: i64,
  file_path: &str,
) -> Result<Vec<RustUnsafeFunctionRecord>> {
  let mut stmt = connection.prepare(
    r#"SELECT ruf.id, ruf.file_id, ruf.qualified_name, ruf.start_line, ruf.end_line
     FROM rust_unsafe_functions ruf
     INNER JOIN files f ON ruf.file_id = f.id
     WHERE f.repository_id = ?1 AND f.path = ?2
     ORDER BY ruf.start_line ASC"#,
  )?;
  let rows = stmt.query_map((repository_id, file_path), |row| {
    Ok(RustUnsafeFunctionRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      qualified_name: row.get(2)?,
      start_line: row.get(3)?,
      end_line: row.get(4)?,
    })
  })?;
  rows.collect()
}

pub fn lookup_rust_ffi_bindings_for_file(
  connection: &Connection,
  repository_id: i64,
  file_path: &str,
) -> Result<Vec<RustFFIBindingRecord>> {
  let mut stmt = connection.prepare(
    r#"SELECT rfb.id, rfb.file_id, rfb.source_symbol_qualified_name, rfb.foreign_item_name, rfb.abi, rfb.start_line, rfb.end_line
     FROM rust_ffi_bindings rfb
     INNER JOIN files f ON rfb.file_id = f.id
     WHERE f.repository_id = ?1 AND f.path = ?2
     ORDER BY rfb.start_line ASC"#,
  )?;
  let rows = stmt.query_map((repository_id, file_path), |row| {
    Ok(RustFFIBindingRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      source_symbol_qualified_name: row.get(2)?,
      foreign_item_name: row.get(3)?,
      abi: row.get(4)?,
      start_line: row.get(5)?,
      end_line: row.get(6)?,
    })
  })?;
  rows.collect()
}

fn escape_sqlite_like(s: &str) -> String {
  s.replace('\\', "\\\\")
    .replace('%', "\\%")
    .replace('_', "\\_")
}

pub fn lookup_concurrency_spawns_for_symbol(
  connection: &Connection,
  repository_id: i64,
  symbol_qname: &str,
) -> Result<Vec<ConcurrencySpawnRecord>> {
  let mut stmt = connection.prepare(
    r#"SELECT cs.id, cs.file_id, cs.source_symbol_qualified_name, cs.spawn_kind, cs.target_name, cs.start_line, cs.end_line
     FROM concurrency_spawns cs
     INNER JOIN files f ON cs.file_id = f.id
     WHERE f.repository_id = ?1 AND (cs.source_symbol_qualified_name = ?2 OR cs.source_symbol_qualified_name LIKE ?3 ESCAPE '\')
     ORDER BY cs.start_line ASC"#,
  )?;
  let escaped = escape_sqlite_like(symbol_qname);
  let pattern = format!("{}::%", escaped);
  let rows = stmt.query_map((repository_id, symbol_qname, &pattern), |row| {
    Ok(ConcurrencySpawnRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      source_symbol_qualified_name: row.get(2)?,
      spawn_kind: row.get(3)?,
      target_name: row.get(4)?,
      start_line: row.get(5)?,
      end_line: row.get(6)?,
    })
  })?;
  rows.collect()
}

pub fn lookup_concurrency_channels_for_symbol(
  connection: &Connection,
  repository_id: i64,
  symbol_qname: &str,
) -> Result<Vec<ConcurrencyChannelRecord>> {
  let mut stmt = connection.prepare(
    r#"SELECT cc.id, cc.file_id, cc.source_symbol_qualified_name, cc.channel_kind, cc.tx_name, cc.rx_name, cc.start_line, cc.end_line
     FROM concurrency_channels cc
     INNER JOIN files f ON cc.file_id = f.id
     WHERE f.repository_id = ?1 AND (cc.source_symbol_qualified_name = ?2 OR cc.source_symbol_qualified_name LIKE ?3 ESCAPE '\')
     ORDER BY cc.start_line ASC"#,
  )?;
  let escaped = escape_sqlite_like(symbol_qname);
  let pattern = format!("{}::%", escaped);
  let rows = stmt.query_map((repository_id, symbol_qname, &pattern), |row| {
    Ok(ConcurrencyChannelRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      source_symbol_qualified_name: row.get(2)?,
      channel_kind: row.get(3)?,
      tx_name: row.get(4)?,
      rx_name: row.get(5)?,
      start_line: row.get(6)?,
      end_line: row.get(7)?,
    })
  })?;
  rows.collect()
}

pub fn lookup_concurrency_selects_for_symbol(
  connection: &Connection,
  repository_id: i64,
  symbol_qname: &str,
) -> Result<Vec<ConcurrencySelectRecord>> {
  let mut stmt = connection.prepare(
    r#"SELECT cs.id, cs.file_id, cs.source_symbol_qualified_name, cs.start_line, cs.end_line
     FROM concurrency_selects cs
     INNER JOIN files f ON cs.file_id = f.id
     WHERE f.repository_id = ?1 AND (cs.source_symbol_qualified_name = ?2 OR cs.source_symbol_qualified_name LIKE ?3 ESCAPE '\')
     ORDER BY cs.start_line ASC"#,
  )?;
  let escaped = escape_sqlite_like(symbol_qname);
  let pattern = format!("{}::%", escaped);
  let rows = stmt.query_map((repository_id, symbol_qname, &pattern), |row| {
    Ok(ConcurrencySelectRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      source_symbol_qualified_name: row.get(2)?,
      start_line: row.get(3)?,
      end_line: row.get(4)?,
    })
  })?;
  rows.collect()
}

pub fn delete_file_by_path(
  connection: &Connection,
  repository_id: i64,
  path: &str,
) -> Result<usize> {
  let transaction = connection.unchecked_transaction()?;
  transaction.execute(
    "UPDATE OR IGNORE edges
     SET unresolved_target = (SELECT qualified_name FROM symbols WHERE id = edges.target_symbol_id),
         target_symbol_id = NULL
     WHERE target_symbol_id IN (
         SELECT s.id 
         FROM symbols s
         INNER JOIN files f ON s.file_id = f.id
         WHERE f.repository_id = ?1 AND f.path = ?2
     )",
    (repository_id, path),
  )?;
  let rows = transaction.execute(
    "DELETE FROM files WHERE repository_id = ?1 AND path = ?2",
    (repository_id, path),
  )?;
  transaction.commit()?;
  Ok(rows)
}

pub fn reindex_file(connection: &Connection, file: &NewFileRecord) -> Result<FileRecord> {
  let transaction = connection.unchecked_transaction()?;
  transaction.execute(
    "UPDATE OR IGNORE edges
     SET unresolved_target = (SELECT qualified_name FROM symbols WHERE id = edges.target_symbol_id),
         target_symbol_id = NULL
     WHERE target_symbol_id IN (
         SELECT s.id 
         FROM symbols s
         INNER JOIN files f ON s.file_id = f.id
         WHERE f.repository_id = ?1 AND f.path = ?2
     )",
    (file.repository_id, file.path.as_str()),
  )?;
  transaction.execute(
    "DELETE FROM files WHERE repository_id = ?1 AND path = ?2",
    (file.repository_id, file.path.as_str()),
  )?;
  transaction.execute(
    "INSERT INTO files (repository_id, path, language, content_hash, line_count, last_index_run_id)
      VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    (
      file.repository_id,
      file.path.as_str(),
      file.language.as_deref(),
      file.content_hash.as_deref(),
      file.line_count,
      file.last_index_run_id,
    ),
  )?;

  let id = transaction.last_insert_rowid();
  let file_row = get_file_by_id(&transaction, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
  transaction.commit()?;
  Ok(file_row)
}

pub fn reindex_file_raw(connection: &Connection, file: &NewFileRecord) -> Result<FileRecord> {
  connection.execute(
    "UPDATE OR IGNORE edges
     SET unresolved_target = (SELECT qualified_name FROM symbols WHERE id = edges.target_symbol_id),
         target_symbol_id = NULL
     WHERE target_symbol_id IN (
         SELECT s.id 
         FROM symbols s
         INNER JOIN files f ON s.file_id = f.id
         WHERE f.repository_id = ?1 AND f.path = ?2
     )",
    (file.repository_id, file.path.as_str()),
  )?;
  connection.execute(
    "DELETE FROM files WHERE repository_id = ?1 AND path = ?2",
    (file.repository_id, file.path.as_str()),
  )?;
  connection.execute(
    "INSERT INTO files (repository_id, path, language, content_hash, line_count, last_index_run_id)
      VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    (
      file.repository_id,
      file.path.as_str(),
      file.language.as_deref(),
      file.content_hash.as_deref(),
      file.line_count,
      file.last_index_run_id,
    ),
  )?;

  let id = connection.last_insert_rowid();
  let file_row = get_file_by_id(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
  Ok(file_row)
}

pub fn initialize_schema(connection: &Connection) -> Result<()> {
  connection.execute_batch(
    r#"
    PRAGMA foreign_keys = ON;

    CREATE TABLE IF NOT EXISTS repositories (
      id INTEGER PRIMARY KEY,
      root_path TEXT NOT NULL UNIQUE,
      vcs_type TEXT NOT NULL DEFAULT 'git',
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

    CREATE TABLE IF NOT EXISTS index_runs (
      id INTEGER PRIMARY KEY,
      repository_id INTEGER NOT NULL,
      started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      finished_at TEXT,
      status TEXT NOT NULL,
      files_scanned INTEGER NOT NULL DEFAULT 0,
      files_changed INTEGER NOT NULL DEFAULT 0,
      files_deleted INTEGER NOT NULL DEFAULT 0,
      error_message TEXT,
      FOREIGN KEY (repository_id) REFERENCES repositories(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS files (
      id INTEGER PRIMARY KEY,
      repository_id INTEGER NOT NULL,
      path TEXT NOT NULL,
      language TEXT,
      content_hash TEXT,
      line_count INTEGER,
      last_index_run_id INTEGER,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      UNIQUE(repository_id, path),
      FOREIGN KEY (repository_id) REFERENCES repositories(id) ON DELETE CASCADE,
      FOREIGN KEY (last_index_run_id) REFERENCES index_runs(id) ON DELETE SET NULL
    );

    CREATE TABLE IF NOT EXISTS symbols (
      id INTEGER PRIMARY KEY,
      file_id INTEGER NOT NULL,
      name TEXT NOT NULL,
      qualified_name TEXT NOT NULL,
      kind TEXT NOT NULL,
      start_line INTEGER NOT NULL,
      end_line INTEGER NOT NULL,
      start_column INTEGER,
      end_column INTEGER,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      UNIQUE(file_id, qualified_name),
      FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS edges (
      id INTEGER PRIMARY KEY,
      source_symbol_id INTEGER NOT NULL,
      target_symbol_id INTEGER,
      unresolved_target TEXT,
      kind TEXT NOT NULL,
      confidence REAL,
      ordering INTEGER,
      placeholders TEXT,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      CHECK (target_symbol_id IS NOT NULL OR unresolved_target IS NOT NULL),
      FOREIGN KEY (source_symbol_id) REFERENCES symbols(id) ON DELETE CASCADE,
      FOREIGN KEY (target_symbol_id) REFERENCES symbols(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS docs (
      id INTEGER PRIMARY KEY,
      file_id INTEGER NOT NULL,
      symbol_id INTEGER,
      title TEXT,
      body TEXT NOT NULL,
      start_line INTEGER,
      end_line INTEGER,
      source_kind TEXT NOT NULL,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE,
      FOREIGN KEY (symbol_id) REFERENCES symbols(id) ON DELETE SET NULL
    );

    CREATE TABLE IF NOT EXISTS tests (
      id INTEGER PRIMARY KEY,
      file_id INTEGER NOT NULL,
      name TEXT NOT NULL,
      qualified_name TEXT NOT NULL,
      kind TEXT NOT NULL,
      is_parametrized INTEGER NOT NULL DEFAULT 0,
      framework TEXT NOT NULL DEFAULT 'pytest',
      start_line INTEGER,
      end_line INTEGER,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      UNIQUE(file_id, qualified_name),
      FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS git_commits (
      id INTEGER PRIMARY KEY,
      repository_id INTEGER NOT NULL,
      commit_hash TEXT NOT NULL,
      author_name TEXT NOT NULL,
      author_email TEXT NOT NULL,
      authored_date TEXT NOT NULL,
      message TEXT NOT NULL,
      UNIQUE(repository_id, commit_hash),
      FOREIGN KEY (repository_id) REFERENCES repositories(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS git_commit_files (
      commit_id INTEGER NOT NULL,
      file_path TEXT NOT NULL,
      PRIMARY KEY (commit_id, file_path),
      FOREIGN KEY (commit_id) REFERENCES git_commits(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS git_commit_symbols (
      commit_id INTEGER NOT NULL,
      symbol_qualified_name TEXT NOT NULL,
      PRIMARY KEY (commit_id, symbol_qualified_name),
      FOREIGN KEY (commit_id) REFERENCES git_commits(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS routes (
      id INTEGER PRIMARY KEY,
      file_id INTEGER NOT NULL,
      symbol_id INTEGER,
      handler_name TEXT NOT NULL,
      qualified_name TEXT NOT NULL,
      method TEXT NOT NULL,
      path TEXT NOT NULL,
      response_model TEXT,
      start_line INTEGER,
      end_line INTEGER,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      UNIQUE(file_id, qualified_name, method, path),
      FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE,
      FOREIGN KEY (symbol_id) REFERENCES symbols(id) ON DELETE SET NULL
    );

    CREATE TABLE IF NOT EXISTS pydantic_models (
      id INTEGER PRIMARY KEY,
      file_id INTEGER NOT NULL,
      symbol_id INTEGER,
      name TEXT NOT NULL,
      qualified_name TEXT NOT NULL,
      start_line INTEGER NOT NULL,
      end_line INTEGER NOT NULL,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      UNIQUE(file_id, qualified_name),
      FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE,
      FOREIGN KEY (symbol_id) REFERENCES symbols(id) ON DELETE SET NULL
    );

    CREATE TABLE IF NOT EXISTS pydantic_fields (
      id INTEGER PRIMARY KEY,
      model_id INTEGER NOT NULL,
      name TEXT NOT NULL,
      type_annotation TEXT NOT NULL,
      default_value TEXT,
      is_required INTEGER NOT NULL DEFAULT 1,
      UNIQUE(model_id, name),
      FOREIGN KEY (model_id) REFERENCES pydantic_models(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS pydantic_validators (
      id INTEGER PRIMARY KEY,
      model_id INTEGER NOT NULL,
      name TEXT NOT NULL,
      validator_type TEXT NOT NULL,
      target_fields TEXT NOT NULL,
      UNIQUE(model_id, name),
      FOREIGN KEY (model_id) REFERENCES pydantic_models(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS cargo_packages (
      id INTEGER PRIMARY KEY,
      repository_id INTEGER NOT NULL,
      name TEXT NOT NULL,
      manifest_path TEXT NOT NULL,
      version TEXT NOT NULL,
      is_workspace_member INTEGER NOT NULL,
      UNIQUE(repository_id, name, manifest_path),
      FOREIGN KEY (repository_id) REFERENCES repositories(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS cargo_dependencies (
      id INTEGER PRIMARY KEY,
      package_id INTEGER NOT NULL,
      name TEXT NOT NULL,
      version_requirement TEXT,
      is_workspace_dependency INTEGER NOT NULL,
      features TEXT NOT NULL,
      is_dev INTEGER NOT NULL,
      UNIQUE(package_id, name),
      FOREIGN KEY (package_id) REFERENCES cargo_packages(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS fsharp_projects (
      id INTEGER PRIMARY KEY,
      repository_id INTEGER NOT NULL,
      name TEXT NOT NULL,
      project_path TEXT NOT NULL,
      target_framework TEXT,
      is_solution_member INTEGER NOT NULL,
      UNIQUE(repository_id, name, project_path),
      FOREIGN KEY (repository_id) REFERENCES repositories(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS fsharp_dependencies (
      id INTEGER PRIMARY KEY,
      project_id INTEGER NOT NULL,
      name TEXT NOT NULL,
      dependency_type TEXT NOT NULL,
      version_requirement TEXT,
      UNIQUE(project_id, name, dependency_type),
      FOREIGN KEY (project_id) REFERENCES fsharp_projects(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS kotlin_projects (
      id INTEGER PRIMARY KEY,
      repository_id INTEGER NOT NULL,
      name TEXT NOT NULL,
      project_path TEXT NOT NULL,
      target_framework TEXT,
      is_solution_member INTEGER NOT NULL,
      generated_source_dirs TEXT,
      UNIQUE(repository_id, name, project_path),
      FOREIGN KEY (repository_id) REFERENCES repositories(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS generated_symbols (
      symbol_id INTEGER PRIMARY KEY,
      provenance TEXT NOT NULL,
      FOREIGN KEY (symbol_id) REFERENCES symbols(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS kotlin_dependencies (
      id INTEGER PRIMARY KEY,
      project_id INTEGER NOT NULL,
      name TEXT NOT NULL,
      dependency_type TEXT NOT NULL,
      version_requirement TEXT,
      UNIQUE(project_id, name, dependency_type),
      FOREIGN KEY (project_id) REFERENCES kotlin_projects(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS swift_projects (
      id INTEGER PRIMARY KEY,
      repository_id INTEGER NOT NULL,
      name TEXT NOT NULL,
      project_path TEXT NOT NULL,
      target_framework TEXT,
      is_solution_member INTEGER NOT NULL,
      UNIQUE(repository_id, name, project_path),
      FOREIGN KEY (repository_id) REFERENCES repositories(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS swift_dependencies (
      id INTEGER PRIMARY KEY,
      project_id INTEGER NOT NULL,
      name TEXT NOT NULL,
      dependency_type TEXT NOT NULL,
      version_requirement TEXT,
      UNIQUE(project_id, name, dependency_type),
      FOREIGN KEY (project_id) REFERENCES swift_projects(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS concurrency_spawns (
      id INTEGER PRIMARY KEY,
      file_id INTEGER NOT NULL,
      source_symbol_qualified_name TEXT NOT NULL,
      spawn_kind TEXT NOT NULL,
      target_name TEXT,
      start_line INTEGER NOT NULL,
      end_line INTEGER NOT NULL,
      FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS concurrency_channels (
      id INTEGER PRIMARY KEY,
      file_id INTEGER NOT NULL,
      source_symbol_qualified_name TEXT NOT NULL,
      channel_kind TEXT NOT NULL,
      tx_name TEXT NOT NULL,
      rx_name TEXT NOT NULL,
      start_line INTEGER NOT NULL,
      end_line INTEGER NOT NULL,
      FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS concurrency_selects (
      id INTEGER PRIMARY KEY,
      file_id INTEGER NOT NULL,
      source_symbol_qualified_name TEXT NOT NULL,
      start_line INTEGER NOT NULL,
      end_line INTEGER NOT NULL,
      FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS rust_unsafe_blocks (
      id INTEGER PRIMARY KEY,
      file_id INTEGER NOT NULL,
      source_symbol_qualified_name TEXT NOT NULL,
      start_line INTEGER NOT NULL,
      end_line INTEGER NOT NULL,
      FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS rust_unsafe_functions (
      id INTEGER PRIMARY KEY,
      file_id INTEGER NOT NULL,
      qualified_name TEXT NOT NULL,
      start_line INTEGER NOT NULL,
      end_line INTEGER NOT NULL,
      FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS rust_ffi_bindings (
      id INTEGER PRIMARY KEY,
      file_id INTEGER NOT NULL,
      source_symbol_qualified_name TEXT NOT NULL,
      foreign_item_name TEXT NOT NULL,
      abi TEXT NOT NULL,
      start_line INTEGER NOT NULL,
      end_line INTEGER NOT NULL,
      FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS symbol_coverage (
      id INTEGER PRIMARY KEY,
      file_id INTEGER NOT NULL,
      symbol_id INTEGER NOT NULL,
      report_path TEXT NOT NULL,
      test_suite TEXT,
      lines_valid INTEGER NOT NULL,
      lines_covered INTEGER NOT NULL,
      branches_valid INTEGER NOT NULL,
      branches_covered INTEGER NOT NULL,
      coverable_lines TEXT NOT NULL,
      uncovered_lines TEXT NOT NULL,
      FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE,
      FOREIGN KEY (symbol_id) REFERENCES symbols(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS android_components (
      id INTEGER PRIMARY KEY,
      file_id INTEGER NOT NULL,
      name TEXT NOT NULL,
      component_type TEXT NOT NULL,
      class_name TEXT NOT NULL,
      permission TEXT,
      intent_actions TEXT,
      intent_categories TEXT,
      start_line INTEGER,
      end_line INTEGER,
      UNIQUE(file_id, class_name),
      FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS android_resources (
      id INTEGER PRIMARY KEY,
      file_id INTEGER NOT NULL,
      name TEXT NOT NULL,
      resource_type TEXT NOT NULL,
      value TEXT,
      start_line INTEGER,
      end_line INTEGER,
      UNIQUE(file_id, name, resource_type),
      FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
    );

    CREATE INDEX IF NOT EXISTS idx_android_components_file_id ON android_components(file_id);
    CREATE INDEX IF NOT EXISTS idx_android_components_class_name ON android_components(class_name);
    CREATE INDEX IF NOT EXISTS idx_android_resources_file_id ON android_resources(file_id);
    CREATE INDEX IF NOT EXISTS idx_android_resources_name ON android_resources(name);

    CREATE INDEX IF NOT EXISTS idx_files_repository_id ON files(repository_id);
    CREATE INDEX IF NOT EXISTS idx_files_path ON files(path);
    CREATE INDEX IF NOT EXISTS idx_symbols_file_id ON symbols(file_id);
    CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);
    CREATE INDEX IF NOT EXISTS idx_symbols_qualified_name ON symbols(qualified_name);
    CREATE INDEX IF NOT EXISTS idx_edges_source_symbol_id ON edges(source_symbol_id);
    CREATE INDEX IF NOT EXISTS idx_edges_target_symbol_id ON edges(target_symbol_id);
    CREATE UNIQUE INDEX IF NOT EXISTS ux_edges_resolved_no_ordering
      ON edges(source_symbol_id, target_symbol_id, kind)
      WHERE target_symbol_id IS NOT NULL AND ordering IS NULL;
    CREATE UNIQUE INDEX IF NOT EXISTS ux_edges_resolved_with_ordering
      ON edges(source_symbol_id, target_symbol_id, kind, ordering)
      WHERE target_symbol_id IS NOT NULL AND ordering IS NOT NULL;
    CREATE UNIQUE INDEX IF NOT EXISTS ux_edges_unresolved_no_ordering
      ON edges(source_symbol_id, unresolved_target, kind)
      WHERE target_symbol_id IS NULL AND unresolved_target IS NOT NULL AND ordering IS NULL;
    CREATE UNIQUE INDEX IF NOT EXISTS ux_edges_unresolved_with_ordering
      ON edges(source_symbol_id, unresolved_target, kind, ordering)
      WHERE target_symbol_id IS NULL AND unresolved_target IS NOT NULL AND ordering IS NOT NULL;
    CREATE INDEX IF NOT EXISTS idx_docs_file_id ON docs(file_id);
    CREATE INDEX IF NOT EXISTS idx_docs_symbol_id ON docs(symbol_id);
    CREATE INDEX IF NOT EXISTS idx_tests_file_id ON tests(file_id);
    CREATE INDEX IF NOT EXISTS idx_index_runs_repository_id ON index_runs(repository_id);
    CREATE INDEX IF NOT EXISTS idx_git_commits_repository_id ON git_commits(repository_id);
    CREATE INDEX IF NOT EXISTS idx_git_commit_files_file_path ON git_commit_files(file_path);
    CREATE INDEX IF NOT EXISTS idx_git_commit_symbols_symbol_qname ON git_commit_symbols(symbol_qualified_name);
    CREATE INDEX IF NOT EXISTS idx_routes_file_id ON routes(file_id);
    CREATE INDEX IF NOT EXISTS idx_routes_symbol_id ON routes(symbol_id);
    CREATE INDEX IF NOT EXISTS idx_routes_qualified_name ON routes(qualified_name);
    CREATE INDEX IF NOT EXISTS idx_pydantic_models_file_id ON pydantic_models(file_id);
    CREATE INDEX IF NOT EXISTS idx_pydantic_models_symbol_id ON pydantic_models(symbol_id);
    CREATE INDEX IF NOT EXISTS idx_pydantic_models_qualified_name ON pydantic_models(qualified_name);
    CREATE INDEX IF NOT EXISTS idx_cargo_packages_repository_id ON cargo_packages(repository_id);
    CREATE INDEX IF NOT EXISTS idx_cargo_dependencies_package_id ON cargo_dependencies(package_id);
    CREATE INDEX IF NOT EXISTS idx_fsharp_projects_repository_id ON fsharp_projects(repository_id);
    CREATE INDEX IF NOT EXISTS idx_fsharp_dependencies_project_id ON fsharp_dependencies(project_id);
    CREATE INDEX IF NOT EXISTS idx_kotlin_projects_repository_id ON kotlin_projects(repository_id);
    CREATE INDEX IF NOT EXISTS idx_kotlin_dependencies_project_id ON kotlin_dependencies(project_id);
    CREATE INDEX IF NOT EXISTS idx_swift_projects_repository_id ON swift_projects(repository_id);
    CREATE INDEX IF NOT EXISTS idx_swift_dependencies_project_id ON swift_dependencies(project_id);

    CREATE INDEX IF NOT EXISTS idx_concurrency_spawns_file_id ON concurrency_spawns(file_id);
    CREATE INDEX IF NOT EXISTS idx_concurrency_spawns_source_symbol ON concurrency_spawns(source_symbol_qualified_name);
    CREATE INDEX IF NOT EXISTS idx_concurrency_channels_file_id ON concurrency_channels(file_id);
    CREATE INDEX IF NOT EXISTS idx_concurrency_channels_source_symbol ON concurrency_channels(source_symbol_qualified_name);
    CREATE INDEX IF NOT EXISTS idx_concurrency_selects_file_id ON concurrency_selects(file_id);
    CREATE INDEX IF NOT EXISTS idx_concurrency_selects_source_symbol ON concurrency_selects(source_symbol_qualified_name);

    CREATE INDEX IF NOT EXISTS idx_rust_unsafe_blocks_file_id ON rust_unsafe_blocks(file_id);
    CREATE INDEX IF NOT EXISTS idx_rust_unsafe_blocks_source_symbol ON rust_unsafe_blocks(source_symbol_qualified_name);
    CREATE INDEX IF NOT EXISTS idx_rust_unsafe_functions_file_id ON rust_unsafe_functions(file_id);
    CREATE INDEX IF NOT EXISTS idx_rust_unsafe_functions_qname ON rust_unsafe_functions(qualified_name);
    CREATE INDEX IF NOT EXISTS idx_rust_ffi_bindings_file_id ON rust_ffi_bindings(file_id);
    CREATE INDEX IF NOT EXISTS idx_rust_ffi_bindings_source_symbol ON rust_ffi_bindings(source_symbol_qualified_name);

    CREATE INDEX IF NOT EXISTS idx_symbol_coverage_file_id ON symbol_coverage(file_id);
    CREATE INDEX IF NOT EXISTS idx_symbol_coverage_symbol_id ON symbol_coverage(symbol_id);
    CREATE INDEX IF NOT EXISTS idx_symbol_coverage_report ON symbol_coverage(report_path, test_suite);

    -- FTS5 Search Virtual Tables
    CREATE VIRTUAL TABLE IF NOT EXISTS docs_fts USING fts5(
      title,
      body
    );
    CREATE VIRTUAL TABLE IF NOT EXISTS symbols_fts USING fts5(
      name,
      qualified_name
    );

    -- FTS5 Synchronization Triggers
    CREATE TRIGGER IF NOT EXISTS docs_ai AFTER INSERT ON docs BEGIN
      INSERT INTO docs_fts(rowid, title, body) VALUES (new.id, new.title, new.body);
    END;
    CREATE TRIGGER IF NOT EXISTS docs_ad AFTER DELETE ON docs BEGIN
      DELETE FROM docs_fts WHERE rowid = old.id;
    END;

    CREATE TRIGGER IF NOT EXISTS symbols_ai AFTER INSERT ON symbols BEGIN
      INSERT INTO symbols_fts(rowid, name, qualified_name) VALUES (new.id, new.name, new.qualified_name);
    END;
    CREATE TRIGGER IF NOT EXISTS symbols_ad AFTER DELETE ON symbols BEGIN
      DELETE FROM symbols_fts WHERE rowid = old.id;
    END;

    -- Backfill FTS tables for rows that existed before the triggers were created.
    -- The subquery excludes rowids already present so this statement is idempotent
    -- and safe to run on every schema initialisation (e.g. database upgrades).
    INSERT INTO docs_fts(rowid, title, body)
      SELECT id, title, body FROM docs
      WHERE id NOT IN (SELECT rowid FROM docs_fts);

    INSERT INTO symbols_fts(rowid, name, qualified_name)
      SELECT id, name, qualified_name FROM symbols
      WHERE id NOT IN (SELECT rowid FROM symbols_fts);
    "#,
  )?;

  // Safe schema migration for added column: generated_source_dirs
  let _ = connection.execute(
    "ALTER TABLE kotlin_projects ADD COLUMN generated_source_dirs TEXT",
    [],
  );

  Ok(())
}

pub fn db_status(connection: &Connection) -> Result<DbStatus> {
  let schema_initialized = core_tables_exist(connection)?;
  if !schema_initialized {
    return Ok(DbStatus {
      schema_initialized,
      repositories: 0,
      files: 0,
      symbols: 0,
      edges: 0,
      docs: 0,
      tests: 0,
      index_runs: 0,
      routes: 0,
      pydantic_models: 0,
      pydantic_fields: 0,
      pydantic_validators: 0,
      cargo_packages: 0,
      cargo_dependencies: 0,
      rust_unsafe_blocks: 0,
      rust_unsafe_functions: 0,
      rust_ffi_bindings: 0,
      fsharp_projects: 0,
      fsharp_dependencies: 0,
      kotlin_projects: 0,
      kotlin_dependencies: 0,
      swift_projects: 0,
      swift_dependencies: 0,
      symbol_coverage: 0,
      generated_symbols: 0,
      android_components: 0,
      android_resources: 0,
    });
  }

  Ok(DbStatus {
    schema_initialized,
    repositories: row_count(connection, "repositories")?,
    files: row_count(connection, "files")?,
    symbols: row_count(connection, "symbols")?,
    edges: row_count(connection, "edges")?,
    docs: row_count(connection, "docs")?,
    tests: row_count(connection, "tests")?,
    index_runs: row_count(connection, "index_runs")?,
    routes: row_count(connection, "routes")?,
    pydantic_models: row_count(connection, "pydantic_models")?,
    pydantic_fields: row_count(connection, "pydantic_fields")?,
    pydantic_validators: row_count(connection, "pydantic_validators")?,
    cargo_packages: row_count(connection, "cargo_packages")?,
    cargo_dependencies: row_count(connection, "cargo_dependencies")?,
    rust_unsafe_blocks: row_count(connection, "rust_unsafe_blocks")?,
    rust_unsafe_functions: row_count(connection, "rust_unsafe_functions")?,
    rust_ffi_bindings: row_count(connection, "rust_ffi_bindings")?,
    fsharp_projects: row_count(connection, "fsharp_projects")?,
    fsharp_dependencies: row_count(connection, "fsharp_dependencies")?,
    kotlin_projects: row_count(connection, "kotlin_projects")?,
    kotlin_dependencies: row_count(connection, "kotlin_dependencies")?,
    swift_projects: row_count(connection, "swift_projects")?,
    swift_dependencies: row_count(connection, "swift_dependencies")?,
    symbol_coverage: row_count(connection, "symbol_coverage")?,
    generated_symbols: row_count(connection, "generated_symbols")?,
    android_components: row_count(connection, "android_components")?,
    android_resources: row_count(connection, "android_resources")?,
  })
}

fn core_tables_exist(connection: &Connection) -> Result<bool> {
  let placeholders = CORE_TABLES
    .iter()
    .enumerate()
    .map(|(i, _)| format!("?{}", i + 1))
    .collect::<Vec<_>>()
    .join(", ");
  let query = format!(
    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ({})",
    placeholders
  );
  let mut statement = connection.prepare(&query)?;
  let count = statement.query_row(rusqlite::params_from_iter(CORE_TABLES.iter()), |row| {
    row.get::<_, i64>(0)
  })?;
  Ok(count == CORE_TABLES.len() as i64)
}

fn row_count(connection: &Connection, table: &str) -> Result<i64> {
  let query = format!("SELECT COUNT(*) FROM {table}");
  let mut statement = connection.prepare(&query)?;
  statement.query_row([], |row| row.get::<_, i64>(0))
}

fn get_file_by_id(connection: &Connection, file_id: i64) -> Result<Option<FileRecord>> {
  let mut statement = connection.prepare(
    "SELECT id, repository_id, path, language, content_hash, line_count, last_index_run_id
      FROM files
      WHERE id = ?1",
  )?;

  statement
    .query_row([file_id], |row| {
      Ok(FileRecord {
        id: row.get(0)?,
        repository_id: row.get(1)?,
        path: row.get(2)?,
        language: row.get(3)?,
        content_hash: row.get(4)?,
        line_count: row.get(5)?,
        last_index_run_id: row.get(6)?,
      })
    })
    .optional()
}

fn get_symbol_by_id(connection: &Connection, symbol_id: i64) -> Result<Option<SymbolRecord>> {
  let mut statement = connection.prepare(
    "SELECT id, file_id, name, qualified_name, kind, start_line, end_line, start_column, end_column
      FROM symbols
      WHERE id = ?1",
  )?;

  statement
    .query_row([symbol_id], |row| {
      Ok(SymbolRecord {
        id: row.get(0)?,
        file_id: row.get(1)?,
        name: row.get(2)?,
        qualified_name: row.get(3)?,
        kind: row.get(4)?,
        start_line: row.get(5)?,
        end_line: row.get(6)?,
        start_column: row.get(7)?,
        end_column: row.get(8)?,
      })
    })
    .optional()
}

fn get_edge_by_id(connection: &Connection, edge_id: i64) -> Result<Option<EdgeRecord>> {
  let mut statement = connection.prepare(
    "SELECT id, source_symbol_id, target_symbol_id, unresolved_target, kind, confidence, ordering, placeholders
      FROM edges
      WHERE id = ?1",
  )?;

  statement
    .query_row([edge_id], |row| {
      Ok(EdgeRecord {
        id: row.get(0)?,
        source_symbol_id: row.get(1)?,
        target_symbol_id: row.get(2)?,
        unresolved_target: row.get(3)?,
        kind: row.get(4)?,
        confidence: row.get(5)?,
        ordering: row.get(6)?,
        placeholders: row.get(7)?,
      })
    })
    .optional()
}

fn get_index_run_by_id(
  connection: &Connection,
  index_run_id: i64,
) -> Result<Option<IndexRunRecord>> {
  let mut statement = connection.prepare(
    "SELECT id, repository_id, status, files_scanned, files_changed, files_deleted, error_message, finished_at
      FROM index_runs
      WHERE id = ?1",
  )?;

  statement
    .query_row([index_run_id], |row| {
      Ok(IndexRunRecord {
        id: row.get(0)?,
        repository_id: row.get(1)?,
        status: row.get(2)?,
        files_scanned: row.get(3)?,
        files_changed: row.get(4)?,
        files_deleted: row.get(5)?,
        error_message: row.get(6)?,
        finished_at: row.get(7)?,
      })
    })
    .optional()
}

pub fn insert_git_commit(
  connection: &Connection,
  repository_id: i64,
  commit: &rkg_core::GitCommitInfo,
) -> Result<i64> {
  connection.execute(
    "INSERT INTO git_commits (repository_id, commit_hash, author_name, author_email, authored_date, message)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
     ON CONFLICT(repository_id, commit_hash) DO NOTHING",
    (
      repository_id,
      &commit.hash,
      &commit.author_name,
      &commit.author_email,
      &commit.date,
      &commit.message,
    ),
  )?;

  let id: i64 = connection.query_row(
    "SELECT id FROM git_commits WHERE repository_id = ?1 AND commit_hash = ?2",
    (repository_id, &commit.hash),
    |row| row.get(0),
  )?;
  Ok(id)
}

pub fn insert_git_commit_file(
  connection: &Connection,
  commit_id: i64,
  file_path: &str,
) -> Result<()> {
  connection.execute(
    "INSERT OR IGNORE INTO git_commit_files (commit_id, file_path)
     VALUES (?1, ?2)",
    (commit_id, file_path),
  )?;
  Ok(())
}

pub fn insert_git_commit_symbol(
  connection: &Connection,
  commit_id: i64,
  symbol_qualified_name: &str,
) -> Result<()> {
  connection.execute(
    "INSERT OR IGNORE INTO git_commit_symbols (commit_id, symbol_qualified_name)
     VALUES (?1, ?2)",
    (commit_id, symbol_qualified_name),
  )?;
  Ok(())
}

pub fn get_last_modified_commit_for_file(
  connection: &Connection,
  repository_id: i64,
  file_path: &str,
) -> Result<Option<rkg_core::GitCommitInfo>> {
  let mut stmt = connection.prepare(
    "SELECT gc.commit_hash, gc.author_name, gc.author_email, gc.authored_date, gc.message
     FROM git_commits gc
     INNER JOIN git_commit_files gcf ON gc.id = gcf.commit_id
     WHERE gc.repository_id = ?1 AND gcf.file_path = ?2
     ORDER BY gc.authored_date DESC
     LIMIT 1",
  )?;

  stmt
    .query_row((repository_id, file_path), |row| {
      Ok(rkg_core::GitCommitInfo {
        hash: row.get(0)?,
        author_name: row.get(1)?,
        author_email: row.get(2)?,
        date: row.get(3)?,
        message: row.get(4)?,
      })
    })
    .optional()
}

pub fn get_file_churn(
  connection: &Connection,
  repository_id: i64,
  file_path: &str,
) -> Result<usize> {
  let count: i64 = connection.query_row(
    "SELECT COUNT(DISTINCT gcf.commit_id)
     FROM git_commit_files gcf
     INNER JOIN git_commits gc ON gcf.commit_id = gc.id
     WHERE gc.repository_id = ?1 AND gcf.file_path = ?2",
    (repository_id, file_path),
    |row| row.get(0),
  )?;
  Ok(count as usize)
}

pub fn get_author_frequency_for_file(
  connection: &Connection,
  repository_id: i64,
  file_path: &str,
) -> Result<Vec<(String, usize)>> {
  let mut stmt = connection.prepare(
    "SELECT gc.author_name || ' <' || gc.author_email || '>', COUNT(*) as commit_count
     FROM git_commits gc
     INNER JOIN git_commit_files gcf ON gc.id = gcf.commit_id
     WHERE gc.repository_id = ?1 AND gcf.file_path = ?2
     GROUP BY gc.author_name, gc.author_email
     ORDER BY commit_count DESC",
  )?;

  let rows = stmt.query_map((repository_id, file_path), |row| {
    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
  })?;

  rows.collect()
}

pub fn get_file_cochanges(
  connection: &Connection,
  repository_id: i64,
  file_path: &str,
) -> Result<Vec<(String, usize)>> {
  let mut stmt = connection.prepare(
    "SELECT gcf2.file_path, COUNT(*) as cochange_count
     FROM git_commit_files gcf1
     INNER JOIN git_commits gc ON gcf1.commit_id = gc.id
     INNER JOIN git_commit_files gcf2 ON gcf1.commit_id = gcf2.commit_id
     WHERE gc.repository_id = ?1 AND gcf1.file_path = ?2 AND gcf2.file_path != ?2
     GROUP BY gcf2.file_path
     ORDER BY cochange_count DESC",
  )?;

  let rows = stmt.query_map((repository_id, file_path), |row| {
    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
  })?;

  rows.collect()
}

pub fn get_symbol_churn(
  connection: &Connection,
  repository_id: i64,
  symbol_qname: &str,
) -> Result<usize> {
  let count: i64 = connection.query_row(
    "SELECT COUNT(DISTINCT gcs.commit_id)
     FROM git_commit_symbols gcs
     INNER JOIN git_commits gc ON gcs.commit_id = gc.id
     WHERE gc.repository_id = ?1 AND gcs.symbol_qualified_name = ?2",
    (repository_id, symbol_qname),
    |row| row.get(0),
  )?;
  Ok(count as usize)
}

pub fn get_symbol_cochanges(
  connection: &Connection,
  repository_id: i64,
  symbol_qname: &str,
) -> Result<Vec<(String, usize)>> {
  let mut stmt = connection.prepare(
    "SELECT gcs2.symbol_qualified_name, COUNT(*) as cochange_count
     FROM git_commit_symbols gcs1
     INNER JOIN git_commits gc ON gcs1.commit_id = gc.id
     INNER JOIN git_commit_symbols gcs2 ON gcs1.commit_id = gcs2.commit_id
     WHERE gc.repository_id = ?1 AND gcs1.symbol_qualified_name = ?2 AND gcs2.symbol_qualified_name != ?2
     GROUP BY gcs2.symbol_qualified_name
     ORDER BY cochange_count DESC",
  )?;

  let rows = stmt.query_map((repository_id, symbol_qname), |row| {
    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
  })?;

  rows.collect()
}

pub fn clear_git_metadata(connection: &Connection, repository_id: i64) -> Result<()> {
  connection.execute(
    "DELETE FROM git_commits WHERE repository_id = ?1",
    [repository_id],
  )?;
  Ok(())
}

pub fn insert_symbol_coverage(
  connection: &Connection,
  record: &NewSymbolCoverageRecord,
) -> Result<SymbolCoverageRecord> {
  connection.execute(
    "INSERT INTO symbol_coverage (file_id, symbol_id, report_path, test_suite, lines_valid, lines_covered, branches_valid, branches_covered, coverable_lines, uncovered_lines)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    (
      record.file_id,
      record.symbol_id,
      &record.report_path,
      record.test_suite.as_deref(),
      record.lines_valid,
      record.lines_covered,
      record.branches_valid,
      record.branches_covered,
      &record.coverable_lines,
      &record.uncovered_lines,
    ),
  )?;

  let id = connection.last_insert_rowid();
  get_symbol_coverage_by_id(connection, id)
}

pub fn get_symbol_coverage_by_id(connection: &Connection, id: i64) -> Result<SymbolCoverageRecord> {
  connection.query_row(
    "SELECT id, file_id, symbol_id, report_path, test_suite, lines_valid, lines_covered, branches_valid, branches_covered, coverable_lines, uncovered_lines
     FROM symbol_coverage WHERE id = ?1",
    [id],
    |row| {
      Ok(SymbolCoverageRecord {
        id: row.get(0)?,
        file_id: row.get(1)?,
        symbol_id: row.get(2)?,
        report_path: row.get(3)?,
        test_suite: row.get(4)?,
        lines_valid: row.get(5)?,
        lines_covered: row.get(6)?,
        branches_valid: row.get(7)?,
        branches_covered: row.get(8)?,
        coverable_lines: row.get(9)?,
        uncovered_lines: row.get(10)?,
      })
    },
  )
}

pub fn delete_symbol_coverage_by_report(
  connection: &Connection,
  report_path: &str,
  test_suite: Option<&str>,
) -> Result<usize> {
  match test_suite {
    Some(suite) => connection.execute(
      "DELETE FROM symbol_coverage WHERE report_path = ?1 AND test_suite = ?2",
      (report_path, suite),
    ),
    None => connection.execute(
      "DELETE FROM symbol_coverage WHERE report_path = ?1 AND test_suite IS NULL",
      [report_path],
    ),
  }
}

pub fn list_symbol_coverage_for_symbol(
  connection: &Connection,
  symbol_id: i64,
) -> Result<Vec<SymbolCoverageRecord>> {
  let mut stmt = connection.prepare(
    "SELECT id, file_id, symbol_id, report_path, test_suite, lines_valid, lines_covered, branches_valid, branches_covered, coverable_lines, uncovered_lines
     FROM symbol_coverage WHERE symbol_id = ?1",
  )?;

  let rows = stmt.query_map([symbol_id], |row| {
    Ok(SymbolCoverageRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      symbol_id: row.get(2)?,
      report_path: row.get(3)?,
      test_suite: row.get(4)?,
      lines_valid: row.get(5)?,
      lines_covered: row.get(6)?,
      branches_valid: row.get(7)?,
      branches_covered: row.get(8)?,
      coverable_lines: row.get(9)?,
      uncovered_lines: row.get(10)?,
    })
  })?;

  let mut results = Vec::new();
  for r in rows {
    results.push(r?);
  }
  Ok(results)
}

pub fn list_symbol_coverage_for_file(
  connection: &Connection,
  file_id: i64,
) -> Result<Vec<SymbolCoverageRecord>> {
  let mut stmt = connection.prepare(
    "SELECT id, file_id, symbol_id, report_path, test_suite, lines_valid, lines_covered, branches_valid, branches_covered, coverable_lines, uncovered_lines
     FROM symbol_coverage WHERE file_id = ?1",
  )?;

  let rows = stmt.query_map([file_id], |row| {
    Ok(SymbolCoverageRecord {
      id: row.get(0)?,
      file_id: row.get(1)?,
      symbol_id: row.get(2)?,
      report_path: row.get(3)?,
      test_suite: row.get(4)?,
      lines_valid: row.get(5)?,
      lines_covered: row.get(6)?,
      branches_valid: row.get(7)?,
      branches_covered: row.get(8)?,
      coverable_lines: row.get(9)?,
      uncovered_lines: row.get(10)?,
    })
  })?;

  let mut results = Vec::new();
  for r in rows {
    results.push(r?);
  }
  Ok(results)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn initializes_all_phase_1_1_tables() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");

    let mut statement = connection
      .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name = ?1")
      .expect("table lookup statement must prepare");

    for table_name in [
      "repositories",
      "files",
      "symbols",
      "edges",
      "docs",
      "tests",
      "index_runs",
    ] {
      let found_name = statement
        .query_row([table_name], |row| row.get::<_, String>(0))
        .expect("required table must exist");
      assert_eq!(found_name, table_name);
    }
  }

  #[test]
  fn schema_initialization_is_idempotent() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("first schema initialization must succeed");
    initialize_schema(&connection).expect("second schema initialization must succeed");
  }

  #[test]
  fn enforces_foreign_key_and_unique_constraints_for_phase_1_2_paths() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");

    connection
      .execute(
        "INSERT INTO repositories (root_path, vcs_type) VALUES (?1, ?2)",
        ["/tmp/repo", "git"],
      )
      .expect("first repository insert must succeed");

    let duplicate_repo_result = connection.execute(
      "INSERT INTO repositories (root_path, vcs_type) VALUES (?1, ?2)",
      ["/tmp/repo", "git"],
    );
    assert!(duplicate_repo_result.is_err());

    let invalid_file_fk_result = connection.execute(
      "INSERT INTO files (repository_id, path) VALUES (?1, ?2)",
      (99_i64, "src/missing.py"),
    );
    assert!(invalid_file_fk_result.is_err());
  }

  #[test]
  fn prevents_duplicate_resolved_and_unresolved_edges() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");

    connection
      .execute(
        "INSERT INTO repositories (root_path, vcs_type) VALUES (?1, ?2)",
        ["/tmp/repo", "git"],
      )
      .expect("repository insert must succeed");
    connection
      .execute(
        "INSERT INTO files (repository_id, path) VALUES (?1, ?2)",
        (1_i64, "src/example.py"),
      )
      .expect("file insert must succeed");
    connection
      .execute(
        "INSERT INTO symbols (file_id, name, qualified_name, kind, start_line, end_line) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (1_i64, "a", "src.example::a", "Function", 1_i64, 1_i64),
      )
      .expect("source symbol insert must succeed");
    connection
      .execute(
        "INSERT INTO symbols (file_id, name, qualified_name, kind, start_line, end_line) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (1_i64, "b", "src.example::b", "Function", 2_i64, 2_i64),
      )
      .expect("target symbol insert must succeed");

    connection
      .execute(
        "INSERT INTO edges (source_symbol_id, target_symbol_id, kind) VALUES (?1, ?2, ?3)",
        (1_i64, 2_i64, "Calls"),
      )
      .expect("first resolved edge insert must succeed");
    let duplicate_resolved = connection.execute(
      "INSERT INTO edges (source_symbol_id, target_symbol_id, kind) VALUES (?1, ?2, ?3)",
      (1_i64, 2_i64, "Calls"),
    );
    assert!(duplicate_resolved.is_err());

    connection
      .execute(
        "INSERT INTO edges (source_symbol_id, unresolved_target, kind) VALUES (?1, ?2, ?3)",
        (1_i64, "external.mod::c", "Calls"),
      )
      .expect("first unresolved edge insert must succeed");
    let duplicate_unresolved = connection.execute(
      "INSERT INTO edges (source_symbol_id, unresolved_target, kind) VALUES (?1, ?2, ?3)",
      (1_i64, "external.mod::c", "Calls"),
    );
    assert!(duplicate_unresolved.is_err());
  }

  #[test]
  fn test_edge_pipeline_ordering_and_placeholders_persistence() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");

    connection
      .execute(
        "INSERT INTO repositories (root_path, vcs_type) VALUES (?1, ?2)",
        ["/tmp/repo", "git"],
      )
      .expect("repository insert must succeed");
    connection
      .execute(
        "INSERT INTO files (repository_id, path) VALUES (?1, ?2)",
        (1_i64, "src/example.py"),
      )
      .expect("file insert must succeed");
    connection
      .execute(
        "INSERT INTO symbols (file_id, name, qualified_name, kind, start_line, end_line) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (1_i64, "a", "src.example::a", "Function", 1_i64, 1_i64),
      )
      .expect("source symbol insert must succeed");
    connection
      .execute(
        "INSERT INTO symbols (file_id, name, qualified_name, kind, start_line, end_line) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (1_i64, "b", "src.example::b", "Function", 2_i64, 2_i64),
      )
      .expect("target symbol insert must succeed");

    let new_edge = NewEdgeRecord {
      source_symbol_id: 1_i64,
      target_symbol_id: Some(2_i64),
      unresolved_target: None,
      kind: "Calls".to_string(),
      confidence: Some(1.0),
    };

    let edge =
      insert_edge_with_pipeline_metadata(&connection, &new_edge, Some(2), Some("y, z".to_string()))
        .expect("insert_edge_with_pipeline_metadata must succeed");

    assert_eq!(edge.ordering, Some(2));
    assert_eq!(edge.placeholders.as_deref(), Some("y, z"));
  }

  #[test]
  fn inserts_and_looks_up_files_by_path() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-a");

    let inserted_file = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/main.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("abc123".to_string()),
        line_count: Some(10),
        last_index_run_id: None,
      },
    )
    .expect("file insert must succeed");

    assert!(inserted_file.id > 0);
    assert_eq!(inserted_file.path, "src/main.py");
    assert_eq!(inserted_file.language.as_deref(), Some("python"));

    let found = lookup_file_by_path(&connection, 1, "src/main.py")
      .expect("file lookup must succeed")
      .expect("inserted file must be found");
    assert_eq!(found.id, inserted_file.id);
    assert_eq!(found.content_hash.as_deref(), Some("abc123"));

    let missing =
      lookup_file_by_path(&connection, 1, "src/missing.py").expect("lookup must succeed");
    assert!(missing.is_none());
  }

  #[test]
  fn inserts_symbols_edges_and_looks_up_symbols_by_name() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-a");
    seed_repository(&connection, "/tmp/repo-b");

    let repo_a_file = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/a.py".to_string(),
        language: Some("python".to_string()),
        content_hash: None,
        line_count: Some(20),
        last_index_run_id: None,
      },
    )
    .expect("repo-a file insert must succeed");
    let repo_b_file = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 2,
        path: "src/b.py".to_string(),
        language: Some("python".to_string()),
        content_hash: None,
        line_count: Some(30),
        last_index_run_id: None,
      },
    )
    .expect("repo-b file insert must succeed");

    let repo_a_symbol_2 = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: repo_a_file.id,
        name: "validate_patient".to_string(),
        qualified_name: "src.a::validate_patient".to_string(),
        kind: "Function".to_string(),
        start_line: 5,
        end_line: 7,
        start_column: Some(0),
        end_column: Some(20),
      },
    )
    .expect("repo-a symbol 2 insert must succeed");
    let repo_a_symbol_1 = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: repo_a_file.id,
        name: "validate_patient".to_string(),
        qualified_name: "src.a::A.validate_patient".to_string(),
        kind: "Method".to_string(),
        start_line: 10,
        end_line: 12,
        start_column: Some(2),
        end_column: Some(24),
      },
    )
    .expect("repo-a symbol 1 insert must succeed");
    let repo_b_symbol = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: repo_b_file.id,
        name: "validate_patient".to_string(),
        qualified_name: "src.b::validate_patient".to_string(),
        kind: "Function".to_string(),
        start_line: 3,
        end_line: 8,
        start_column: Some(0),
        end_column: Some(20),
      },
    )
    .expect("repo-b symbol insert must succeed");

    let resolved_edge = insert_edge(
      &connection,
      &NewEdgeRecord {
        source_symbol_id: repo_a_symbol_1.id,
        target_symbol_id: Some(repo_a_symbol_2.id),
        unresolved_target: None,
        kind: "Calls".to_string(),
        confidence: Some(0.9),
      },
    )
    .expect("resolved edge insert must succeed");
    assert!(resolved_edge.id > 0);
    assert_eq!(resolved_edge.target_symbol_id, Some(repo_a_symbol_2.id));

    let unresolved_edge = insert_edge(
      &connection,
      &NewEdgeRecord {
        source_symbol_id: repo_a_symbol_1.id,
        target_symbol_id: None,
        unresolved_target: Some("external.mod::validate_patient".to_string()),
        kind: "Calls".to_string(),
        confidence: Some(0.5),
      },
    )
    .expect("unresolved edge insert must succeed");
    assert!(unresolved_edge.id > 0);
    assert_eq!(
      unresolved_edge.unresolved_target.as_deref(),
      Some("external.mod::validate_patient")
    );

    let repo_a_symbols = lookup_symbols_by_name(&connection, 1, "validate_patient")
      .expect("repo-a symbol lookup must succeed");
    assert_eq!(repo_a_symbols.len(), 2);
    assert_eq!(
      repo_a_symbols[0].qualified_name,
      "src.a::A.validate_patient"
    );
    assert_eq!(repo_a_symbols[1].qualified_name, "src.a::validate_patient");

    let repo_b_symbols = lookup_symbols_by_name(&connection, 2, "validate_patient")
      .expect("repo-b symbol lookup must succeed");
    assert_eq!(repo_b_symbols.len(), 1);
    assert_eq!(repo_b_symbols[0].id, repo_b_symbol.id);

    // Test list_symbols_for_repository
    let all_repo_a_symbols =
      list_symbols_for_repository(&connection, 1).expect("list repo-a symbols must succeed");
    assert_eq!(all_repo_a_symbols.len(), 2);
    assert_eq!(
      all_repo_a_symbols[0].qualified_name,
      "src.a::A.validate_patient"
    );
    assert_eq!(
      all_repo_a_symbols[1].qualified_name,
      "src.a::validate_patient"
    );

    // Test lookup_symbol_by_qualified_name
    let found_sym = lookup_symbol_by_qualified_name(&connection, 1, "src.a::A.validate_patient")
      .expect("lookup symbol by qualified name must succeed");
    assert!(found_sym.is_some());
    assert_eq!(found_sym.unwrap().name, "validate_patient");

    let not_found_sym = lookup_symbol_by_qualified_name(&connection, 1, "non_existent")
      .expect("lookup non-existent symbol must succeed");
    assert!(not_found_sym.is_none());
  }

  #[test]
  fn inserts_duplicate_symbols_enforces_upsert() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-a");

    let repo_a_file = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/a.py".to_string(),
        language: Some("python".to_string()),
        content_hash: None,
        line_count: Some(20),
        last_index_run_id: None,
      },
    )
    .expect("repo-a file insert must succeed");

    let sym1 = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: repo_a_file.id,
        name: "helper".to_string(),
        qualified_name: "src.a::helper".to_string(),
        kind: "Function".to_string(),
        start_line: 5,
        end_line: 7,
        start_column: Some(0),
        end_column: Some(20),
      },
    )
    .expect("first insert must succeed");

    let sym2 = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: repo_a_file.id,
        name: "helper".to_string(),
        qualified_name: "src.a::helper".to_string(),
        kind: "Function".to_string(),
        start_line: 10,
        end_line: 12,
        start_column: Some(4),
        end_column: Some(24),
      },
    )
    .expect("second duplicate insert must succeed via upsert");

    assert_eq!(sym1.id, sym2.id);
    assert_eq!(sym2.start_line, 10);
    assert_eq!(sym2.end_line, 12);
  }

  #[test]
  fn deletes_and_reindexes_a_file_path() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-a");

    let old_file = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/reindex.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("old-hash".to_string()),
        line_count: Some(11),
        last_index_run_id: None,
      },
    )
    .expect("old file insert must succeed");

    let old_symbol = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: old_file.id,
        name: "validate_patient".to_string(),
        qualified_name: "src.reindex::validate_patient".to_string(),
        kind: "Function".to_string(),
        start_line: 1,
        end_line: 3,
        start_column: Some(0),
        end_column: Some(18),
      },
    )
    .expect("old symbol insert must succeed");

    let deleted_count =
      delete_file_by_path(&connection, 1, "src/reindex.py").expect("file delete must succeed");
    assert_eq!(deleted_count, 1);

    let deleted_file =
      lookup_file_by_path(&connection, 1, "src/reindex.py").expect("lookup must succeed");
    assert!(deleted_file.is_none());

    let symbols_after_delete = lookup_symbols_by_name(&connection, 1, "validate_patient")
      .expect("symbol lookup must succeed");
    assert!(symbols_after_delete.is_empty());
    assert!(old_symbol.id > 0);

    let reindexed_file = reindex_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/reindex.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("new-hash".to_string()),
        line_count: Some(21),
        last_index_run_id: None,
      },
    )
    .expect("file reindex must succeed");

    assert_eq!(reindexed_file.content_hash.as_deref(), Some("new-hash"));
    let reindexed_lookup = lookup_file_by_path(&connection, 1, "src/reindex.py")
      .expect("lookup must succeed")
      .expect("reindexed file must exist");
    assert_eq!(reindexed_lookup.content_hash.as_deref(), Some("new-hash"));
    assert_eq!(reindexed_lookup.line_count, Some(21));
  }

  fn seed_repository(connection: &Connection, root_path: &str) {
    connection
      .execute(
        "INSERT INTO repositories (root_path, vcs_type) VALUES (?1, ?2)",
        (root_path, "git"),
      )
      .expect("repository insert must succeed");
  }

  #[test]
  fn db_status_reports_schema_not_initialized_when_core_tables_are_missing() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    connection
      .execute("CREATE TABLE random_table (id INTEGER PRIMARY KEY)", [])
      .expect("random table creation must succeed");

    let status = db_status(&connection).expect("db status must succeed");
    assert!(!status.schema_initialized);
    assert_eq!(status.files, 0);
    assert_eq!(status.symbols, 0);
  }

  #[test]
  fn db_status_reports_core_row_counts() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-a");

    let file = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/status.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("h1".to_string()),
        line_count: Some(4),
        last_index_run_id: None,
      },
    )
    .expect("file insert must succeed");
    insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file.id,
        name: "status_fn".to_string(),
        qualified_name: "src.status::status_fn".to_string(),
        kind: "Function".to_string(),
        start_line: 1,
        end_line: 2,
        start_column: Some(0),
        end_column: Some(9),
      },
    )
    .expect("symbol insert must succeed");

    let status = db_status(&connection).expect("db status must succeed");
    assert!(status.schema_initialized);
    assert_eq!(status.repositories, 1);
    assert_eq!(status.files, 1);
    assert_eq!(status.symbols, 1);
    assert_eq!(status.edges, 0);
    assert_eq!(status.docs, 0);
    assert_eq!(status.tests, 0);
    assert_eq!(status.index_runs, 0);
  }

  #[test]
  fn upserts_repository_by_root_path() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");

    let first = upsert_repository(&connection, "/tmp/repo-a").expect("first upsert must succeed");
    assert!(first.id > 0);
    assert_eq!(first.root_path, "/tmp/repo-a");
    assert_eq!(first.vcs_type, "git");

    let second = upsert_repository(&connection, "/tmp/repo-a").expect("second upsert must succeed");
    assert_eq!(first.id, second.id);

    let loaded = lookup_repository_by_root_path(&connection, "/tmp/repo-a")
      .expect("lookup must succeed")
      .expect("repository should exist");
    assert_eq!(loaded.id, first.id);
  }

  #[test]
  fn lists_files_for_repository_sorted_by_path() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-a");

    insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "b.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash-b".to_string()),
        line_count: Some(1),
        last_index_run_id: None,
      },
    )
    .expect("file insert must succeed");
    insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "a.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash-a".to_string()),
        line_count: Some(1),
        last_index_run_id: None,
      },
    )
    .expect("file insert must succeed");

    let files = list_files_for_repository(&connection, 1).expect("list must succeed");
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, vec!["a.py", "b.py"]);
  }

  #[test]
  fn starts_and_finishes_index_run_with_metadata() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    let repository =
      upsert_repository(&connection, "/tmp/repo-a").expect("repository upsert must succeed");

    let started = start_index_run(
      &connection,
      &NewIndexRunRecord {
        repository_id: repository.id,
        status: "running".to_string(),
      },
    )
    .expect("start index run must succeed");
    assert_eq!(started.status, "running");
    assert_eq!(started.files_scanned, 0);
    assert!(started.finished_at.is_none());

    let finished = finish_index_run(
      &connection,
      started.id,
      "completed",
      &FinishedIndexRunRecord {
        files_scanned: 3,
        files_changed: 2,
        files_deleted: 1,
        error_message: None,
      },
    )
    .expect("finish index run must succeed");
    assert_eq!(finished.status, "completed");
    assert_eq!(finished.files_scanned, 3);
    assert_eq!(finished.files_changed, 2);
    assert_eq!(finished.files_deleted, 1);
    assert!(finished.finished_at.is_some());
  }

  #[test]
  fn resolves_unresolved_edges_and_looks_up_imports_and_imported_by() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-a");

    // Insert file A (importer)
    let file_a = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/a.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash-a".to_string()),
        line_count: Some(10),
        last_index_run_id: None,
      },
    )
    .expect("file a insert must succeed");

    let module_a = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file_a.id,
        name: "a".to_string(),
        qualified_name: "src.a".to_string(),
        kind: "Module".to_string(),
        start_line: 1,
        end_line: 10,
        start_column: Some(0),
        end_column: Some(0),
      },
    )
    .expect("module a insert must succeed");

    // Insert unresolved edge from module a to src.b (which doesn't exist yet)
    let edge = insert_edge(
      &connection,
      &NewEdgeRecord {
        source_symbol_id: module_a.id,
        target_symbol_id: None,
        unresolved_target: Some("src.b".to_string()),
        kind: "Imports".to_string(),
        confidence: Some(1.0),
      },
    )
    .expect("edge insert must succeed");
    assert!(edge.target_symbol_id.is_none());
    assert_eq!(edge.unresolved_target.as_deref(), Some("src.b"));

    // Check imports for a.py - should be unresolved
    let imports_before =
      lookup_imports_by_file_path(&connection, 1, "src/a.py").expect("lookup imports must succeed");
    assert_eq!(imports_before.len(), 1);
    assert_eq!(imports_before[0].0, "src.b");
    assert_eq!(imports_before[0].1, None);
    assert_eq!(imports_before[0].2, Some("src.b".to_string()));

    // Now insert file B (imported)
    let file_b = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/b.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash-b".to_string()),
        line_count: Some(5),
        last_index_run_id: None,
      },
    )
    .expect("file b insert must succeed");

    let _module_b = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file_b.id,
        name: "b".to_string(),
        qualified_name: "src.b".to_string(),
        kind: "Module".to_string(),
        start_line: 1,
        end_line: 5,
        start_column: Some(0),
        end_column: Some(0),
      },
    )
    .expect("module b insert must succeed");

    // Resolve unresolved edges
    let resolved_count =
      resolve_unresolved_edges(&connection, 1).expect("resolve unresolved edges must succeed");
    assert_eq!(resolved_count, 1);

    // Verify imports are now resolved
    let imports_after =
      lookup_imports_by_file_path(&connection, 1, "src/a.py").expect("lookup imports must succeed");
    assert_eq!(imports_after.len(), 1);
    assert_eq!(imports_after[0].0, "src.b");
    assert_eq!(imports_after[0].1, Some("src/b.py".to_string()));
    assert_eq!(imports_after[0].2, None);

    // Verify imported-by for b.py shows a.py
    let imported_by = lookup_imported_by_file_path(&connection, 1, "src/b.py")
      .expect("lookup imported by must succeed");
    assert_eq!(imported_by.len(), 1);
    assert_eq!(imported_by[0].0, "src.a");
    assert_eq!(imported_by[0].1, "src/a.py");
  }

  #[test]
  fn test_reindex_and_delete_preserves_incoming_edges_as_unresolved() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-a");

    // Insert file A (importer)
    let file_a = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/a.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash-a".to_string()),
        line_count: Some(10),
        last_index_run_id: None,
      },
    )
    .expect("file a insert must succeed");

    let module_a = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file_a.id,
        name: "a".to_string(),
        qualified_name: "src.a".to_string(),
        kind: "Module".to_string(),
        start_line: 1,
        end_line: 10,
        start_column: Some(0),
        end_column: Some(0),
      },
    )
    .expect("module a insert must succeed");

    // Insert file B (imported)
    let file_b = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/b.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash-b".to_string()),
        line_count: Some(5),
        last_index_run_id: None,
      },
    )
    .expect("file b insert must succeed");

    let module_b = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file_b.id,
        name: "b".to_string(),
        qualified_name: "src.b".to_string(),
        kind: "Module".to_string(),
        start_line: 1,
        end_line: 5,
        start_column: Some(0),
        end_column: Some(0),
      },
    )
    .expect("module b insert must succeed");

    // Insert resolved edge A -> B
    let _edge = insert_edge(
      &connection,
      &NewEdgeRecord {
        source_symbol_id: module_a.id,
        target_symbol_id: Some(module_b.id),
        unresolved_target: None,
        kind: "Imports".to_string(),
        confidence: Some(1.0),
      },
    )
    .expect("edge insert must succeed");

    // Verify it is resolved
    let imports_before =
      lookup_imports_by_file_path(&connection, 1, "src/a.py").expect("lookup imports must succeed");
    assert_eq!(imports_before.len(), 1);
    assert_eq!(imports_before[0].1, Some("src/b.py".to_string()));

    // Now reindex file B (simulate file change)
    let _reindexed_b = reindex_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/b.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash-b-new".to_string()),
        line_count: Some(6),
        last_index_run_id: None,
      },
    )
    .expect("reindex must succeed");

    // Verify that the edge is NOT deleted, but is now UNRESOLVED
    let imports_after_reindex =
      lookup_imports_by_file_path(&connection, 1, "src/a.py").expect("lookup imports must succeed");
    assert_eq!(imports_after_reindex.len(), 1);
    assert_eq!(imports_after_reindex[0].1, None); // target file path is now None
    assert_eq!(imports_after_reindex[0].2, Some("src.b".to_string())); // unresolved target is now set!

    // Now re-insert the symbol module B (simulate indexing of B's symbols again)
    let _new_module_b = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: _reindexed_b.id,
        name: "b".to_string(),
        qualified_name: "src.b".to_string(),
        kind: "Module".to_string(),
        start_line: 1,
        end_line: 6,
        start_column: Some(0),
        end_column: Some(0),
      },
    )
    .expect("module b insert must succeed");

    // Resolve unresolved edges
    let resolved_count =
      resolve_unresolved_edges(&connection, 1).expect("resolve unresolved edges must succeed");
    assert_eq!(resolved_count, 1);

    // Verify it is resolved again!
    let imports_resolved =
      lookup_imports_by_file_path(&connection, 1, "src/a.py").expect("lookup imports must succeed");
    assert_eq!(imports_resolved.len(), 1);
    assert_eq!(imports_resolved[0].1, Some("src/b.py".to_string()));
    assert_eq!(imports_resolved[0].2, None);

    // Now test that deleting B also unresolves the edge rather than deleting it
    delete_file_by_path(&connection, 1, "src/b.py").expect("delete must succeed");
    let imports_after_delete =
      lookup_imports_by_file_path(&connection, 1, "src/a.py").expect("lookup imports must succeed");
    assert_eq!(imports_after_delete.len(), 1);
    assert_eq!(imports_after_delete[0].1, None);
    assert_eq!(imports_after_delete[0].2, Some("src.b".to_string()));
  }

  #[test]
  fn test_callers_and_callees_lookups_and_resolutions() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-a");

    // Insert file A (caller)
    let file_a = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/a.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash-a".to_string()),
        line_count: Some(10),
        last_index_run_id: None,
      },
    )
    .expect("file a insert must succeed");

    let caller_sym = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file_a.id,
        name: "helper_fn".to_string(),
        qualified_name: "src.a::helper_fn".to_string(),
        kind: "Function".to_string(),
        start_line: 1,
        end_line: 5,
        start_column: Some(0),
        end_column: Some(0),
      },
    )
    .expect("caller symbol insert must succeed");

    // Insert call edge: unresolved call to "validate_patient"
    insert_edge(
      &connection,
      &NewEdgeRecord {
        source_symbol_id: caller_sym.id,
        target_symbol_id: None,
        unresolved_target: Some("validate_patient".to_string()),
        kind: "Calls".to_string(),
        confidence: Some(0.7),
      },
    )
    .expect("edge insert must succeed");

    // Verify unresolved callees
    let callees_before = lookup_callees_by_symbol_name(&connection, 1, "src.a::helper_fn")
      .expect("lookup callees must succeed");
    assert_eq!(callees_before.len(), 1);
    assert_eq!(callees_before[0].0, "validate_patient");
    assert_eq!(callees_before[0].1, None); // target_kind
    assert_eq!(callees_before[0].2, None); // target_file_path
    assert_eq!(callees_before[0].3, Some(0.7));

    // Verify callers using simple name
    let callers_before = lookup_callers_by_symbol_name(&connection, 1, "validate_patient")
      .expect("lookup callers must succeed");
    assert_eq!(callers_before.len(), 1);
    assert_eq!(callers_before[0].0, "src.a::helper_fn");
    assert_eq!(callers_before[0].3, Some(0.7));

    // Now insert file B containing target symbol
    let file_b = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/b.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash-b".to_string()),
        line_count: Some(10),
        last_index_run_id: None,
      },
    )
    .expect("file b insert must succeed");

    let _callee_sym = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file_b.id,
        name: "validate_patient".to_string(),
        qualified_name: "src.b::validate_patient".to_string(),
        kind: "Function".to_string(),
        start_line: 1,
        end_line: 5,
        start_column: Some(0),
        end_column: Some(0),
      },
    )
    .expect("callee symbol insert must succeed");

    // Resolve unresolved edges
    let resolved_count = resolve_unresolved_edges(&connection, 1).expect("resolve must succeed");
    assert_eq!(resolved_count, 1);

    // Verify callee is now resolved!
    let callees_after = lookup_callees_by_symbol_name(&connection, 1, "src.a::helper_fn")
      .expect("lookup callees must succeed");
    assert_eq!(callees_after.len(), 1);
    assert_eq!(callees_after[0].0, "src.b::validate_patient");
    assert_eq!(callees_after[0].1, Some("Function".to_string()));
    assert_eq!(callees_after[0].2, Some("src/b.py".to_string()));

    // Verify caller using simple name
    let callers_after = lookup_callers_by_symbol_name(&connection, 1, "validate_patient")
      .expect("lookup callers must succeed");
    assert_eq!(callers_after.len(), 1);
    assert_eq!(callers_after[0].0, "src.a::helper_fn");
  }

  #[test]
  fn test_type_references_lookups_and_resolutions() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-a");

    // Insert file A (source symbol)
    let file_a = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/a.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash-a".to_string()),
        line_count: Some(10),
        last_index_run_id: None,
      },
    )
    .expect("file a insert must succeed");

    let source_sym = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file_a.id,
        name: "Patient".to_string(),
        qualified_name: "src.a::Patient".to_string(),
        kind: "Class".to_string(),
        start_line: 1,
        end_line: 5,
        start_column: Some(0),
        end_column: Some(0),
      },
    )
    .expect("source symbol insert must succeed");

    // Insert unresolved type edge to "BaseModel"
    insert_edge(
      &connection,
      &NewEdgeRecord {
        source_symbol_id: source_sym.id,
        target_symbol_id: None,
        unresolved_target: Some("BaseModel".to_string()),
        kind: "ReferencesType".to_string(),
        confidence: Some(1.0),
      },
    )
    .expect("edge insert must succeed");

    // Verify unresolved type reference (forward query)
    let refs_before = lookup_type_references_by_symbol_name(&connection, 1, "src.a::Patient")
      .expect("lookup forward type refs must succeed");
    assert_eq!(refs_before.len(), 1);
    assert_eq!(refs_before[0].0, "BaseModel");
    assert_eq!(refs_before[0].1, None); // target_kind
    assert_eq!(refs_before[0].2, None); // target_file_path

    // Verify type referencers using simple name (backward query)
    let referencers_before = lookup_type_referencers_by_symbol_name(&connection, 1, "BaseModel")
      .expect("lookup backward type referencers must succeed");
    assert_eq!(referencers_before.len(), 1);
    assert_eq!(referencers_before[0].0, "src.a::Patient");

    // Now insert file B containing BaseModel
    let file_b = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/b.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash-b".to_string()),
        line_count: Some(10),
        last_index_run_id: None,
      },
    )
    .expect("file b insert must succeed");

    let _target_sym = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file_b.id,
        name: "BaseModel".to_string(),
        qualified_name: "src.b::BaseModel".to_string(),
        kind: "Class".to_string(),
        start_line: 1,
        end_line: 5,
        start_column: Some(0),
        end_column: Some(0),
      },
    )
    .expect("target symbol insert must succeed");

    // Resolve unresolved edges
    let resolved_count = resolve_unresolved_edges(&connection, 1).expect("resolve must succeed");
    assert_eq!(resolved_count, 1);

    // Verify type reference is now resolved!
    let refs_after = lookup_type_references_by_symbol_name(&connection, 1, "src.a::Patient")
      .expect("lookup must succeed");
    assert_eq!(refs_after.len(), 1);
    assert_eq!(refs_after[0].0, "src.b::BaseModel");
    assert_eq!(refs_after[0].1, Some("Class".to_string()));
    assert_eq!(refs_after[0].2, Some("src/b.py".to_string()));

    // Verify type referencers using simple name
    let referencers_after = lookup_type_referencers_by_symbol_name(&connection, 1, "BaseModel")
      .expect("lookup must succeed");
    assert_eq!(referencers_after.len(), 1);
    assert_eq!(referencers_after[0].0, "src.a::Patient");
  }

  #[test]
  fn test_duplicate_edge_resolutions_deduplicated() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-a");

    let file_a = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/a.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash-a".to_string()),
        line_count: Some(10),
        last_index_run_id: None,
      },
    )
    .expect("file a insert must succeed");

    let source_sym = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file_a.id,
        name: "Patient".to_string(),
        qualified_name: "src.a::Patient".to_string(),
        kind: "Class".to_string(),
        start_line: 1,
        end_line: 5,
        start_column: Some(0),
        end_column: Some(0),
      },
    )
    .expect("source symbol insert must succeed");

    let file_b = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/b.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash-b".to_string()),
        line_count: Some(10),
        last_index_run_id: None,
      },
    )
    .expect("file b insert must succeed");

    let target_sym = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file_b.id,
        name: "BaseModel".to_string(),
        qualified_name: "src.b::BaseModel".to_string(),
        kind: "Class".to_string(),
        start_line: 1,
        end_line: 5,
        start_column: Some(0),
        end_column: Some(0),
      },
    )
    .expect("target symbol insert must succeed");

    // 1. Insert an ALREADY RESOLVED edge: Patient -> BaseModel
    insert_edge(
      &connection,
      &NewEdgeRecord {
        source_symbol_id: source_sym.id,
        target_symbol_id: Some(target_sym.id),
        unresolved_target: None,
        kind: "ReferencesType".to_string(),
        confidence: Some(1.0),
      },
    )
    .expect("resolved edge insert must succeed");

    // 2. Insert an UNRESOLVED edge to the same target using simple name "BaseModel"
    insert_edge(
      &connection,
      &NewEdgeRecord {
        source_symbol_id: source_sym.id,
        target_symbol_id: None,
        unresolved_target: Some("BaseModel".to_string()),
        kind: "ReferencesType".to_string(),
        confidence: Some(1.0),
      },
    )
    .expect("unresolved edge insert must succeed");

    // 3. Resolve unresolved edges: This should resolve the unresolved edge,
    // detect that it duplicates the already-resolved edge, and gracefully DELETE/ignore it
    // without causing a UNIQUE constraint failed error!
    let resolved_count = resolve_unresolved_edges(&connection, 1).expect("resolve must succeed");

    // The unresolved edge was resolved/deduplicated (deleted), so resolved_count is 0 (as it was deleted rather than updated)
    assert_eq!(resolved_count, 0);

    // Verify only 1 resolved edge exists for Patient -> BaseModel
    let refs = lookup_type_references_by_symbol_name(&connection, 1, "src.a::Patient")
      .expect("lookup must succeed");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].0, "src.b::BaseModel");
  }

  #[test]
  fn test_decorators_lookups_and_resolutions() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-a");

    // Insert file A (decorated function symbol)
    let file_a = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/a.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash-a".to_string()),
        line_count: Some(10),
        last_index_run_id: None,
      },
    )
    .expect("file a insert must succeed");

    let source_sym = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file_a.id,
        name: "validate_patient".to_string(),
        qualified_name: "src.a::validate_patient".to_string(),
        kind: "Function".to_string(),
        start_line: 1,
        end_line: 5,
        start_column: Some(0),
        end_column: Some(0),
      },
    )
    .expect("source symbol insert must succeed");

    // Insert unresolved decorator edge to "route"
    insert_edge(
      &connection,
      &NewEdgeRecord {
        source_symbol_id: source_sym.id,
        target_symbol_id: None,
        unresolved_target: Some("route".to_string()),
        kind: "ModifiedWith".to_string(),
        confidence: Some(1.0),
      },
    )
    .expect("edge insert must succeed");

    // Verify unresolved decorator (forward query)
    let decs_before = lookup_decorators_by_symbol_name(&connection, 1, "src.a::validate_patient")
      .expect("lookup forward decorators must succeed");
    assert_eq!(decs_before.len(), 1);
    assert_eq!(decs_before[0].0, "route");
    assert_eq!(decs_before[0].1, None); // target_kind
    assert_eq!(decs_before[0].2, None); // target_file_path

    // Verify decorated symbols (backward query)
    let decorated_before = lookup_decorated_symbols_by_symbol_name(&connection, 1, "route")
      .expect("lookup backward decorated symbols must succeed");
    assert_eq!(decorated_before.len(), 1);
    assert_eq!(decorated_before[0].0, "src.a::validate_patient");

    // Now insert file B containing route decorator definition
    let file_b = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/b.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash-b".to_string()),
        line_count: Some(10),
        last_index_run_id: None,
      },
    )
    .expect("file b insert must succeed");

    let _target_sym = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file_b.id,
        name: "route".to_string(),
        qualified_name: "src.b::route".to_string(),
        kind: "Function".to_string(),
        start_line: 1,
        end_line: 5,
        start_column: Some(0),
        end_column: Some(0),
      },
    )
    .expect("target symbol insert must succeed");

    // Resolve unresolved edges
    let resolved_count = resolve_unresolved_edges(&connection, 1).expect("resolve must succeed");
    assert_eq!(resolved_count, 1);

    // Verify decorator is now resolved!
    let decs_after = lookup_decorators_by_symbol_name(&connection, 1, "src.a::validate_patient")
      .expect("lookup must succeed");
    assert_eq!(decs_after.len(), 1);
    assert_eq!(decs_after[0].0, "src.b::route");
    assert_eq!(decs_after[0].1, Some("Function".to_string()));
    assert_eq!(decs_after[0].2, Some("src/b.py".to_string()));

    // Verify backward lookups
    let decorated_after = lookup_decorated_symbols_by_symbol_name(&connection, 1, "route")
      .expect("lookup must succeed");
    assert_eq!(decorated_after.len(), 1);
    assert_eq!(decorated_after[0].0, "src.a::validate_patient");

    // --- Dotted external/local resolver behavior assertions ---

    // 1. Insert a local "fixture" symbol
    let _fixture_sym = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file_b.id,
        name: "fixture".to_string(),
        qualified_name: "src.b::fixture".to_string(),
        kind: "Function".to_string(),
        start_line: 1,
        end_line: 5,
        start_column: Some(0),
        end_column: Some(0),
      },
    )
    .expect("local fixture symbol insert must succeed");

    // 2. Insert unresolved decorator to "pytest.fixture" (dotted external)
    insert_edge(
      &connection,
      &NewEdgeRecord {
        source_symbol_id: source_sym.id,
        target_symbol_id: None,
        unresolved_target: Some("pytest.fixture".to_string()),
        kind: "ModifiedWith".to_string(),
        confidence: Some(1.0),
      },
    )
    .expect("edge insert must succeed");

    // 3. Insert unresolved decorator to "my_module.my_decorator" (local qualified)
    insert_edge(
      &connection,
      &NewEdgeRecord {
        source_symbol_id: source_sym.id,
        target_symbol_id: None,
        unresolved_target: Some("my_module.my_decorator".to_string()),
        kind: "ModifiedWith".to_string(),
        confidence: Some(1.0),
      },
    )
    .expect("edge insert must succeed");

    // And insert a local symbol representing "my_module.my_decorator" (my_module::my_decorator)
    let _dec_sym = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file_b.id,
        name: "my_decorator".to_string(),
        qualified_name: "my_module::my_decorator".to_string(),
        kind: "Function".to_string(),
        start_line: 1,
        end_line: 5,
        start_column: Some(0),
        end_column: Some(0),
      },
    )
    .expect("local my_decorator symbol insert must succeed");

    // Resolve unresolved edges
    let resolved_count_2 = resolve_unresolved_edges(&connection, 1).expect("resolve must succeed");
    // Only "my_module.my_decorator" is resolved (via its qualified "my_module::my_decorator" match).
    // "pytest.fixture" is NOT resolved to "src.b::fixture" (since external dotted targets avoid the basename fallback).
    assert_eq!(resolved_count_2, 1);

    // Let's verify decorators on "src.a::validate_patient"
    let decs_final = lookup_decorators_by_symbol_name(&connection, 1, "src.a::validate_patient")
      .expect("lookup must succeed");

    // We expect 3 decorators:
    // - "src.b::route" [Resolved]
    // - "my_module::my_decorator" [Resolved]
    // - "pytest.fixture" [Unresolved]
    let resolved_targets: Vec<&str> = decs_final.iter().map(|d| d.0.as_str()).collect();
    assert!(resolved_targets.contains(&"src.b::route"));
    assert!(resolved_targets.contains(&"my_module::my_decorator"));
    assert!(resolved_targets.contains(&"pytest.fixture"));

    // Verify "pytest.fixture" is unresolved (no file path)
    let pytest_dec = decs_final.iter().find(|d| d.0 == "pytest.fixture").unwrap();
    assert_eq!(pytest_dec.2, None); // file path is None because it is unresolved!
  }

  #[test]
  fn test_tests_table_insertion_and_cascade() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-a");

    let file_a = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "tests/test_auth.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash-test".to_string()),
        line_count: Some(20),
        last_index_run_id: None,
      },
    )
    .expect("file insert must succeed");

    // Insert a test class
    let t_class = insert_test(
      &connection,
      &NewTestRecord {
        file_id: file_a.id,
        name: "TestAuth".to_string(),
        qualified_name: "tests.test_auth::TestAuth".to_string(),
        kind: "Class".to_string(),
        is_parametrized: false,
        framework: "pytest".to_string(),
        start_line: Some(5),
        end_line: Some(15),
      },
    )
    .expect("test class insert must succeed");

    // Insert a test function
    let t_func = insert_test(
      &connection,
      &NewTestRecord {
        file_id: file_a.id,
        name: "test_login".to_string(),
        qualified_name: "tests.test_auth::TestAuth.test_login".to_string(),
        kind: "Function".to_string(),
        is_parametrized: true,
        framework: "pytest".to_string(),
        start_line: Some(8),
        end_line: Some(12),
      },
    )
    .expect("test function insert must succeed");

    assert_eq!(t_class.kind, "Class");
    assert!(!t_class.is_parametrized);
    assert_eq!(t_func.kind, "Function");
    assert!(t_func.is_parametrized);

    // Lookup tests by file id
    let tests = lookup_tests_by_file_id(&connection, file_a.id).expect("lookup must succeed");
    assert_eq!(tests.len(), 2);
    assert_eq!(tests[0].name, "TestAuth");
    assert_eq!(tests[1].name, "test_login");

    // Verify deletion cascade
    delete_file_by_path(&connection, 1, "tests/test_auth.py").expect("delete file must succeed");
    let tests_after = lookup_tests_by_file_id(&connection, file_a.id).expect("lookup must succeed");
    assert_eq!(tests_after.len(), 0);
  }

  #[test]
  fn test_docs_indexing_resolution_and_cascade() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-a");

    let file_py = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/a.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash-py".to_string()),
        line_count: Some(15),
        last_index_run_id: None,
      },
    )
    .expect("file insert must succeed");

    let file_md = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "README.md".to_string(),
        language: Some("markdown".to_string()),
        content_hash: Some("hash-md".to_string()),
        line_count: Some(50),
        last_index_run_id: None,
      },
    )
    .expect("file insert must succeed");

    let sym_fn = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file_py.id,
        name: "validate_patient".to_string(),
        qualified_name: "src.a::validate_patient".to_string(),
        kind: "Function".to_string(),
        start_line: 1,
        end_line: 5,
        start_column: Some(0),
        end_column: Some(0),
      },
    )
    .expect("symbol insert must succeed");

    // 1. Insert a docstring directly owned by the symbol
    let doc1 = insert_doc(
      &connection,
      &NewDocRecord {
        file_id: file_py.id,
        symbol_id: Some(sym_fn.id),
        title: None,
        body: "Validates a patient record.".to_string(),
        start_line: Some(2),
        end_line: Some(4),
        source_kind: "Docstring".to_string(),
      },
    )
    .expect("insert docstring must succeed");

    assert_eq!(doc1.symbol_id, Some(sym_fn.id));
    assert_eq!(doc1.source_kind, "Docstring");

    // 2. Insert a markdown section with same-name heading
    let doc2 = insert_doc(
      &connection,
      &NewDocRecord {
        file_id: file_md.id,
        symbol_id: None,
        title: Some("validate_patient".to_string()),
        body: "Detailed section describing validate_patient function.".to_string(),
        start_line: Some(10),
        end_line: Some(15),
        source_kind: "Markdown".to_string(),
      },
    )
    .expect("insert markdown section must succeed");

    assert_eq!(doc2.symbol_id, None);

    // 3. Resolve symbol-doc linkages
    let links = resolve_symbol_doc_linkages(&connection, 1).expect("resolve must succeed");
    assert_eq!(links, 1);

    let links_second_run =
      resolve_symbol_doc_linkages(&connection, 1).expect("resolve must succeed");
    assert_eq!(links_second_run, 0);

    // Verify markdown section is now linked to sym_fn
    let doc2_after = get_doc_by_id(&connection, doc2.id)
      .expect("lookup doc2 must succeed")
      .expect("doc2 must exist");
    assert_eq!(doc2_after.symbol_id, Some(sym_fn.id));

    // Lookup docs for symbol
    let docs =
      lookup_docs_for_symbol(&connection, 1, "validate_patient").expect("lookup must succeed");
    assert_eq!(docs.len(), 2);
    let kinds: Vec<&str> = docs.iter().map(|d| d.source_kind.as_str()).collect();
    assert!(kinds.contains(&"Docstring"));
    assert!(kinds.contains(&"Markdown"));

    // Search docs
    let search_res = search_docs(&connection, 1, "Validates").expect("search must succeed");
    assert_eq!(search_res.len(), 1);
    assert_eq!(search_res[0].0.id, doc1.id);
    assert_eq!(search_res[0].1, "src/a.py");

    // Verify deletion cascade on file delete
    delete_file_by_path(&connection, 1, "src/a.py").expect("delete file must succeed");
    let docs_after_delete =
      lookup_docs_for_symbol(&connection, 1, "validate_patient").expect("lookup must succeed");
    assert_eq!(docs_after_delete.len(), 0);
  }

  #[test]
  fn test_routes_indexing_persistence_and_cascade() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-a");

    let file_py = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/api.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash-py".to_string()),
        line_count: Some(25),
        last_index_run_id: None,
      },
    )
    .expect("file insert must succeed");

    let sym_fn = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file_py.id,
        name: "read_item".to_string(),
        qualified_name: "src.api::read_item".to_string(),
        kind: "Function".to_string(),
        start_line: 10,
        end_line: 15,
        start_column: Some(0),
        end_column: Some(0),
      },
    )
    .expect("symbol insert must succeed");

    // Insert a route
    let route = insert_route(
      &connection,
      &NewRouteRecord {
        file_id: file_py.id,
        symbol_id: Some(sym_fn.id),
        handler_name: "read_item".to_string(),
        qualified_name: "src.api::read_item".to_string(),
        method: "GET".to_string(),
        path: "/items/{item_id}".to_string(),
        response_model: Some("ItemResponse".to_string()),
        start_line: Some(8),
        end_line: Some(15),
      },
    )
    .expect("route insert must succeed");

    assert_eq!(route.handler_name, "read_item");
    assert_eq!(route.method, "GET");
    assert_eq!(route.path, "/items/{item_id}");
    assert_eq!(route.response_model, Some("ItemResponse".to_string()));
    assert_eq!(route.symbol_id, Some(sym_fn.id));

    // Lookup routes by file id
    let routes_by_file =
      lookup_routes_by_file_id(&connection, file_py.id).expect("lookup by file must succeed");
    assert_eq!(routes_by_file.len(), 1);
    assert_eq!(routes_by_file[0].id, route.id);

    // Lookup routes for repository
    let repo_routes =
      lookup_routes_for_repository(&connection, 1).expect("lookup by repository must succeed");
    assert_eq!(repo_routes.len(), 1);
    assert_eq!(repo_routes[0].id, route.id);

    // Verify status includes routes count
    let status = db_status(&connection).expect("db_status must succeed");
    assert_eq!(status.routes, 1);

    // Verify deletion cascade on file deletion
    delete_file_by_path(&connection, 1, "src/api.py").expect("delete file must succeed");
    let routes_after =
      lookup_routes_by_file_id(&connection, file_py.id).expect("lookup must succeed");
    assert_eq!(routes_after.len(), 0);

    let status_after = db_status(&connection).expect("db_status must succeed");
    assert_eq!(status_after.routes, 0);
  }

  #[test]
  fn test_pydantic_indexing_persistence_and_cascade() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-a");

    let file_py = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: 1,
        path: "src/models.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash-py".to_string()),
        line_count: Some(50),
        last_index_run_id: None,
      },
    )
    .expect("file insert must succeed");

    let sym_class = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file_py.id,
        name: "Patient".to_string(),
        qualified_name: "src.models::Patient".to_string(),
        kind: "Class".to_string(),
        start_line: 10,
        end_line: 40,
        start_column: Some(0),
        end_column: Some(0),
      },
    )
    .expect("symbol insert must succeed");

    // Insert a pydantic model
    let model = insert_pydantic_model(
      &connection,
      &NewPydanticModelRecord {
        file_id: file_py.id,
        symbol_id: Some(sym_class.id),
        name: "Patient".to_string(),
        qualified_name: "src.models::Patient".to_string(),
        start_line: 10,
        end_line: 40,
      },
    )
    .expect("model insert must succeed");

    assert_eq!(model.name, "Patient");
    assert_eq!(model.qualified_name, "src.models::Patient");
    assert_eq!(model.symbol_id, Some(sym_class.id));

    // Insert a field
    let field = insert_pydantic_field(
      &connection,
      &NewPydanticFieldRecord {
        model_id: model.id,
        name: "age".to_string(),
        type_annotation: "int".to_string(),
        default_value: Some("30".to_string()),
        is_required: false,
      },
    )
    .expect("field insert must succeed");

    assert_eq!(field.name, "age");
    assert_eq!(field.type_annotation, "int");
    assert_eq!(field.default_value, Some("30".to_string()));
    assert!(!field.is_required);

    // Insert a validator
    let validator = insert_pydantic_validator(
      &connection,
      &NewPydanticValidatorRecord {
        model_id: model.id,
        name: "check_age".to_string(),
        validator_type: "field".to_string(),
        target_fields: "age".to_string(),
      },
    )
    .expect("validator insert must succeed");

    assert_eq!(validator.name, "check_age");
    assert_eq!(validator.validator_type, "field");
    assert_eq!(validator.target_fields, "age");

    // Lookups
    let repo_models = lookup_pydantic_models_for_repository(&connection, 1)
      .expect("lookup by repository must succeed");
    assert_eq!(repo_models.len(), 1);
    assert_eq!(repo_models[0].id, model.id);

    let model_by_name = lookup_pydantic_model_by_name(&connection, 1, "Patient")
      .expect("lookup by name must succeed")
      .expect("model must exist");
    assert_eq!(model_by_name.id, model.id);

    let fields =
      lookup_pydantic_fields_for_model(&connection, model.id).expect("lookup fields must succeed");
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].id, field.id);

    let validators = lookup_pydantic_validators_for_model(&connection, model.id)
      .expect("lookup validators must succeed");
    assert_eq!(validators.len(), 1);
    assert_eq!(validators[0].id, validator.id);

    // db_status row counts
    let status = db_status(&connection).expect("db_status must succeed");
    assert_eq!(status.pydantic_models, 1);
    assert_eq!(status.pydantic_fields, 1);
    assert_eq!(status.pydantic_validators, 1);

    // Cascade deletion on file deletion
    delete_file_by_path(&connection, 1, "src/models.py").expect("delete file must succeed");
    let status_after = db_status(&connection).expect("db_status must succeed");
    assert_eq!(status_after.pydantic_models, 0);
    assert_eq!(status_after.pydantic_fields, 0);
    assert_eq!(status_after.pydantic_validators, 0);
  }

  #[test]
  fn test_cargo_workspace_indexing() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-cargo");

    // Insert cargo package
    let pkg = insert_cargo_package(
      &connection,
      &NewCargoPackageRecord {
        repository_id: 1,
        name: "rkg-indexer".to_string(),
        manifest_path: "crates/rkg-indexer/Cargo.toml".to_string(),
        version: "0.1.0".to_string(),
        is_workspace_member: true,
      },
    )
    .expect("cargo package insert must succeed");

    assert_eq!(pkg.name, "rkg-indexer");
    assert_eq!(pkg.manifest_path, "crates/rkg-indexer/Cargo.toml");
    assert_eq!(pkg.version, "0.1.0");
    assert!(pkg.is_workspace_member);

    // Insert cargo dependency
    let dep = insert_cargo_dependency(
      &connection,
      &NewCargoDependencyRecord {
        package_id: pkg.id,
        name: "tokio".to_string(),
        version_requirement: Some("1.35.0".to_string()),
        is_workspace_dependency: false,
        features: "full,sync".to_string(),
        is_dev: false,
      },
    )
    .expect("cargo dependency insert must succeed");

    assert_eq!(dep.name, "tokio");
    assert_eq!(dep.version_requirement, Some("1.35.0".to_string()));
    assert_eq!(dep.features, "full,sync");
    assert!(!dep.is_workspace_dependency);
    assert!(!dep.is_dev);

    // List packages and dependencies
    let pkgs = list_cargo_packages(&connection, 1).expect("list packages must succeed");
    assert_eq!(pkgs.len(), 1);
    assert_eq!(pkgs[0].name, "rkg-indexer");

    let deps =
      list_cargo_dependencies(&connection, pkg.id).expect("list dependencies must succeed");
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].name, "tokio");

    // Verify status row counts
    let status = db_status(&connection).expect("db_status must succeed");
    assert_eq!(status.cargo_packages, 1);
    assert_eq!(status.cargo_dependencies, 1);
  }

  #[test]
  fn test_fsharp_workspace_indexing() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-fsharp");

    // Insert F# project
    let proj = insert_fsharp_project(
      &connection,
      &NewFSharpProjectRecord {
        repository_id: 1,
        name: "MyLib".to_string(),
        project_path: "src/MyLib/MyLib.fsproj".to_string(),
        target_framework: Some("net8.0".to_string()),
        is_solution_member: true,
      },
    )
    .expect("F# project insert must succeed");

    assert_eq!(proj.name, "MyLib");
    assert_eq!(proj.project_path, "src/MyLib/MyLib.fsproj");
    assert_eq!(proj.target_framework.as_deref(), Some("net8.0"));
    assert!(proj.is_solution_member);

    // Insert F# dependency
    let dep = insert_fsharp_dependency(
      &connection,
      &NewFSharpDependencyRecord {
        project_id: proj.id,
        name: "Newtonsoft.Json".to_string(),
        dependency_type: "package".to_string(),
        version_requirement: Some("13.0.3".to_string()),
      },
    )
    .expect("F# dependency insert must succeed");

    assert_eq!(dep.name, "Newtonsoft.Json");
    assert_eq!(dep.dependency_type, "package");
    assert_eq!(dep.version_requirement.as_deref(), Some("13.0.3"));

    // List F# projects and dependencies
    let projs = list_fsharp_projects(&connection, 1).expect("list projects must succeed");
    assert_eq!(projs.len(), 1);
    assert_eq!(projs[0].name, "MyLib");

    let deps =
      list_fsharp_dependencies(&connection, proj.id).expect("list dependencies must succeed");
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].name, "Newtonsoft.Json");

    // Verify status row counts
    let status = db_status(&connection).expect("db_status must succeed");
    assert_eq!(status.fsharp_projects, 1);
    assert_eq!(status.fsharp_dependencies, 1);
  }

  #[test]
  fn test_kotlin_workspace_indexing() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-kotlin");

    // Insert Kotlin project
    let proj = insert_kotlin_project(
      &connection,
      &NewKotlinProjectRecord {
        repository_id: 1,
        name: "app".to_string(),
        project_path: "app/build.gradle.kts".to_string(),
        target_framework: Some("jvm".to_string()),
        is_solution_member: true,
        generated_source_dirs: None,
      },
    )
    .expect("Kotlin project insert must succeed");

    assert_eq!(proj.name, "app");
    assert_eq!(proj.project_path, "app/build.gradle.kts");
    assert_eq!(proj.target_framework.as_deref(), Some("jvm"));
    assert!(proj.is_solution_member);

    // Insert Kotlin dependency
    let dep = insert_kotlin_dependency(
      &connection,
      &NewKotlinDependencyRecord {
        project_id: proj.id,
        name: "org.jetbrains.kotlinx:kotlinx-coroutines-core".to_string(),
        dependency_type: "package".to_string(),
        version_requirement: Some("1.7.3".to_string()),
      },
    )
    .expect("Kotlin dependency insert must succeed");

    assert_eq!(dep.name, "org.jetbrains.kotlinx:kotlinx-coroutines-core");
    assert_eq!(dep.dependency_type, "package");
    assert_eq!(dep.version_requirement.as_deref(), Some("1.7.3"));

    // List Kotlin projects and dependencies
    let projs = list_kotlin_projects(&connection, 1).expect("list projects must succeed");
    assert_eq!(projs.len(), 1);
    assert_eq!(projs[0].name, "app");

    let deps =
      list_kotlin_dependencies(&connection, proj.id).expect("list dependencies must succeed");
    assert_eq!(deps.len(), 1);
    assert_eq!(
      deps[0].name,
      "org.jetbrains.kotlinx:kotlinx-coroutines-core"
    );

    // Verify status row counts
    let status = db_status(&connection).expect("db_status must succeed");
    assert_eq!(status.kotlin_projects, 1);
    assert_eq!(status.kotlin_dependencies, 1);

    // Prune stale projects
    delete_kotlin_projects_not_in_paths(&connection, 1, &[]).expect("prune must succeed");
    let projs_after = list_kotlin_projects(&connection, 1).expect("list must succeed");
    assert_eq!(projs_after.len(), 0);

    let status_after = db_status(&connection).expect("db_status must succeed");
    assert_eq!(status_after.kotlin_projects, 0);
    assert_eq!(status_after.kotlin_dependencies, 0);
  }

  #[test]
  fn test_swift_workspace_indexing() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");
    seed_repository(&connection, "/tmp/repo-swift");

    // Insert Swift project
    let proj = insert_swift_project(
      &connection,
      &NewSwiftProjectRecord {
        repository_id: 1,
        name: "MySwiftPackage".to_string(),
        project_path: "Package.swift".to_string(),
        target_framework: Some("5.9".to_string()),
        is_solution_member: true,
      },
    )
    .expect("Swift project insert must succeed");

    assert_eq!(proj.name, "MySwiftPackage");
    assert_eq!(proj.project_path, "Package.swift");
    assert_eq!(proj.target_framework.as_deref(), Some("5.9"));
    assert!(proj.is_solution_member);

    // Insert Swift dependency
    let dep = insert_swift_dependency(
      &connection,
      &NewSwiftDependencyRecord {
        project_id: proj.id,
        name: "swift-algorithms".to_string(),
        dependency_type: "package".to_string(),
        version_requirement: Some("1.2.0".to_string()),
      },
    )
    .expect("Swift dependency insert must succeed");

    assert_eq!(dep.name, "swift-algorithms");
    assert_eq!(dep.dependency_type, "package");
    assert_eq!(dep.version_requirement.as_deref(), Some("1.2.0"));

    // List Swift projects and dependencies
    let projs = list_swift_projects(&connection, 1).expect("list projects must succeed");
    assert_eq!(projs.len(), 1);
    assert_eq!(projs[0].name, "MySwiftPackage");

    let deps =
      list_swift_dependencies(&connection, proj.id).expect("list dependencies must succeed");
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].name, "swift-algorithms");

    // Verify status row counts
    let status = db_status(&connection).expect("db_status must succeed");
    assert_eq!(status.swift_projects, 1);
    assert_eq!(status.swift_dependencies, 1);

    // Prune stale projects
    delete_swift_projects_not_in_paths(&connection, 1, &[]).expect("prune must succeed");
    let projs_after = list_swift_projects(&connection, 1).expect("list must succeed");
    assert_eq!(projs_after.len(), 0);

    let status_after = db_status(&connection).expect("db_status must succeed");
    assert_eq!(status_after.swift_projects, 0);
    assert_eq!(status_after.swift_dependencies, 0);
  }

  #[test]
  fn test_concurrency_persistence_and_cascade() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_schema(&connection).unwrap();

    let repo_id = upsert_repository(&connection, "/repo").unwrap().id;
    let file = reindex_file(
      &connection,
      &NewFileRecord {
        repository_id: repo_id,
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        content_hash: Some("abc".to_string()),
        line_count: Some(10),
        last_index_run_id: None,
      },
    )
    .unwrap();

    let symbol = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file.id,
        name: "main".to_string(),
        qualified_name: "src::lib::main".to_string(),
        kind: "Function".to_string(),
        start_line: 1,
        end_line: 10,
        start_column: None,
        end_column: None,
      },
    )
    .unwrap();

    let _spawn = insert_concurrency_spawn(
      &connection,
      &NewConcurrencySpawnRecord {
        file_id: file.id,
        source_symbol_qualified_name: symbol.qualified_name.clone(),
        spawn_kind: "tokio::spawn".to_string(),
        target_name: Some("worker".to_string()),
        start_line: 4,
        end_line: 6,
      },
    )
    .unwrap();

    let _channel = insert_concurrency_channel(
      &connection,
      &NewConcurrencyChannelRecord {
        file_id: file.id,
        source_symbol_qualified_name: symbol.qualified_name.clone(),
        channel_kind: "mpsc".to_string(),
        tx_name: "tx".to_string(),
        rx_name: "rx".to_string(),
        start_line: 2,
        end_line: 2,
      },
    )
    .unwrap();

    let _select = insert_concurrency_select(
      &connection,
      &NewConcurrencySelectRecord {
        file_id: file.id,
        source_symbol_qualified_name: symbol.qualified_name.clone(),
        start_line: 8,
        end_line: 9,
      },
    )
    .unwrap();

    // Verify wildcard escaping
    let symbol_foo_bar = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file.id,
        name: "foo_bar".to_string(),
        qualified_name: "src::lib::foo_bar".to_string(),
        kind: "Function".to_string(),
        start_line: 11,
        end_line: 15,
        start_column: None,
        end_column: None,
      },
    )
    .unwrap();

    let _symbol_fooxbar = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file.id,
        name: "fooxbar".to_string(),
        qualified_name: "src::lib::fooxbar".to_string(),
        kind: "Function".to_string(),
        start_line: 16,
        end_line: 20,
        start_column: None,
        end_column: None,
      },
    )
    .unwrap();

    let _spawn_fooxbar = insert_concurrency_spawn(
      &connection,
      &NewConcurrencySpawnRecord {
        file_id: file.id,
        source_symbol_qualified_name: "src::lib::fooxbar::nested".to_string(),
        spawn_kind: "tokio::spawn".to_string(),
        target_name: Some("fooxbar_worker".to_string()),
        start_line: 18,
        end_line: 19,
      },
    )
    .unwrap();

    // Looking up foo_bar should NOT return fooxbar's spawns because of wildcard escaping
    let foo_bar_spawns =
      lookup_concurrency_spawns_for_symbol(&connection, repo_id, &symbol_foo_bar.qualified_name)
        .unwrap();
    assert!(foo_bar_spawns.is_empty());

    let spawns =
      lookup_concurrency_spawns_for_symbol(&connection, repo_id, &symbol.qualified_name).unwrap();
    assert_eq!(spawns.len(), 1);
    assert_eq!(spawns[0].target_name, Some("worker".to_string()));

    let channels =
      lookup_concurrency_channels_for_symbol(&connection, repo_id, &symbol.qualified_name).unwrap();
    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0].tx_name, "tx");

    let selects =
      lookup_concurrency_selects_for_symbol(&connection, repo_id, &symbol.qualified_name).unwrap();
    assert_eq!(selects.len(), 1);
    assert_eq!(selects[0].start_line, 8);

    delete_file_by_path(&connection, repo_id, "src/lib.rs").unwrap();

    let spawns_after =
      lookup_concurrency_spawns_for_symbol(&connection, repo_id, &symbol.qualified_name).unwrap();
    assert!(spawns_after.is_empty());

    let channels_after =
      lookup_concurrency_channels_for_symbol(&connection, repo_id, &symbol.qualified_name).unwrap();
    assert!(channels_after.is_empty());

    let selects_after =
      lookup_concurrency_selects_for_symbol(&connection, repo_id, &symbol.qualified_name).unwrap();
    assert!(selects_after.is_empty());
  }

  #[test]
  fn test_safety_persistence_and_cascade() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_schema(&connection).unwrap();

    let repo_id = upsert_repository(&connection, "/repo").unwrap().id;
    let file = reindex_file(
      &connection,
      &NewFileRecord {
        repository_id: repo_id,
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        content_hash: Some("abc".to_string()),
        line_count: Some(10),
        last_index_run_id: None,
      },
    )
    .unwrap();

    let symbol = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file.id,
        name: "main".to_string(),
        qualified_name: "src::lib::main".to_string(),
        kind: "Function".to_string(),
        start_line: 1,
        end_line: 10,
        start_column: None,
        end_column: None,
      },
    )
    .unwrap();

    let _block = insert_rust_unsafe_block(
      &connection,
      &NewRustUnsafeBlockRecord {
        file_id: file.id,
        source_symbol_qualified_name: symbol.qualified_name.clone(),
        start_line: 4,
        end_line: 6,
      },
    )
    .unwrap();

    let _func = insert_rust_unsafe_function(
      &connection,
      &NewRustUnsafeFunctionRecord {
        file_id: file.id,
        qualified_name: "src::lib::my_unsafe_func".to_string(),
        start_line: 7,
        end_line: 9,
      },
    )
    .unwrap();

    let _ffi = insert_rust_ffi_binding(
      &connection,
      &NewRustFFIBindingRecord {
        file_id: file.id,
        source_symbol_qualified_name: symbol.qualified_name.clone(),
        foreign_item_name: "c_func".to_string(),
        abi: "C".to_string(),
        start_line: 2,
        end_line: 3,
      },
    )
    .unwrap();

    // Verify lookup by symbol
    let blocks =
      lookup_rust_unsafe_blocks_for_symbol(&connection, repo_id, &symbol.qualified_name).unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].start_line, 4);

    let funcs =
      lookup_rust_unsafe_functions_for_symbol(&connection, repo_id, "src::lib::my_unsafe_func")
        .unwrap();
    assert_eq!(funcs.len(), 1);
    assert_eq!(funcs[0].qualified_name, "src::lib::my_unsafe_func");

    let ffis =
      lookup_rust_ffi_bindings_for_symbol(&connection, repo_id, &symbol.qualified_name).unwrap();
    assert_eq!(ffis.len(), 1);
    assert_eq!(ffis[0].foreign_item_name, "c_func");

    // Verify lookup by file
    let file_blocks =
      lookup_rust_unsafe_blocks_for_file(&connection, repo_id, "src/lib.rs").unwrap();
    assert_eq!(file_blocks.len(), 1);

    let file_funcs =
      lookup_rust_unsafe_functions_for_file(&connection, repo_id, "src/lib.rs").unwrap();
    assert_eq!(file_funcs.len(), 1);

    let file_ffis = lookup_rust_ffi_bindings_for_file(&connection, repo_id, "src/lib.rs").unwrap();
    assert_eq!(file_ffis.len(), 1);

    // Verify status row counts
    let status = db_status(&connection).unwrap();
    assert_eq!(status.rust_unsafe_blocks, 1);
    assert_eq!(status.rust_unsafe_functions, 1);
    assert_eq!(status.rust_ffi_bindings, 1);

    // Verify cascade deletes
    delete_file_by_path(&connection, repo_id, "src/lib.rs").unwrap();

    let blocks_after =
      lookup_rust_unsafe_blocks_for_symbol(&connection, repo_id, &symbol.qualified_name).unwrap();
    assert!(blocks_after.is_empty());

    let funcs_after =
      lookup_rust_unsafe_functions_for_symbol(&connection, repo_id, "src::lib::my_unsafe_func")
        .unwrap();
    assert!(funcs_after.is_empty());

    let ffis_after =
      lookup_rust_ffi_bindings_for_symbol(&connection, repo_id, &symbol.qualified_name).unwrap();
    assert!(ffis_after.is_empty());
  }

  #[test]
  fn test_sqlite_fts5_indexing_and_triggers() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");

    // Insert repository
    let repo_id = upsert_repository(&connection, "/Users/test/patient-service")
      .expect("insert repo must succeed")
      .id;

    // Insert file
    let file = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: repo_id,
        path: "src/Patient.rs".to_string(),
        language: Some("rust".to_string()),
        content_hash: Some("hash123".to_string()),
        line_count: Some(100),
        last_index_run_id: None,
      },
    )
    .expect("insert file must succeed");

    // Insert symbol
    let symbol = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file.id,
        name: "PatientManager".to_string(),
        qualified_name: "src.Patient::PatientManager".to_string(),
        kind: "struct".to_string(),
        start_line: 10,
        end_line: 45,
        start_column: Some(0),
        end_column: Some(1),
      },
    )
    .expect("insert symbol must succeed");

    // Insert document
    let doc = insert_doc(
      &connection,
      &NewDocRecord {
        file_id: file.id,
        symbol_id: Some(symbol.id),
        title: Some("Patient registration and checkout".to_string()),
        body: "This class registers patients, records medical details, and bills them.".to_string(),
        start_line: Some(5),
        end_line: Some(9),
        source_kind: "docstring".to_string(),
      },
    )
    .expect("insert doc must succeed");

    // Assert FTS tables exist by running a simple FTS MATCH query directly on the virtual tables
    let doc_fts_count: i64 = connection
      .query_row(
        "SELECT count(*) FROM docs_fts WHERE docs_fts MATCH 'patient'",
        [],
        |r| r.get(0),
      )
      .expect("docs_fts table search must execute successfully");
    assert_eq!(doc_fts_count, 1);

    let symbol_fts_count: i64 = connection
      .query_row(
        "SELECT count(*) FROM symbols_fts WHERE symbols_fts MATCH 'PatientManager'",
        [],
        |r| r.get(0),
      )
      .expect("symbols_fts table search must execute successfully");
    assert_eq!(symbol_fts_count, 1);

    // Call FTS Search helper functions (these will fail on stubs)
    let doc_results = search_docs_fts(&connection, repo_id, "medical details").unwrap();
    assert_eq!(doc_results.len(), 1);
    assert_eq!(doc_results[0].0.id, doc.id);
    assert_eq!(doc_results[0].1, "src/Patient.rs");

    let symbol_results = search_symbols_fts(&connection, repo_id, "PatientManager").unwrap();
    assert_eq!(symbol_results.len(), 1);
    assert_eq!(symbol_results[0].0.id, symbol.id);
    assert_eq!(symbol_results[0].1, "src/Patient.rs");

    // Test Cascade Deletion: delete the file and verify both core and FTS tables are automatically cleaned up
    delete_file_by_path(&connection, repo_id, "src/Patient.rs").unwrap();

    let doc_fts_count_after: i64 = connection
      .query_row(
        "SELECT count(*) FROM docs_fts WHERE docs_fts MATCH 'patient'",
        [],
        |r| r.get(0),
      )
      .unwrap_or(0);
    assert_eq!(doc_fts_count_after, 0);

    let symbol_fts_count_after: i64 = connection
      .query_row(
        "SELECT count(*) FROM symbols_fts WHERE symbols_fts MATCH 'PatientManager'",
        [],
        |r| r.get(0),
      )
      .unwrap_or(0);
    assert_eq!(symbol_fts_count_after, 0);
  }

  #[test]
  fn test_fts_backfill_on_schema_upgrade() {
    // Simulate a database that already contains docs/symbols rows before FTS tables
    // were introduced. We achieve this by: (1) running schema init to create base tables,
    // (2) dropping the FTS virtual tables and triggers, (3) inserting rows, then
    // (4) running schema init again — which should backfill FTS for the pre-existing rows.
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("first schema init must succeed");

    let repo_id = upsert_repository(&connection, "/backfill-test")
      .expect("upsert repo must succeed")
      .id;
    let file = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: repo_id,
        path: "src/old.rs".to_string(),
        language: Some("rust".to_string()),
        content_hash: Some("abc".to_string()),
        line_count: None,
        last_index_run_id: None,
      },
    )
    .expect("insert file must succeed");

    // Drop FTS tables and triggers to simulate pre-FTS database state
    connection
      .execute_batch(
        "DROP TABLE IF EXISTS docs_fts;
       DROP TABLE IF EXISTS symbols_fts;
       DROP TRIGGER IF EXISTS docs_ai;
       DROP TRIGGER IF EXISTS docs_ad;
       DROP TRIGGER IF EXISTS symbols_ai;
       DROP TRIGGER IF EXISTS symbols_ad;",
      )
      .expect("drop FTS artifacts must succeed");

    // Insert rows WITHOUT triggers active — these rows would be missed by old code
    let symbol = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file.id,
        name: "OldStruct".to_string(),
        qualified_name: "src.old::OldStruct".to_string(),
        kind: "struct".to_string(),
        start_line: 1,
        end_line: 10,
        start_column: Some(0),
        end_column: Some(1),
      },
    )
    .expect("insert symbol must succeed");
    let doc = insert_doc(
      &connection,
      &NewDocRecord {
        file_id: file.id,
        symbol_id: Some(symbol.id),
        title: Some("OldStruct documentation".to_string()),
        body: "Legacy struct written before FTS was available.".to_string(),
        start_line: Some(1),
        end_line: Some(3),
        source_kind: "docstring".to_string(),
      },
    )
    .expect("insert doc must succeed");

    // Re-run schema init — backfill INSERT should populate FTS for the pre-existing rows
    initialize_schema(&connection).expect("second schema init (upgrade) must succeed");

    let doc_hit: i64 = connection
      .query_row(
        "SELECT count(*) FROM docs_fts WHERE docs_fts MATCH 'Legacy'",
        [],
        |r| r.get(0),
      )
      .expect("docs_fts query must succeed");
    assert_eq!(
      doc_hit, 1,
      "pre-existing doc row must be backfilled into docs_fts"
    );

    let sym_hit: i64 = connection
      .query_row(
        "SELECT count(*) FROM symbols_fts WHERE symbols_fts MATCH 'OldStruct'",
        [],
        |r| r.get(0),
      )
      .expect("symbols_fts query must succeed");
    assert_eq!(
      sym_hit, 1,
      "pre-existing symbol row must be backfilled into symbols_fts"
    );

    // Verify that running schema init a third time does not duplicate FTS rows
    initialize_schema(&connection).expect("third schema init must succeed");

    let doc_dedup: i64 = connection
      .query_row(
        "SELECT count(*) FROM docs_fts WHERE docs_fts MATCH 'Legacy'",
        [],
        |r| r.get(0),
      )
      .expect("docs_fts dedup query must succeed");
    assert_eq!(
      doc_dedup, 1,
      "backfill must be idempotent — no duplicate FTS rows"
    );

    let _ = (doc.id, symbol.id); // suppress unused warnings
  }

  #[test]
  fn test_sanitize_query_preserves_dotted_names() {
    // Dotted qualified names must survive sanitization so LIKE fallback can still
    // match `pkg.module.Class` stored in `qualified_name`.
    assert_eq!(
      sanitize_query_for_like("pkg.module.Class"),
      "pkg.module.Class"
    );
    assert_eq!(
      sanitize_query_for_like("src::lib::my_func"),
      "src::lib::my_func"
    );
    assert_eq!(sanitize_query_for_like("some-name_here"), "some-name_here");
    // FTS5-specific characters that have no meaning in a LIKE pattern are still removed
    assert_eq!(sanitize_query_for_like("hello\"world"), "helloworld");
    assert_eq!(sanitize_query_for_like("foo*bar"), "foobar");
    // Leading/trailing whitespace is trimmed
    assert_eq!(sanitize_query_for_like("  trim me  "), "trim me");
  }

  #[test]
  fn test_symbol_coverage_db_lifecycle() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");

    let repo = upsert_repository(&connection, "/test/repo").unwrap();
    let file = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: repo.id,
        path: "src/main.rs".to_string(),
        language: Some("rust".to_string()),
        content_hash: None,
        line_count: None,
        last_index_run_id: None,
      },
    )
    .unwrap();

    let symbol = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file.id,
        name: "main".to_string(),
        qualified_name: "src/main.rs::main".to_string(),
        kind: "function".to_string(),
        start_line: 1,
        end_line: 10,
        start_column: None,
        end_column: None,
      },
    )
    .unwrap();

    let coverage_record = NewSymbolCoverageRecord {
      file_id: file.id,
      symbol_id: symbol.id,
      report_path: "coverage.xml".to_string(),
      test_suite: Some("unit".to_string()),
      lines_valid: 10,
      lines_covered: 8,
      branches_valid: 4,
      branches_covered: 3,
      coverable_lines: "1,2,3,4,5,6,7,8,9,10".to_string(),
      uncovered_lines: "9,10".to_string(),
    };

    let inserted = insert_symbol_coverage(&connection, &coverage_record).unwrap();
    assert_eq!(inserted.lines_covered, 8);
    assert_eq!(inserted.uncovered_lines, "9,10");

    let retrieved = list_symbol_coverage_for_symbol(&connection, symbol.id).unwrap();
    assert_eq!(retrieved.len(), 1);
    assert_eq!(retrieved[0].test_suite.as_deref(), Some("unit"));

    // Idempotent delete:
    delete_symbol_coverage_by_report(&connection, "coverage.xml", Some("unit")).unwrap();
    let after_delete = list_symbol_coverage_for_symbol(&connection, symbol.id).unwrap();
    assert_eq!(after_delete.len(), 0);

    // Re-insert and test cascade on file delete:
    let _ = insert_symbol_coverage(&connection, &coverage_record).unwrap();
    delete_file_by_path(&connection, repo.id, "src/main.rs").unwrap();
    let after_cascade = list_symbol_coverage_for_symbol(&connection, symbol.id).unwrap();
    assert_eq!(after_cascade.len(), 0);
  }

  #[test]
  fn test_android_components_and_resources_db_lifecycle() {
    let connection = Connection::open_in_memory().expect("in-memory db must open");
    initialize_schema(&connection).expect("schema initialization must succeed");

    let repo = upsert_repository(&connection, "/test/repo").unwrap();
    let file = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: repo.id,
        path: "AndroidManifest.xml".to_string(),
        language: Some("xml".to_string()),
        content_hash: None,
        line_count: None,
        last_index_run_id: None,
      },
    )
    .unwrap();

    let comp = NewAndroidComponentRecord {
      file_id: file.id,
      name: "MainActivity".to_string(),
      component_type: "activity".to_string(),
      class_name: "com.example.app.MainActivity".to_string(),
      permission: Some("android.permission.INTERNET".to_string()),
      intent_actions: vec!["android.intent.action.MAIN".to_string()],
      intent_categories: vec!["android.intent.category.LAUNCHER".to_string()],
      start_line: Some(5),
      end_line: Some(15),
    };

    let inserted_comp = insert_android_component(&connection, &comp).unwrap();
    assert_eq!(inserted_comp.name, "MainActivity");
    assert_eq!(inserted_comp.intent_actions.len(), 1);
    assert_eq!(
      inserted_comp.intent_actions[0],
      "android.intent.action.MAIN"
    );

    let retrieved_comps = lookup_android_components_for_repository(&connection, repo.id).unwrap();
    assert_eq!(retrieved_comps.len(), 1);
    assert_eq!(
      retrieved_comps[0].class_name,
      "com.example.app.MainActivity"
    );

    let res = NewAndroidResourceRecord {
      file_id: file.id,
      name: "activity_main".to_string(),
      resource_type: "layout".to_string(),
      value: None,
      start_line: Some(1),
      end_line: Some(10),
    };

    let inserted_res = insert_android_resource(&connection, &res).unwrap();
    assert_eq!(inserted_res.name, "activity_main");
    assert_eq!(inserted_res.resource_type, "layout");

    let retrieved_res = lookup_android_resources_for_repository(&connection, repo.id).unwrap();
    assert_eq!(retrieved_res.len(), 1);
    assert_eq!(retrieved_res[0].name, "activity_main");

    // Test cascade delete
    delete_file_by_path(&connection, repo.id, "AndroidManifest.xml").unwrap();
    let after_delete_comps =
      lookup_android_components_for_repository(&connection, repo.id).unwrap();
    let after_delete_res = lookup_android_resources_for_repository(&connection, repo.id).unwrap();
    assert_eq!(after_delete_comps.len(), 0);
    assert_eq!(after_delete_res.len(), 0);
  }
}
