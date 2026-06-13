pub const CRATE_NAME: &str = "rkg-query";

use rusqlite::Connection;

/// Import relationship row: imported qualified name, resolved file path, resolved target symbol.
pub type ImportRow = (String, Option<String>, Option<String>);
/// Reverse import row: importing symbol qualified name and source file path.
pub type ImportedByRow = (String, String);
/// Reverse relationship row: source symbol, source file path, edge kind, and confidence.
pub type ReverseRelationshipRow = (String, String, String, Option<f64>);
/// Forward relationship row: target name, resolved file path, resolved target symbol, confidence.
pub type ForwardRelationshipRow = (String, Option<String>, Option<String>, Option<f64>);
/// Test relationship row: test qualified name, test file path, and confidence.
pub type TestRelationshipRow = (String, String, Option<f64>);
/// Fixture row: fixture name and optional resolved file path.
pub type FixtureRow = (String, Option<String>);
/// Documentation search row: matching doc record and source kind.
pub type DocSearchRow = (rkg_db::DocRecord, String);
pub type SymbolSearchRow = (rkg_db::SymbolRecord, String);

pub fn get_imports_for_file(
  connection: &Connection,
  repository_id: i64,
  file_path: &str,
) -> rusqlite::Result<Vec<ImportRow>> {
  rkg_db::lookup_imports_by_file_path(connection, repository_id, file_path)
}

pub fn get_imported_by_for_file(
  connection: &Connection,
  repository_id: i64,
  file_path: &str,
) -> rusqlite::Result<Vec<ImportedByRow>> {
  rkg_db::lookup_imported_by_file_path(connection, repository_id, file_path)
}

pub fn get_callers(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> rusqlite::Result<Vec<ReverseRelationshipRow>> {
  rkg_db::lookup_callers_by_symbol_name(connection, repository_id, name)
}

pub fn get_callees(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> rusqlite::Result<Vec<ForwardRelationshipRow>> {
  rkg_db::lookup_callees_by_symbol_name(connection, repository_id, name)
}

pub fn get_type_references(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> rusqlite::Result<Vec<ForwardRelationshipRow>> {
  rkg_db::lookup_type_references_by_symbol_name(connection, repository_id, name)
}

pub fn get_type_referencers(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> rusqlite::Result<Vec<ReverseRelationshipRow>> {
  rkg_db::lookup_type_referencers_by_symbol_name(connection, repository_id, name)
}

pub fn get_decorators(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> rusqlite::Result<Vec<ForwardRelationshipRow>> {
  rkg_db::lookup_decorators_by_symbol_name(connection, repository_id, name)
}

pub fn get_decorated_symbols(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> rusqlite::Result<Vec<ReverseRelationshipRow>> {
  rkg_db::lookup_decorated_symbols_by_symbol_name(connection, repository_id, name)
}

pub fn get_tests_for_symbol(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> rusqlite::Result<Vec<TestRelationshipRow>> {
  rkg_db::lookup_tests_for_symbol(connection, repository_id, name)
}

pub fn get_test_deps(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> rusqlite::Result<Vec<ForwardRelationshipRow>> {
  rkg_db::lookup_test_deps(connection, repository_id, name)
}

pub fn get_fixtures_for_test(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> rusqlite::Result<Vec<FixtureRow>> {
  rkg_db::lookup_fixtures_for_test(connection, repository_id, name)
}

pub fn get_docs_for_symbol(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> rusqlite::Result<Vec<rkg_db::DocRecord>> {
  rkg_db::lookup_docs_for_symbol(connection, repository_id, name)
}

pub fn search_docs(
  connection: &Connection,
  repository_id: i64,
  query: &str,
) -> rusqlite::Result<Vec<DocSearchRow>> {
  rkg_db::search_docs(connection, repository_id, query)
}

pub fn search_docs_fts(
  connection: &Connection,
  repository_id: i64,
  query: &str,
) -> rusqlite::Result<Vec<DocSearchRow>> {
  rkg_db::search_docs_fts(connection, repository_id, query)
}

pub fn search_symbols_fts(
  connection: &Connection,
  repository_id: i64,
  query: &str,
) -> rusqlite::Result<Vec<SymbolSearchRow>> {
  rkg_db::search_symbols_fts(connection, repository_id, query)
}

pub fn get_git_metadata_for_file(
  connection: &Connection,
  repository_id: i64,
  file_path: &str,
) -> rusqlite::Result<rkg_core::GitFileMetadata> {
  let churn = rkg_db::get_file_churn(connection, repository_id, file_path)?;
  let last_commit =
    rkg_db::get_last_modified_commit_for_file(connection, repository_id, file_path)?;
  let author_frequency =
    rkg_db::get_author_frequency_for_file(connection, repository_id, file_path)?;

  Ok(rkg_core::GitFileMetadata {
    path: file_path.to_string(),
    churn,
    last_commit,
    author_frequency,
  })
}

pub fn analyze_cochanges(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> rusqlite::Result<rkg_core::CochangeAnalysis> {
  let is_file = {
    let mut stmt =
      connection.prepare("SELECT 1 FROM files WHERE repository_id = ?1 AND path = ?2 LIMIT 1")?;
    let exists = stmt.exists((repository_id, name))?;
    exists
      || name.contains('/')
      || name.contains('\\')
      || name.ends_with(".py")
      || name.ends_with(".rs")
      || name.ends_with(".md")
      || name.ends_with(".toml")
      || name.ends_with(".json")
  };

  if is_file {
    let file_churn = rkg_db::get_file_churn(connection, repository_id, name)?;
    let raw_file_cochanges = rkg_db::get_file_cochanges(connection, repository_id, name)?;

    let mut file_cochanges = Vec::new();
    for (co_path, count) in raw_file_cochanges {
      let rate = if file_churn > 0 {
        (count as f64 / file_churn as f64) * 100.0
      } else {
        0.0
      };
      file_cochanges.push(rkg_core::CochangeRecord {
        name: co_path,
        count,
        rate,
      });
    }

    Ok(rkg_core::CochangeAnalysis {
      target: name.to_string(),
      churn: file_churn,
      symbol_cochanges: Vec::new(),
      file_cochanges,
    })
  } else {
    let start_symbols = resolve_start_symbols(connection, repository_id, name)?;
    if start_symbols.is_empty() {
      return Ok(rkg_core::CochangeAnalysis {
        target: name.to_string(),
        churn: 0,
        symbol_cochanges: Vec::new(),
        file_cochanges: Vec::new(),
      });
    }

    let symbol = &start_symbols[0];
    let symbol_churn = rkg_db::get_symbol_churn(connection, repository_id, &symbol.qualified_name)?;

    let raw_symbol_cochanges =
      rkg_db::get_symbol_cochanges(connection, repository_id, &symbol.qualified_name)?;
    let mut symbol_cochanges = Vec::new();
    for (co_qname, count) in raw_symbol_cochanges {
      let rate = if symbol_churn > 0 {
        (count as f64 / symbol_churn as f64) * 100.0
      } else {
        0.0
      };
      symbol_cochanges.push(rkg_core::CochangeRecord {
        name: co_qname,
        count,
        rate,
      });
    }

    let mut stmt = connection.prepare(
      "SELECT f.path FROM symbols s INNER JOIN files f ON s.file_id = f.id WHERE s.id = ?1",
    )?;
    let file_path: String = stmt.query_row([symbol.id], |row| row.get(0))?;
    let file_churn = rkg_db::get_file_churn(connection, repository_id, &file_path)?;
    let raw_file_cochanges = rkg_db::get_file_cochanges(connection, repository_id, &file_path)?;
    let mut file_cochanges = Vec::new();
    for (co_path, count) in raw_file_cochanges {
      let rate = if file_churn > 0 {
        (count as f64 / file_churn as f64) * 100.0
      } else {
        0.0
      };
      file_cochanges.push(rkg_core::CochangeRecord {
        name: co_path,
        count,
        rate,
      });
    }

    Ok(rkg_core::CochangeAnalysis {
      target: symbol.qualified_name.clone(),
      churn: symbol_churn,
      symbol_cochanges,
      file_cochanges,
    })
  }
}

use rusqlite::OptionalExtension;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraversalDirection {
  Forward,
  Backward,
}

#[derive(Debug, Clone)]
pub struct TraversalNode {
  pub symbol: rkg_db::SymbolRecord,
  pub file_path: String,
  pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct TraversalEdge {
  pub source_symbol_id: i64,
  pub target_symbol_id: i64,
  pub kind: String,
  pub confidence: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct TraversalResult {
  pub nodes: Vec<TraversalNode>,
  pub edges: Vec<TraversalEdge>,
}

#[derive(Debug, Clone)]
pub struct AffectedTest {
  pub test_qualified_name: String,
  pub test_kind: String,
  pub file_path: String,
  pub linked_symbol_qualified_name: String,
  pub confidence: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct AffectedDoc {
  pub title: Option<String>,
  pub body: String,
  pub file_path: String,
  pub start_line: Option<i64>,
  pub end_line: Option<i64>,
  pub linked_symbol_qualified_name: String,
}

#[derive(Debug, Clone)]
pub struct ImpactAnalysisResult {
  pub target_symbols: Vec<rkg_db::SymbolRecord>,
  pub downstream_nodes: Vec<TraversalNode>,
  pub downstream_edges: Vec<TraversalEdge>,
  pub upstream_nodes: Vec<TraversalNode>,
  pub upstream_edges: Vec<TraversalEdge>,
  pub affected_tests: Vec<AffectedTest>,
  pub affected_docs: Vec<AffectedDoc>,
}

pub fn resolve_start_symbols(
  connection: &Connection,
  repository_id: i64,
  name: &str,
) -> rusqlite::Result<Vec<rkg_db::SymbolRecord>> {
  if (name.contains("::") || name.contains('.'))
    && let Some(sym) = rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, name)?
  {
    return Ok(vec![sym]);
  }
  rkg_db::lookup_symbols_by_name(connection, repository_id, name)
}

pub fn traverse_relationships(
  connection: &Connection,
  repository_id: i64,
  start_symbol_ids: &[i64],
  edge_kinds: &[&str],
  direction: TraversalDirection,
  max_depth: usize,
) -> rusqlite::Result<TraversalResult> {
  if start_symbol_ids.is_empty() || max_depth == 0 || edge_kinds.is_empty() {
    return Ok(TraversalResult {
      nodes: Vec::new(),
      edges: Vec::new(),
    });
  }

  let mut visited = HashMap::new(); // symbol_id -> depth
  let mut queue = VecDeque::new();

  for &id in start_symbol_ids {
    visited.insert(id, 0);
    queue.push_back((id, 0));
  }

  let mut traversed_edges = Vec::new();
  let mut seen_edges = HashSet::new();

  let edge_kinds_placeholders = edge_kinds
    .iter()
    .map(|_| "?")
    .collect::<Vec<_>>()
    .join(", ");

  let sql = match direction {
    TraversalDirection::Forward => format!(
      "SELECT e.source_symbol_id, e.target_symbol_id, e.kind, e.confidence
       FROM edges e
       INNER JOIN symbols s ON e.target_symbol_id = s.id
       INNER JOIN files f ON s.file_id = f.id
       WHERE f.repository_id = ?1
         AND e.source_symbol_id = ?2
         AND e.target_symbol_id IS NOT NULL
         AND e.kind IN ({})",
      edge_kinds_placeholders
    ),
    TraversalDirection::Backward => format!(
      "SELECT e.source_symbol_id, e.target_symbol_id, e.kind, e.confidence
       FROM edges e
       INNER JOIN symbols s ON e.source_symbol_id = s.id
       INNER JOIN files f ON s.file_id = f.id
       WHERE f.repository_id = ?1
         AND e.target_symbol_id = ?2
         AND e.kind IN ({})",
      edge_kinds_placeholders
    ),
  };

  while let Some((current_id, depth)) = queue.pop_front() {
    if depth >= max_depth {
      continue;
    }

    let mut stmt = connection.prepare(&sql)?;

    let mut params: Vec<rusqlite::types::Value> = vec![
      rusqlite::types::Value::Integer(repository_id),
      rusqlite::types::Value::Integer(current_id),
    ];
    for kind in edge_kinds {
      params.push(rusqlite::types::Value::Text(kind.to_string()));
    }

    let params_ref = rusqlite::params_from_iter(params);
    let mut rows = stmt.query(params_ref)?;

    while let Some(row) = rows.next()? {
      let source_id: i64 = row.get(0)?;
      let target_id: i64 = row.get(1)?;
      let kind: String = row.get(2)?;
      let confidence: Option<f64> = row.get(3)?;

      let edge_key = (source_id, target_id, kind.clone());
      if seen_edges.insert(edge_key) {
        traversed_edges.push(TraversalEdge {
          source_symbol_id: source_id,
          target_symbol_id: target_id,
          kind,
          confidence,
        });
      }

      let neighbor_id = match direction {
        TraversalDirection::Forward => target_id,
        TraversalDirection::Backward => source_id,
      };

      if let std::collections::hash_map::Entry::Vacant(e) = visited.entry(neighbor_id) {
        e.insert(depth + 1);
        queue.push_back((neighbor_id, depth + 1));
      }
    }
  }

  let mut nodes = Vec::new();
  if !visited.is_empty() {
    for (symbol_id, depth) in visited {
      let mut stmt = connection.prepare(
        "SELECT s.id, s.file_id, s.name, s.qualified_name, s.kind, s.start_line, s.end_line, s.start_column, s.end_column, f.path
         FROM symbols s
         INNER JOIN files f ON s.file_id = f.id
         WHERE s.id = ?1"
      )?;
      let node_opt = stmt
        .query_row([symbol_id], |row| {
          Ok(TraversalNode {
            symbol: rkg_db::SymbolRecord {
              id: row.get(0)?,
              file_id: row.get(1)?,
              name: row.get(2)?,
              qualified_name: row.get(3)?,
              kind: row.get(4)?,
              start_line: row.get(5)?,
              end_line: row.get(6)?,
              start_column: row.get(7)?,
              end_column: row.get(8)?,
            },
            file_path: row.get(9)?,
            depth,
          })
        })
        .optional()?;

      if let Some(node) = node_opt {
        nodes.push(node);
      }
    }
  }

  nodes.sort_by(|a, b| {
    a.depth
      .cmp(&b.depth)
      .then(a.symbol.qualified_name.cmp(&b.symbol.qualified_name))
  });

  Ok(TraversalResult {
    nodes,
    edges: traversed_edges,
  })
}

pub fn analyze_impact(
  connection: &Connection,
  repository_id: i64,
  symbol_name: &str,
  max_depth: usize,
) -> rusqlite::Result<ImpactAnalysisResult> {
  let target_symbols = resolve_start_symbols(connection, repository_id, symbol_name)?;
  let target_ids: Vec<i64> = target_symbols.iter().map(|s| s.id).collect();

  if target_symbols.is_empty() {
    return Ok(ImpactAnalysisResult {
      target_symbols,
      downstream_nodes: Vec::new(),
      downstream_edges: Vec::new(),
      upstream_nodes: Vec::new(),
      upstream_edges: Vec::new(),
      affected_tests: Vec::new(),
      affected_docs: Vec::new(),
    });
  }

  let edge_kinds = &[
    "Calls",
    "Imports",
    "ReferencesType",
    "ModifiedWith",
    "Spawns",
    "SendsTo",
  ];

  let downstream_result = traverse_relationships(
    connection,
    repository_id,
    &target_ids,
    edge_kinds,
    TraversalDirection::Forward,
    max_depth,
  )?;

  let upstream_result = traverse_relationships(
    connection,
    repository_id,
    &target_ids,
    edge_kinds,
    TraversalDirection::Backward,
    max_depth,
  )?;

  let mut affected_tests = Vec::new();
  let mut seen_tests = HashSet::new();

  let mut symbols_to_query_tests = target_symbols.clone();
  for node in &upstream_result.nodes {
    if !symbols_to_query_tests.contains(&node.symbol) {
      symbols_to_query_tests.push(node.symbol.clone());
    }
  }

  for sym in &symbols_to_query_tests {
    let sql = "
      SELECT DISTINCT s_source.qualified_name, s_source.kind, f_source.path, e.confidence
      FROM edges e
      INNER JOIN symbols s_source ON e.source_symbol_id = s_source.id
      INNER JOIN files f_source ON s_source.file_id = f_source.id
      WHERE f_source.repository_id = ?1
        AND e.kind = 'TestedBy'
        AND (
          e.target_symbol_id = ?2
          OR e.unresolved_target = ?3
          OR e.unresolved_target = ?4
          OR e.unresolved_target LIKE '%::' || ?4
          OR e.unresolved_target LIKE '%.' || ?4
        )
      ORDER BY s_source.qualified_name ASC";

    let mut stmt = connection.prepare(sql)?;
    let rows = stmt.query_map(
      (repository_id, sym.id, &sym.qualified_name, &sym.name),
      |row| {
        Ok(AffectedTest {
          test_qualified_name: row.get(0)?,
          test_kind: row.get(1)?,
          file_path: row.get(2)?,
          linked_symbol_qualified_name: sym.qualified_name.clone(),
          confidence: row.get(3)?,
        })
      },
    )?;

    for test in rows.flatten() {
      let key = (
        test.test_qualified_name.clone(),
        test.linked_symbol_qualified_name.clone(),
      );
      if seen_tests.insert(key) {
        affected_tests.push(test);
      }
    }
  }

  let mut affected_docs = Vec::new();
  let mut seen_docs = HashSet::new();

  let mut symbols_to_query_docs = target_symbols.clone();
  for node in &downstream_result.nodes {
    if !symbols_to_query_docs.contains(&node.symbol) {
      symbols_to_query_docs.push(node.symbol.clone());
    }
  }
  for node in &upstream_result.nodes {
    if !symbols_to_query_docs.contains(&node.symbol) {
      symbols_to_query_docs.push(node.symbol.clone());
    }
  }

  for sym in &symbols_to_query_docs {
    let sql = "
      SELECT d.title, d.body, f.path, d.start_line, d.end_line
      FROM docs d
      INNER JOIN files f ON d.file_id = f.id
      WHERE f.repository_id = ?1
        AND d.symbol_id = ?2
      ORDER BY d.title ASC, d.id ASC";

    let mut stmt = connection.prepare(sql)?;
    let rows = stmt.query_map((repository_id, sym.id), |row| {
      Ok(AffectedDoc {
        title: row.get(0)?,
        body: row.get(1)?,
        file_path: row.get(2)?,
        start_line: row.get(3)?,
        end_line: row.get(4)?,
        linked_symbol_qualified_name: sym.qualified_name.clone(),
      })
    })?;

    for doc in rows.flatten() {
      let key = (
        doc.file_path.clone(),
        doc.start_line,
        doc.end_line,
        doc.linked_symbol_qualified_name.clone(),
      );
      if seen_docs.insert(key) {
        affected_docs.push(doc);
      }
    }
  }

  affected_tests.sort_by(|a, b| a.test_qualified_name.cmp(&b.test_qualified_name));
  affected_docs.sort_by(|a, b| {
    a.linked_symbol_qualified_name
      .cmp(&b.linked_symbol_qualified_name)
      .then(a.file_path.cmp(&b.file_path))
      .then(a.start_line.cmp(&b.start_line))
  });

  Ok(ImpactAnalysisResult {
    target_symbols,
    downstream_nodes: downstream_result.nodes,
    downstream_edges: downstream_result.edges,
    upstream_nodes: upstream_result.nodes,
    upstream_edges: upstream_result.edges,
    affected_tests,
    affected_docs,
  })
}

pub fn estimate_tokens(text: &str) -> usize {
  (text.chars().count() as f64 / 4.0).ceil() as usize
}

fn map_symbol_kind(kind: &str) -> rkg_core::SymbolKind {
  match kind {
    "Function" => rkg_core::SymbolKind::Function,
    "Method" => rkg_core::SymbolKind::Method,
    "Class" => rkg_core::SymbolKind::Class,
    "Trait" => rkg_core::SymbolKind::Trait,
    "Struct" => rkg_core::SymbolKind::Struct,
    "Enum" => rkg_core::SymbolKind::Enum,
    "Module" => rkg_core::SymbolKind::Module,
    "Interface" => rkg_core::SymbolKind::Interface,
    "TypeAlias" => rkg_core::SymbolKind::TypeAlias,
    _ => rkg_core::SymbolKind::Unknown,
  }
}

fn map_edge_kind(kind: &str) -> rkg_core::EdgeKind {
  match kind {
    "Imports" => rkg_core::EdgeKind::Imports,
    "Calls" => rkg_core::EdgeKind::Calls,
    "Defines" => rkg_core::EdgeKind::Defines,
    "Implements" => rkg_core::EdgeKind::Implements,
    "Extends" => rkg_core::EdgeKind::Extends,
    "ReferencesType" => rkg_core::EdgeKind::ReferencesType,
    "TestedBy" => rkg_core::EdgeKind::TestedBy,
    "DocumentedBy" => rkg_core::EdgeKind::DocumentedBy,
    "ConfiguredBy" => rkg_core::EdgeKind::ConfiguredBy,
    "ModifiedWith" => rkg_core::EdgeKind::ModifiedWith,
    "Spawns" => rkg_core::EdgeKind::Spawns,
    "SendsTo" => rkg_core::EdgeKind::SendsTo,
    _ => rkg_core::EdgeKind::Imports,
  }
}

fn get_file_span_content(
  repo_root: &std::path::Path,
  path_rel: &str,
  start_line: usize,
  end_line: usize,
) -> String {
  let file_path = repo_root.join(path_rel);
  match std::fs::read_to_string(&file_path) {
    Ok(content) => {
      let lines: Vec<&str> = content.lines().collect();
      let start_idx = start_line.saturating_sub(1);
      let end_idx = end_line.min(lines.len());
      if start_idx < lines.len() && start_idx < end_idx {
        lines[start_idx..end_idx].join("\n")
      } else {
        String::new()
      }
    }
    Err(_) => format!("<Failed to read source file: {}>", path_rel),
  }
}

pub fn format_context_pack_markdown(
  pack: &rkg_core::ContextPack,
  repo_root: &std::path::Path,
) -> String {
  let mut out = String::new();

  if let Some(target) = &pack.target {
    out.push_str(&format!("# rkg Context Pack: {}\n\n", target));
  } else {
    out.push_str("# rkg Context Pack\n\n");
  }

  if let Some(budget) = pack.token_budget {
    out.push_str(&format!("- Token Budget: {} tokens\n", budget));
  }

  let mut body = String::new();

  if !pack.files.is_empty() {
    body.push_str("## Discovered Files\n\n");
    for f in &pack.files {
      body.push_str(&format!(
        "- **{}** (Language: {:?}, Lines: {:?})\n",
        f.path,
        f.language.as_deref().unwrap_or("unknown"),
        f.line_count.unwrap_or(0)
      ));
    }
    body.push('\n');
  }

  if !pack.symbols.is_empty() {
    body.push_str("## Symbol Definitions\n\n");
    for sym in &pack.symbols {
      body.push_str(&format!(
        "### Symbol: `{}` ({:?})\n",
        sym.qualified_name, sym.kind
      ));
      body.push_str(&format!(
        "- File: `{}` (Lines {}-{})\n\n",
        sym.location.file_path, sym.location.start_line, sym.location.end_line
      ));

      let snippet = get_file_span_content(
        repo_root,
        &sym.location.file_path,
        sym.location.start_line,
        sym.location.end_line,
      );
      let lang_class = sym
        .location
        .file_path
        .split('.')
        .next_back()
        .unwrap_or("python");
      body.push_str(&format!(
        "```{}\n// File: {} (Lines {}-{})\n{}\n```\n\n",
        lang_class, sym.location.file_path, sym.location.start_line, sym.location.end_line, snippet
      ));
    }
  }

  if !pack.docs.is_empty() {
    body.push_str("## Documentation Blocks\n\n");
    for doc in &pack.docs {
      let title_str = doc.title.as_deref().unwrap_or("Documentation");
      body.push_str(&format!("### {}\n", title_str));
      body.push_str(&format!(
        "- Location: `{}` (Lines {}-{})\n\n",
        doc.location.file_path, doc.location.start_line, doc.location.end_line
      ));
      body.push_str(&format!("{}\n\n", doc.text.trim()));
    }
  }

  if !pack.edges.is_empty() {
    body.push_str("## Relationships\n\n");
    for edge in &pack.edges {
      body.push_str(&format!(
        "- `{}` -> `{:?}` -> `{}`\n",
        edge.source, edge.kind, edge.target
      ));
    }
    body.push('\n');
  }

  if !pack.tests.is_empty() {
    body.push_str("## Related Test Cases\n\n");
    for test in &pack.tests {
      body.push_str(&format!("### Test: `{}`\n", test.name));
      body.push_str(&format!(
        "- Location: `{}` (Lines {}-{})\n",
        test.location.file_path, test.location.start_line, test.location.end_line
      ));
      if !test.target_symbols.is_empty() {
        body.push_str(&format!(
          "- Target Symbols: {}\n",
          test.target_symbols.join(", ")
        ));
      }
      body.push('\n');

      let snippet = get_file_span_content(
        repo_root,
        &test.location.file_path,
        test.location.start_line,
        test.location.end_line,
      );
      let lang_class = test
        .location
        .file_path
        .split('.')
        .next_back()
        .unwrap_or("python");
      body.push_str(&format!(
        "```{}\n// File: {} (Lines {}-{})\n{}\n```\n\n",
        lang_class,
        test.location.file_path,
        test.location.start_line,
        test.location.end_line,
        snippet
      ));
    }
  }

  let temp_full = format!("{}{}", out, body);
  let est = estimate_tokens(&temp_full);
  out.push_str(&format!("- Estimated Tokens: {} tokens\n\n", est));
  out.push_str(&body);

  out
}

fn escape_json_string(s: &str) -> String {
  let mut escaped = String::new();
  for c in s.chars() {
    match c {
      '"' => escaped.push_str("\\\""),
      '\\' => escaped.push_str("\\\\"),
      '\n' => escaped.push_str("\\n"),
      '\r' => escaped.push_str("\\r"),
      '\t' => escaped.push_str("\\t"),
      _ if c.is_control() => escaped.push_str(&format!("\\u{:04x}", c as u32)),
      _ => escaped.push(c),
    }
  }
  escaped
}

pub fn format_context_pack_json(
  pack: &rkg_core::ContextPack,
  repo_root: &std::path::Path,
) -> String {
  let mut parts = Vec::new();

  parts.push(format!(
    "\"target\": {}",
    pack
      .target
      .as_ref()
      .map(|t| format!("\"{}\"", escape_json_string(t)))
      .unwrap_or_else(|| "null".to_string())
  ));
  parts.push(format!(
    "\"token_budget\": {}",
    pack
      .token_budget
      .map(|b| b.to_string())
      .unwrap_or_else(|| "null".to_string())
  ));

  let mut file_parts = Vec::new();
  for f in &pack.files {
    file_parts.push(format!(
      "{{\"path\": \"{}\", \"language\": {}, \"line_count\": {}}}",
      escape_json_string(&f.path),
      f.language
        .as_ref()
        .map(|l| format!("\"{}\"", escape_json_string(l)))
        .unwrap_or_else(|| "null".to_string()),
      f.line_count
        .map(|c| c.to_string())
        .unwrap_or_else(|| "null".to_string())
    ));
  }
  parts.push(format!("\"files\": [{}]", file_parts.join(", ")));

  let mut symbol_parts = Vec::new();
  for sym in &pack.symbols {
    let snippet = get_file_span_content(
      repo_root,
      &sym.location.file_path,
      sym.location.start_line,
      sym.location.end_line,
    );
    symbol_parts.push(format!(
      "{{\"name\": \"{}\", \"qualified_name\": \"{}\", \"kind\": \"{:?}\", \"location\": {{\"file_path\": \"{}\", \"start_line\": {}, \"end_line\": {}, \"start_column\": {}, \"end_column\": {}}}, \"code_snippet\": \"{}\"}}",
      escape_json_string(&sym.name),
      escape_json_string(&sym.qualified_name),
      sym.kind,
      escape_json_string(&sym.location.file_path),
      sym.location.start_line,
      sym.location.end_line,
      sym.location.start_column.map(|c| c.to_string()).unwrap_or_else(|| "null".to_string()),
      sym.location.end_column.map(|c| c.to_string()).unwrap_or_else(|| "null".to_string()),
      escape_json_string(&snippet)
    ));
  }
  parts.push(format!("\"symbols\": [{}]", symbol_parts.join(", ")));

  let mut doc_parts = Vec::new();
  for doc in &pack.docs {
    doc_parts.push(format!(
      "{{\"title\": {}, \"text\": \"{}\", \"location\": {{\"file_path\": \"{}\", \"start_line\": {}, \"end_line\": {}, \"start_column\": {}, \"end_column\": {}}}}}",
      doc.title.as_ref().map(|t| format!("\"{}\"", escape_json_string(t))).unwrap_or_else(|| "null".to_string()),
      escape_json_string(&doc.text),
      escape_json_string(&doc.location.file_path),
      doc.location.start_line,
      doc.location.end_line,
      doc.location.start_column.map(|c| c.to_string()).unwrap_or_else(|| "null".to_string()),
      doc.location.end_column.map(|c| c.to_string()).unwrap_or_else(|| "null".to_string())
    ));
  }
  parts.push(format!("\"docs\": [{}]", doc_parts.join(", ")));

  let mut edge_parts = Vec::new();
  for edge in &pack.edges {
    edge_parts.push(format!(
      "{{\"source\": \"{}\", \"target\": \"{}\", \"kind\": \"{:?}\"}}",
      escape_json_string(&edge.source),
      escape_json_string(&edge.target),
      edge.kind
    ));
  }
  parts.push(format!("\"edges\": [{}]", edge_parts.join(", ")));

  let mut test_parts = Vec::new();
  for test in &pack.tests {
    let snippet = get_file_span_content(
      repo_root,
      &test.location.file_path,
      test.location.start_line,
      test.location.end_line,
    );
    let target_sym_strs: Vec<String> = test
      .target_symbols
      .iter()
      .map(|s| format!("\"{}\"", escape_json_string(s)))
      .collect();
    test_parts.push(format!(
      "{{\"name\": \"{}\", \"location\": {{\"file_path\": \"{}\", \"start_line\": {}, \"end_line\": {}, \"start_column\": {}, \"end_column\": {}}}, \"target_symbols\": [{}], \"code_snippet\": \"{}\"}}",
      escape_json_string(&test.name),
      escape_json_string(&test.location.file_path),
      test.location.start_line,
      test.location.end_line,
      test.location.start_column.map(|c| c.to_string()).unwrap_or_else(|| "null".to_string()),
      test.location.end_column.map(|c| c.to_string()).unwrap_or_else(|| "null".to_string()),
      target_sym_strs.join(", "),
      escape_json_string(&snippet)
    ));
  }
  parts.push(format!("\"tests\": [{}]", test_parts.join(", ")));

  let temp_json = format!("{{ {} }}", parts.join(", "));
  let est = estimate_tokens(&temp_json);

  format!(
    "{{\n  \"estimated_tokens\": {},\n  {}\n}}",
    est,
    parts.join(",\n  ")
  )
}

pub fn pack_context(
  connection: &Connection,
  repository_id: i64,
  repo_root: &std::path::Path,
  symbol_name: &str,
  token_budget: Option<usize>,
  format: &str,
) -> rusqlite::Result<rkg_core::ContextPack> {
  let target_symbols = resolve_start_symbols(connection, repository_id, symbol_name)?;

  if target_symbols.is_empty() {
    return Ok(rkg_core::ContextPack {
      target: Some(symbol_name.to_string()),
      files: Vec::new(),
      symbols: Vec::new(),
      edges: Vec::new(),
      docs: Vec::new(),
      tests: Vec::new(),
      token_budget,
    });
  }

  let mut target_files = Vec::new();
  let mut target_core_symbols = Vec::new();
  let mut target_ids = Vec::new();

  for sym in &target_symbols {
    target_ids.push(sym.id);
    let mut stmt = connection.prepare("SELECT id, repository_id, path, language, content_hash, line_count, last_index_run_id FROM files WHERE id = ?1")?;
    let file_rec: rkg_db::FileRecord = stmt.query_row([sym.file_id], |row| {
      Ok(rkg_db::FileRecord {
        id: row.get(0)?,
        repository_id: row.get(1)?,
        path: row.get(2)?,
        language: row.get(3)?,
        content_hash: row.get(4)?,
        line_count: row.get(5)?,
        last_index_run_id: row.get(6)?,
      })
    })?;

    let file_core = rkg_core::File {
      path: file_rec.path.clone(),
      language: file_rec.language.clone(),
      hash: file_rec.content_hash.clone(),
      line_count: file_rec.line_count.map(|c| c as usize),
    };
    if !target_files.contains(&file_core) {
      target_files.push(file_core);
    }

    let symbol_core = rkg_core::Symbol {
      name: sym.name.clone(),
      qualified_name: sym.qualified_name.clone(),
      kind: map_symbol_kind(&sym.kind),
      location: rkg_core::Location {
        file_path: file_rec.path.clone(),
        start_line: sym.start_line as usize,
        end_line: sym.end_line as usize,
        start_column: sym.start_column.map(|c| c as usize),
        end_column: sym.end_column.map(|c| c as usize),
      },
    };
    target_core_symbols.push(symbol_core);
  }

  let mut target_docs = Vec::new();
  for sym in &target_symbols {
    let doc_records =
      rkg_db::lookup_docs_for_symbol(connection, repository_id, &sym.qualified_name)?;
    for doc in doc_records {
      let mut stmt = connection.prepare("SELECT path FROM files WHERE id = ?1")?;
      let file_path: String = stmt.query_row([doc.file_id], |row| row.get(0))?;
      target_docs.push(rkg_core::DocBlock {
        title: doc.title.clone(),
        text: doc.body.clone(),
        location: rkg_core::Location {
          file_path,
          start_line: doc.start_line.unwrap_or(1) as usize,
          end_line: doc.end_line.unwrap_or(1) as usize,
          start_column: None,
          end_column: None,
        },
      });
    }
  }

  let mut target_tests = Vec::new();
  for sym in &target_symbols {
    let test_records =
      rkg_db::lookup_tests_for_symbol(connection, repository_id, &sym.qualified_name)?;
    for (test_qname, test_file_path, _conf) in test_records {
      let mut stmt = connection.prepare(
        "SELECT name, start_line, end_line FROM tests WHERE qualified_name = ?1 LIMIT 1",
      )?;
      let test_details_opt: Option<(String, Option<i64>, Option<i64>)> = stmt
        .query_row([&test_qname], |row| {
          Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .optional()?;

      if let Some((name, start_line, end_line)) = test_details_opt {
        target_tests.push(rkg_core::TestCase {
          name,
          location: rkg_core::Location {
            file_path: test_file_path,
            start_line: start_line.unwrap_or(1) as usize,
            end_line: end_line.unwrap_or(1) as usize,
            start_column: None,
            end_column: None,
          },
          target_symbols: vec![sym.qualified_name.clone()],
        });
      }
    }
  }

  let edge_kinds = &["Calls", "Imports", "ReferencesType", "ModifiedWith"];

  let downstream = traverse_relationships(
    connection,
    repository_id,
    &target_ids,
    edge_kinds,
    TraversalDirection::Forward,
    1,
  )?;

  let upstream = traverse_relationships(
    connection,
    repository_id,
    &target_ids,
    edge_kinds,
    TraversalDirection::Backward,
    1,
  )?;

  let mut neighbor_nodes = Vec::new();
  let mut neighbor_edges = Vec::new();

  for node in downstream.nodes.iter().chain(upstream.nodes.iter()) {
    if !target_ids.contains(&node.symbol.id)
      && !neighbor_nodes
        .iter()
        .any(|n: &TraversalNode| n.symbol.id == node.symbol.id)
    {
      neighbor_nodes.push(node.clone());
    }
  }

  for edge in downstream.edges.iter().chain(upstream.edges.iter()) {
    if !neighbor_edges.iter().any(|e: &TraversalEdge| {
      e.source_symbol_id == edge.source_symbol_id
        && e.target_symbol_id == edge.target_symbol_id
        && e.kind == edge.kind
    }) {
      neighbor_edges.push(edge.clone());
    }
  }

  let mut edges_core = Vec::new();
  for edge in neighbor_edges {
    let mut stmt = connection.prepare("SELECT qualified_name FROM symbols WHERE id = ?1")?;
    let source_qname_opt: Option<String> = stmt
      .query_row([edge.source_symbol_id], |row| row.get(0))
      .optional()?;
    let target_qname_opt: Option<String> = stmt
      .query_row([edge.target_symbol_id], |row| row.get(0))
      .optional()?;

    if let (Some(source), Some(target)) = (source_qname_opt, target_qname_opt) {
      edges_core.push(rkg_core::Edge {
        source,
        target,
        kind: map_edge_kind(&edge.kind),
      });
    }
  }

  let mut related_symbols = Vec::new();
  let mut related_files = Vec::new();
  let mut related_docs = Vec::new();
  let mut related_tests = Vec::new();

  for node in neighbor_nodes {
    let mut stmt = connection.prepare("SELECT id, repository_id, path, language, content_hash, line_count, last_index_run_id FROM files WHERE path = ?1 LIMIT 1")?;
    let file_rec_opt: Option<rkg_db::FileRecord> = stmt
      .query_row([&node.file_path], |row| {
        Ok(rkg_db::FileRecord {
          id: row.get(0)?,
          repository_id: row.get(1)?,
          path: row.get(2)?,
          language: row.get(3)?,
          content_hash: row.get(4)?,
          line_count: row.get(5)?,
          last_index_run_id: row.get(6)?,
        })
      })
      .optional()?;

    if let Some(file_rec) = file_rec_opt {
      let file_core = rkg_core::File {
        path: file_rec.path.clone(),
        language: file_rec.language.clone(),
        hash: file_rec.content_hash.clone(),
        line_count: file_rec.line_count.map(|c| c as usize),
      };
      related_files.push(file_core);
    }

    related_symbols.push(rkg_core::Symbol {
      name: node.symbol.name.clone(),
      qualified_name: node.symbol.qualified_name.clone(),
      kind: map_symbol_kind(&node.symbol.kind),
      location: rkg_core::Location {
        file_path: node.file_path.clone(),
        start_line: node.symbol.start_line as usize,
        end_line: node.symbol.end_line as usize,
        start_column: node.symbol.start_column.map(|c| c as usize),
        end_column: node.symbol.end_column.map(|c| c as usize),
      },
    });

    let doc_records =
      rkg_db::lookup_docs_for_symbol(connection, repository_id, &node.symbol.qualified_name)?;
    for doc in doc_records {
      let mut stmt = connection.prepare("SELECT path FROM files WHERE id = ?1")?;
      let file_path: String = stmt.query_row([doc.file_id], |row| row.get(0))?;
      related_docs.push(rkg_core::DocBlock {
        title: doc.title.clone(),
        text: doc.body.clone(),
        location: rkg_core::Location {
          file_path,
          start_line: doc.start_line.unwrap_or(1) as usize,
          end_line: doc.end_line.unwrap_or(1) as usize,
          start_column: None,
          end_column: None,
        },
      });
    }

    let test_records =
      rkg_db::lookup_tests_for_symbol(connection, repository_id, &node.symbol.qualified_name)?;
    for (test_qname, test_file_path, _conf) in test_records {
      let mut stmt = connection.prepare(
        "SELECT name, start_line, end_line FROM tests WHERE qualified_name = ?1 LIMIT 1",
      )?;
      let test_details_opt: Option<(String, Option<i64>, Option<i64>)> = stmt
        .query_row([&test_qname], |row| {
          Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .optional()?;

      if let Some((name, start_line, end_line)) = test_details_opt {
        related_tests.push(rkg_core::TestCase {
          name,
          location: rkg_core::Location {
            file_path: test_file_path,
            start_line: start_line.unwrap_or(1) as usize,
            end_line: end_line.unwrap_or(1) as usize,
            start_column: None,
            end_column: None,
          },
          target_symbols: vec![node.symbol.qualified_name.clone()],
        });
      }
    }
  }

  fn sort_pack(pack: &mut rkg_core::ContextPack) {
    pack.files.sort_by(|a, b| a.path.cmp(&b.path));
    pack
      .symbols
      .sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));
    pack.edges.sort_by(|a, b| {
      a.source
        .cmp(&b.source)
        .then(a.target.cmp(&b.target))
        .then(format!("{:?}", a.kind).cmp(&format!("{:?}", b.kind)))
    });
    pack.docs.sort_by(|a, b| {
      a.location
        .file_path
        .cmp(&b.location.file_path)
        .then(a.location.start_line.cmp(&b.location.start_line))
        .then(a.text.cmp(&b.text))
    });
    pack.tests.sort_by(|a, b| {
      a.name
        .cmp(&b.name)
        .then(a.location.file_path.cmp(&b.location.file_path))
    });
  }

  let mut final_pack = rkg_core::ContextPack {
    target: Some(symbol_name.to_string()),
    files: Vec::new(),
    symbols: Vec::new(),
    edges: Vec::new(),
    docs: Vec::new(),
    tests: Vec::new(),
    token_budget,
  };

  if let Some(budget) = token_budget {
    final_pack.symbols = target_core_symbols.clone();
    final_pack.files = target_files.clone();
    sort_pack(&mut final_pack);

    let formatted = if format == "json" {
      format_context_pack_json(&final_pack, repo_root)
    } else {
      format_context_pack_markdown(&final_pack, repo_root)
    };

    if estimate_tokens(&formatted) > budget {
      return Ok(final_pack);
    }

    let mut sorted_target_docs = target_docs.clone();
    sorted_target_docs.sort_by(|a, b| {
      a.location
        .file_path
        .cmp(&b.location.file_path)
        .then(a.location.start_line.cmp(&b.location.start_line))
    });
    for doc in sorted_target_docs {
      let mut temp = final_pack.clone();
      temp.docs.push(doc);
      sort_pack(&mut temp);
      let formatted = if format == "json" {
        format_context_pack_json(&temp, repo_root)
      } else {
        format_context_pack_markdown(&temp, repo_root)
      };
      if estimate_tokens(&formatted) <= budget {
        final_pack = temp;
      } else {
        break;
      }
    }

    let mut sorted_target_tests = target_tests.clone();
    sorted_target_tests.sort_by(|a, b| a.name.cmp(&b.name));
    for test in sorted_target_tests {
      let mut temp = final_pack.clone();
      temp.tests.push(test.clone());
      let file_core = rkg_core::File {
        path: test.location.file_path.clone(),
        language: Some("python".to_string()),
        hash: None,
        line_count: None,
      };
      if !temp.files.contains(&file_core) {
        temp.files.push(file_core);
      }
      sort_pack(&mut temp);
      let formatted = if format == "json" {
        format_context_pack_json(&temp, repo_root)
      } else {
        format_context_pack_markdown(&temp, repo_root)
      };
      if estimate_tokens(&formatted) <= budget {
        final_pack = temp;
      } else {
        break;
      }
    }

    let mut temp = final_pack.clone();
    temp.edges = edges_core.clone();
    sort_pack(&mut temp);
    let formatted = if format == "json" {
      format_context_pack_json(&temp, repo_root)
    } else {
      format_context_pack_markdown(&temp, repo_root)
    };
    if estimate_tokens(&formatted) <= budget {
      final_pack = temp;
    }

    let mut sorted_related_symbols = related_symbols;
    sorted_related_symbols.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));

    for rel_sym in sorted_related_symbols {
      let mut temp = final_pack.clone();
      temp.symbols.push(rel_sym.clone());
      let file_core = rkg_core::File {
        path: rel_sym.location.file_path.clone(),
        language: Some("python".to_string()),
        hash: None,
        line_count: None,
      };
      if !temp.files.contains(&file_core) {
        temp.files.push(file_core);
      }
      sort_pack(&mut temp);
      let formatted = if format == "json" {
        format_context_pack_json(&temp, repo_root)
      } else {
        format_context_pack_markdown(&temp, repo_root)
      };
      if estimate_tokens(&formatted) <= budget {
        final_pack = temp;
      } else {
        break;
      }
    }

    let mut sorted_related_docs = related_docs;
    sorted_related_docs.sort_by(|a, b| a.location.file_path.cmp(&b.location.file_path));
    for rel_doc in sorted_related_docs {
      let mut temp = final_pack.clone();
      temp.docs.push(rel_doc.clone());
      sort_pack(&mut temp);
      let formatted = if format == "json" {
        format_context_pack_json(&temp, repo_root)
      } else {
        format_context_pack_markdown(&temp, repo_root)
      };
      if estimate_tokens(&formatted) <= budget {
        final_pack = temp;
      } else {
        break;
      }
    }

    let mut sorted_related_tests = related_tests;
    sorted_related_tests.sort_by(|a, b| a.name.cmp(&b.name));
    for rel_test in sorted_related_tests {
      let mut temp = final_pack.clone();
      temp.tests.push(rel_test.clone());
      let file_core = rkg_core::File {
        path: rel_test.location.file_path.clone(),
        language: Some("python".to_string()),
        hash: None,
        line_count: None,
      };
      if !temp.files.contains(&file_core) {
        temp.files.push(file_core);
      }
      sort_pack(&mut temp);
      let formatted = if format == "json" {
        format_context_pack_json(&temp, repo_root)
      } else {
        format_context_pack_markdown(&temp, repo_root)
      };
      if estimate_tokens(&formatted) <= budget {
        final_pack = temp;
      } else {
        break;
      }
    }
  } else {
    final_pack.symbols = target_core_symbols;
    final_pack.files = target_files;

    final_pack.docs.extend(target_docs);
    for test in &target_tests {
      let file_core = rkg_core::File {
        path: test.location.file_path.clone(),
        language: Some("python".to_string()),
        hash: None,
        line_count: None,
      };
      if !final_pack.files.contains(&file_core) {
        final_pack.files.push(file_core);
      }
    }
    final_pack.tests.extend(target_tests);

    final_pack.edges.extend(edges_core);

    for rel_sym in related_symbols {
      if !final_pack.symbols.contains(&rel_sym) {
        final_pack.symbols.push(rel_sym);
      }
    }
    for file_core in related_files {
      if !final_pack.files.contains(&file_core) {
        final_pack.files.push(file_core);
      }
    }
    for rel_doc in related_docs {
      if !final_pack.docs.contains(&rel_doc) {
        final_pack.docs.push(rel_doc);
      }
    }
    for rel_test in &related_tests {
      let file_core = rkg_core::File {
        path: rel_test.location.file_path.clone(),
        language: Some("python".to_string()),
        hash: None,
        line_count: None,
      };
      if !final_pack.files.contains(&file_core) {
        final_pack.files.push(file_core);
      }
    }
    final_pack.tests.extend(related_tests);

    sort_pack(&mut final_pack);
  }

  let active_symbol_qnames: HashSet<String> = final_pack
    .symbols
    .iter()
    .map(|s| s.qualified_name.clone())
    .collect();
  final_pack.edges.retain(|e| {
    active_symbol_qnames.contains(&e.source) && active_symbol_qnames.contains(&e.target)
  });

  Ok(final_pack)
}

#[cfg(test)]
mod tests {
  use super::*;
  use rkg_db::{
    NewEdgeRecord, NewFileRecord, NewSymbolRecord, initialize_schema, insert_edge, insert_file,
    insert_symbol,
  };

  fn seed_test_db() -> (Connection, i64) {
    let connection = Connection::open_in_memory().expect("in-memory db opens");
    initialize_schema(&connection).expect("schema initializes");

    connection
      .execute(
        "INSERT INTO repositories (root_path, vcs_type) VALUES (?1, ?2)",
        ("/tmp/repo", "git"),
      )
      .expect("repository insert");
    let repo_id = connection.last_insert_rowid();
    (connection, repo_id)
  }

  #[test]
  fn test_resolve_start_symbols() {
    let (connection, repo_id) = seed_test_db();
    let file = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: repo_id,
        path: "src/main.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash1".to_string()),
        line_count: Some(10),
        last_index_run_id: None,
      },
    )
    .unwrap();

    insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file.id,
        name: "foo".to_string(),
        qualified_name: "src.main::foo".to_string(),
        kind: "Function".to_string(),
        start_line: 1,
        end_line: 5,
        start_column: None,
        end_column: None,
      },
    )
    .unwrap();

    let resolved = resolve_start_symbols(&connection, repo_id, "src.main::foo").unwrap();
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].name, "foo");

    let resolved_fuzzy = resolve_start_symbols(&connection, repo_id, "foo").unwrap();
    assert_eq!(resolved_fuzzy.len(), 1);
    assert_eq!(resolved_fuzzy[0].qualified_name, "src.main::foo");
  }

  #[test]
  fn test_traverse_relationships_and_impact() {
    let (connection, repo_id) = seed_test_db();
    let file = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: repo_id,
        path: "src/main.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash1".to_string()),
        line_count: Some(10),
        last_index_run_id: None,
      },
    )
    .unwrap();

    let s1 = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file.id,
        name: "a".to_string(),
        qualified_name: "src.main::a".to_string(),
        kind: "Function".to_string(),
        start_line: 1,
        end_line: 2,
        start_column: None,
        end_column: None,
      },
    )
    .unwrap();

    let s2 = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file.id,
        name: "b".to_string(),
        qualified_name: "src.main::b".to_string(),
        kind: "Function".to_string(),
        start_line: 3,
        end_line: 4,
        start_column: None,
        end_column: None,
      },
    )
    .unwrap();

    let s3 = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file.id,
        name: "c".to_string(),
        qualified_name: "src.main::c".to_string(),
        kind: "Function".to_string(),
        start_line: 5,
        end_line: 6,
        start_column: None,
        end_column: None,
      },
    )
    .unwrap();

    // s1 calls s2 calls s3
    insert_edge(
      &connection,
      &NewEdgeRecord {
        source_symbol_id: s1.id,
        target_symbol_id: Some(s2.id),
        unresolved_target: None,
        kind: "Calls".to_string(),
        confidence: Some(1.0),
      },
    )
    .unwrap();

    insert_edge(
      &connection,
      &NewEdgeRecord {
        source_symbol_id: s2.id,
        target_symbol_id: Some(s3.id),
        unresolved_target: None,
        kind: "Calls".to_string(),
        confidence: Some(1.0),
      },
    )
    .unwrap();

    // Traverse forward from s1
    let forward = traverse_relationships(
      &connection,
      repo_id,
      &[s1.id],
      &["Calls"],
      TraversalDirection::Forward,
      2,
    )
    .unwrap();

    assert_eq!(forward.nodes.len(), 3); // s1 (depth 0), s2 (depth 1), s3 (depth 2)
    assert_eq!(forward.edges.len(), 2);

    // Analyze impact of s2
    let impact = analyze_impact(&connection, repo_id, "src.main::b", 2).unwrap();
    assert_eq!(impact.target_symbols.len(), 1);
    assert_eq!(impact.target_symbols[0].name, "b");

    // b calls c (downstream)
    assert!(impact.downstream_nodes.iter().any(|n| n.symbol.name == "c"));
    // a calls b (upstream)
    assert!(impact.upstream_nodes.iter().any(|n| n.symbol.name == "a"));
  }

  #[test]
  fn test_estimate_tokens() {
    assert_eq!(estimate_tokens(""), 0);
    assert_eq!(estimate_tokens("abcd"), 1);
    assert_eq!(estimate_tokens("abcdefgh"), 2);
    assert_eq!(estimate_tokens("abcdefghi"), 3);
  }

  #[test]
  fn test_pack_context_and_formatting() {
    let (connection, repo_id) = seed_test_db();
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let repo_root = temp_dir.path();

    // Create dummy files
    let main_py = repo_root.join("src/main.py");
    std::fs::create_dir_all(main_py.parent().unwrap()).unwrap();
    std::fs::write(&main_py, "def foo():\n    pass\n\ndef bar():\n    foo()\n").unwrap();

    let file = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: repo_id,
        path: "src/main.py".to_string(),
        language: Some("python".to_string()),
        content_hash: Some("hash1".to_string()),
        line_count: Some(5),
        last_index_run_id: None,
      },
    )
    .unwrap();

    let s_foo = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file.id,
        name: "foo".to_string(),
        qualified_name: "src.main::foo".to_string(),
        kind: "Function".to_string(),
        start_line: 1,
        end_line: 2,
        start_column: None,
        end_column: None,
      },
    )
    .unwrap();

    let s_bar = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file.id,
        name: "bar".to_string(),
        qualified_name: "src.main::bar".to_string(),
        kind: "Function".to_string(),
        start_line: 4,
        end_line: 5,
        start_column: None,
        end_column: None,
      },
    )
    .unwrap();

    // bar calls foo
    insert_edge(
      &connection,
      &NewEdgeRecord {
        source_symbol_id: s_bar.id,
        target_symbol_id: Some(s_foo.id),
        unresolved_target: None,
        kind: "Calls".to_string(),
        confidence: Some(1.0),
      },
    )
    .unwrap();

    // Insert doc for foo
    rkg_db::insert_doc(
      &connection,
      &rkg_db::NewDocRecord {
        file_id: file.id,
        symbol_id: Some(s_foo.id),
        title: Some("foo doc".to_string()),
        body: "documentation of foo function".to_string(),
        start_line: Some(1),
        end_line: Some(2),
        source_kind: "Docstring".to_string(),
      },
    )
    .unwrap();

    // Insert test for foo
    rkg_db::insert_test(
      &connection,
      &rkg_db::NewTestRecord {
        file_id: file.id,
        name: "test_foo".to_string(),
        qualified_name: "tests.test_main::test_foo".to_string(),
        kind: "Function".to_string(),
        is_parametrized: false,
        framework: "pytest".to_string(),
        start_line: Some(1),
        end_line: Some(3),
      },
    )
    .unwrap();

    let s_test_foo = insert_symbol(
      &connection,
      &NewSymbolRecord {
        file_id: file.id,
        name: "test_foo".to_string(),
        qualified_name: "tests.test_main::test_foo".to_string(),
        kind: "Function".to_string(),
        start_line: 1,
        end_line: 3,
        start_column: None,
        end_column: None,
      },
    )
    .unwrap();

    // Link test to foo
    insert_edge(
      &connection,
      &NewEdgeRecord {
        source_symbol_id: s_test_foo.id,
        target_symbol_id: Some(s_foo.id),
        unresolved_target: None,
        kind: "TestedBy".to_string(),
        confidence: Some(1.0),
      },
    )
    .unwrap();

    // 1. Test unbounded context packing
    let pack = pack_context(&connection, repo_id, repo_root, "foo", None, "markdown").unwrap();
    assert_eq!(pack.target, Some("foo".to_string()));
    assert_eq!(pack.symbols.len(), 2); // foo itself, plus test_foo (tested_by relationship) and bar (caller) are neighbors but wait: we do depth-1 neighbors. Let's see: bar calls foo (so bar is upstream neighbor). So symbols = foo, bar, test_foo (wait, test_foo is connected via TestedBy edge to foo. Wait, TestedBy is not in traverse_relationships edge_kinds unless included. Edge kinds in traverse_relationships is Calls, Imports, ReferencesType, ModifiedWith. So test_foo is not a neighbor node via traverse_relationships, but it is added in target_tests!).
    // Wait, let's verify symbols in pack:
    // target_core_symbols: foo.
    // traverse_relationships forward/backward depth 1 from foo:
    // forward Calls/Imports/TypeRefs/Decorators from foo: none.
    // backward Calls/Imports/TypeRefs/Decorators to foo: bar calls foo. So bar is upstream.
    // So related_symbols has bar.
    // So final symbols has foo and bar.
    assert!(pack.symbols.iter().any(|s| s.name == "foo"));
    assert!(pack.symbols.iter().any(|s| s.name == "bar"));
    assert_eq!(pack.tests.len(), 1);
    assert_eq!(pack.tests[0].name, "test_foo");

    // 2. Test Markdown formatting
    let md = format_context_pack_markdown(&pack, repo_root);
    assert!(md.contains("# rkg Context Pack: foo"));
    assert!(md.contains("## Symbol Definitions"));
    assert!(md.contains("Symbol: `src.main::foo`"));
    assert!(md.contains("def foo():"));
    assert!(md.contains("## Related Test Cases"));
    assert!(md.contains("Test: `test_foo`"));

    // 3. Test JSON formatting
    let json = format_context_pack_json(&pack, repo_root);
    assert!(json.contains("\"target\": \"foo\""));
    assert!(json.contains("\"name\": \"foo\""));
    assert!(json.contains("\"name\": \"bar\""));
    assert!(json.contains("\"name\": \"test_foo\""));
    assert!(json.contains("def foo():"));

    // 4. Test budget pruning
    // Let's set a small budget that only allows target symbol + docs
    let budget_pack = pack_context(
      &connection,
      repo_id,
      repo_root,
      "foo",
      Some(100),
      "markdown",
    )
    .unwrap();
    // In this case, it should successfully prune the rest of related symbols/docs/tests to fit in budget!
    assert!(budget_pack.symbols.iter().any(|s| s.name == "foo"));
  }

  #[test]
  fn test_query_concurrency_topology() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    rkg_db::initialize_schema(&connection).unwrap();

    let repo_id = rkg_db::upsert_repository(&connection, "/repo").unwrap().id;
    let file = rkg_db::reindex_file(
      &connection,
      &rkg_db::NewFileRecord {
        repository_id: repo_id,
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        content_hash: Some("abc".to_string()),
        line_count: Some(10),
        last_index_run_id: None,
      },
    )
    .unwrap();

    let symbol = rkg_db::insert_symbol(
      &connection,
      &rkg_db::NewSymbolRecord {
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

    rkg_db::insert_concurrency_spawn(
      &connection,
      &rkg_db::NewConcurrencySpawnRecord {
        file_id: file.id,
        source_symbol_qualified_name: symbol.qualified_name.clone(),
        spawn_kind: "tokio::spawn".to_string(),
        target_name: Some("worker".to_string()),
        start_line: 4,
        end_line: 6,
      },
    )
    .unwrap();

    rkg_db::insert_concurrency_channel(
      &connection,
      &rkg_db::NewConcurrencyChannelRecord {
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

    rkg_db::insert_concurrency_select(
      &connection,
      &rkg_db::NewConcurrencySelectRecord {
        file_id: file.id,
        source_symbol_qualified_name: symbol.qualified_name.clone(),
        start_line: 8,
        end_line: 9,
      },
    )
    .unwrap();

    let report = get_concurrency_topology(&connection, repo_id, "main").unwrap();

    assert_eq!(report.target_symbol, "src::lib::main");
    assert_eq!(report.spawns.len(), 1);
    assert_eq!(report.spawns[0].target_name, Some("worker".to_string()));
    assert_eq!(report.channels.len(), 1);
    assert_eq!(report.channels[0].tx_name, "tx");
    assert_eq!(report.selects.len(), 1);
  }

  #[test]
  fn test_query_safety_profile() {
    let connection = Connection::open_in_memory().unwrap();
    rkg_db::initialize_schema(&connection).unwrap();

    let repo_id = rkg_db::upsert_repository(&connection, "/repo").unwrap().id;
    let file = rkg_db::reindex_file(
      &connection,
      &rkg_db::NewFileRecord {
        repository_id: repo_id,
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        content_hash: Some("abc".to_string()),
        line_count: Some(10),
        last_index_run_id: None,
      },
    )
    .unwrap();

    let symbol = rkg_db::insert_symbol(
      &connection,
      &rkg_db::NewSymbolRecord {
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

    // 1. One safely wrapped unsafe block
    rkg_db::insert_rust_unsafe_block(
      &connection,
      &rkg_db::NewRustUnsafeBlockRecord {
        file_id: file.id,
        source_symbol_qualified_name: symbol.qualified_name.clone(),
        start_line: 4,
        end_line: 6,
      },
    )
    .unwrap();

    // 2. One FFI binding
    rkg_db::insert_rust_ffi_binding(
      &connection,
      &rkg_db::NewRustFFIBindingRecord {
        file_id: file.id,
        source_symbol_qualified_name: symbol.qualified_name.clone(),
        foreign_item_name: "c_func".to_string(),
        abi: "C".to_string(),
        start_line: 2,
        end_line: 3,
      },
    )
    .unwrap();

    let profile = get_safety_profile(&connection, repo_id, "main").unwrap();

    assert_eq!(profile.target_name, "src::lib::main");
    assert_eq!(profile.unsafe_blocks.len(), 1);
    assert_eq!(profile.ffi_bindings.len(), 1);
    assert_eq!(profile.safe_wrapper_percentage, 100.0);

    // Score calculation: 100 - 15 (FFI) - 2 (wrapped block) = 83
    assert_eq!(profile.safety_score, 83);
    assert_eq!(profile.risk_level, "Medium");

    // Query via file path
    let file_profile = get_safety_profile(&connection, repo_id, "src/lib.rs").unwrap();
    assert_eq!(file_profile.target_name, "src/lib.rs");
    assert_eq!(file_profile.unsafe_blocks.len(), 1);
    assert_eq!(file_profile.ffi_bindings.len(), 1);
  }

  #[test]
  fn test_query_coverage_profile() {
    let (connection, repo_id) = seed_test_db();

    let file = insert_file(
      &connection,
      &NewFileRecord {
        repository_id: repo_id,
        path: "src/math.rs".to_string(),
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
        name: "add".to_string(),
        qualified_name: "src/math.rs::add".to_string(),
        kind: "function".to_string(),
        start_line: 1,
        end_line: 5,
        start_column: None,
        end_column: None,
      },
    )
    .unwrap();

    let _ = rkg_db::insert_symbol_coverage(
      &connection,
      &rkg_db::NewSymbolCoverageRecord {
        file_id: file.id,
        symbol_id: symbol.id,
        report_path: "cov.xml".to_string(),
        test_suite: Some("unit".to_string()),
        lines_valid: 5,
        lines_covered: 4,
        branches_valid: 2,
        branches_covered: 1,
        coverable_lines: "1,2,3,4,5".to_string(),
        uncovered_lines: "5".to_string(),
      },
    )
    .unwrap();

    // Query coverage profile by symbol:
    let profile = get_coverage_profile(&connection, repo_id, "src/math.rs::add").unwrap();
    assert_eq!(profile.target_name, "src/math.rs::add");
    assert_eq!(profile.lines_valid, 5);
    assert_eq!(profile.lines_covered, 4);
    assert_eq!(profile.uncovered_lines, vec![5]);
    assert_eq!(profile.test_suites.len(), 1);
    assert_eq!(profile.test_suites[0].test_suite.as_deref(), Some("unit"));

    // Query coverage profile by file:
    let file_profile = get_coverage_profile(&connection, repo_id, "src/math.rs").unwrap();
    assert_eq!(file_profile.target_name, "src/math.rs");
    assert_eq!(file_profile.lines_valid, 5);
    assert_eq!(file_profile.lines_covered, 4);
  }
}

#[derive(Debug, Clone)]
pub struct ConcurrencyTopologyReport {
  pub target_symbol: String,
  pub spawns: Vec<rkg_core::ConcurrencySpawn>,
  pub channels: Vec<rkg_core::ConcurrencyChannel>,
  pub selects: Vec<rkg_core::ConcurrencySelect>,
}

pub fn get_concurrency_topology(
  connection: &rusqlite::Connection,
  repository_id: i64,
  symbol_name: &str,
) -> Result<ConcurrencyTopologyReport, String> {
  let start_symbols =
    resolve_start_symbols(connection, repository_id, symbol_name).map_err(|e| e.to_string())?;
  if start_symbols.is_empty() {
    return Err(format!("Symbol not found: {symbol_name}"));
  }
  let symbol = &start_symbols[0];

  let db_spawns =
    rkg_db::lookup_concurrency_spawns_for_symbol(connection, repository_id, &symbol.qualified_name)
      .map_err(|e| e.to_string())?;

  let db_channels = rkg_db::lookup_concurrency_channels_for_symbol(
    connection,
    repository_id,
    &symbol.qualified_name,
  )
  .map_err(|e| e.to_string())?;

  let db_selects = rkg_db::lookup_concurrency_selects_for_symbol(
    connection,
    repository_id,
    &symbol.qualified_name,
  )
  .map_err(|e| e.to_string())?;

  let mut spawns = Vec::new();
  for s in db_spawns {
    let file = rkg_db::list_files_for_repository(connection, repository_id)
      .map_err(|e| e.to_string())?
      .into_iter()
      .find(|f| f.id == s.file_id)
      .ok_or_else(|| "file not found".to_string())?;

    spawns.push(rkg_core::ConcurrencySpawn {
      source_symbol_qualified_name: s.source_symbol_qualified_name,
      spawn_kind: s.spawn_kind,
      target_name: s.target_name,
      location: rkg_core::Location {
        file_path: file.path,
        start_line: s.start_line as usize,
        end_line: s.end_line as usize,
        start_column: Some(0),
        end_column: Some(0),
      },
    });
  }

  let mut channels = Vec::new();
  for c in db_channels {
    let file = rkg_db::list_files_for_repository(connection, repository_id)
      .map_err(|e| e.to_string())?
      .into_iter()
      .find(|f| f.id == c.file_id)
      .ok_or_else(|| "file not found".to_string())?;

    channels.push(rkg_core::ConcurrencyChannel {
      source_symbol_qualified_name: c.source_symbol_qualified_name,
      channel_kind: c.channel_kind,
      tx_name: c.tx_name,
      rx_name: c.rx_name,
      location: rkg_core::Location {
        file_path: file.path,
        start_line: c.start_line as usize,
        end_line: c.end_line as usize,
        start_column: Some(0),
        end_column: Some(0),
      },
    });
  }

  let mut selects = Vec::new();
  for s in db_selects {
    let file = rkg_db::list_files_for_repository(connection, repository_id)
      .map_err(|e| e.to_string())?
      .into_iter()
      .find(|f| f.id == s.file_id)
      .ok_or_else(|| "file not found".to_string())?;

    selects.push(rkg_core::ConcurrencySelect {
      source_symbol_qualified_name: s.source_symbol_qualified_name,
      location: rkg_core::Location {
        file_path: file.path,
        start_line: s.start_line as usize,
        end_line: s.end_line as usize,
        start_column: Some(0),
        end_column: Some(0),
      },
    });
  }

  Ok(ConcurrencyTopologyReport {
    target_symbol: symbol.qualified_name.clone(),
    spawns,
    channels,
    selects,
  })
}

pub fn get_safety_profile(
  connection: &rusqlite::Connection,
  repository_id: i64,
  target: &str,
) -> Result<rkg_core::SafetyProfile, String> {
  let is_file = target.ends_with(".rs") || target.contains('/') || target.contains('\\');

  let (db_blocks, db_funcs, db_ffis, target_display_name) = if is_file {
    let files =
      rkg_db::list_files_for_repository(connection, repository_id).map_err(|e| e.to_string())?;
    let matched_file = files
      .iter()
      .find(|f| f.path == target)
      .ok_or_else(|| format!("File not found in repository: {target}"))?;

    let blocks =
      rkg_db::lookup_rust_unsafe_blocks_for_file(connection, repository_id, &matched_file.path)
        .map_err(|e| e.to_string())?;
    let funcs =
      rkg_db::lookup_rust_unsafe_functions_for_file(connection, repository_id, &matched_file.path)
        .map_err(|e| e.to_string())?;
    let ffis =
      rkg_db::lookup_rust_ffi_bindings_for_file(connection, repository_id, &matched_file.path)
        .map_err(|e| e.to_string())?;

    (blocks, funcs, ffis, target.to_string())
  } else {
    let start_symbols =
      resolve_start_symbols(connection, repository_id, target).map_err(|e| e.to_string())?;
    if start_symbols.is_empty() {
      return Err(format!("Symbol not found: {target}"));
    }
    let symbol = &start_symbols[0];

    let blocks = rkg_db::lookup_rust_unsafe_blocks_for_symbol(
      connection,
      repository_id,
      &symbol.qualified_name,
    )
    .map_err(|e| e.to_string())?;
    let funcs = rkg_db::lookup_rust_unsafe_functions_for_symbol(
      connection,
      repository_id,
      &symbol.qualified_name,
    )
    .map_err(|e| e.to_string())?;
    let ffis = rkg_db::lookup_rust_ffi_bindings_for_symbol(
      connection,
      repository_id,
      &symbol.qualified_name,
    )
    .map_err(|e| e.to_string())?;

    (blocks, funcs, ffis, symbol.qualified_name.clone())
  };

  // Get file paths mapping
  let files =
    rkg_db::list_files_for_repository(connection, repository_id).map_err(|e| e.to_string())?;

  let mut unsafe_blocks = Vec::new();
  for b in &db_blocks {
    let file = files
      .iter()
      .find(|f| f.id == b.file_id)
      .ok_or_else(|| "file not found".to_string())?;
    unsafe_blocks.push(rkg_core::RustUnsafeBlock {
      source_symbol_qualified_name: b.source_symbol_qualified_name.clone(),
      location: rkg_core::Location {
        file_path: file.path.clone(),
        start_line: b.start_line as usize,
        end_line: b.end_line as usize,
        start_column: Some(0),
        end_column: Some(0),
      },
    });
  }

  let mut unsafe_functions = Vec::new();
  for f in &db_funcs {
    let file = files
      .iter()
      .find(|fl| fl.id == f.file_id)
      .ok_or_else(|| "file not found".to_string())?;
    unsafe_functions.push(rkg_core::RustUnsafeFunction {
      qualified_name: f.qualified_name.clone(),
      location: rkg_core::Location {
        file_path: file.path.clone(),
        start_line: f.start_line as usize,
        end_line: f.end_line as usize,
        start_column: Some(0),
        end_column: Some(0),
      },
    });
  }

  let mut ffi_bindings = Vec::new();
  for f in &db_ffis {
    let file = files
      .iter()
      .find(|fl| fl.id == f.file_id)
      .ok_or_else(|| "file not found".to_string())?;
    ffi_bindings.push(rkg_core::RustFFIBinding {
      source_symbol_qualified_name: f.source_symbol_qualified_name.clone(),
      foreign_item_name: f.foreign_item_name.clone(),
      abi: f.abi.clone(),
      location: rkg_core::Location {
        file_path: file.path.clone(),
        start_line: f.start_line as usize,
        end_line: f.end_line as usize,
        start_column: Some(0),
        end_column: Some(0),
      },
    });
  }

  // Calculate safe wrapper percentage and exposed blocks count
  let mut wrapped_count = 0;
  let mut exposed_count = 0;
  for b in &db_blocks {
    let mut in_unsafe_func = false;
    for f in &db_funcs {
      if b.file_id == f.file_id && b.start_line >= f.start_line && b.end_line <= f.end_line {
        in_unsafe_func = true;
        break;
      }
    }
    if in_unsafe_func {
      exposed_count += 1;
    } else {
      wrapped_count += 1;
    }
  }

  let safe_wrapper_percentage = if db_blocks.is_empty() {
    100.0
  } else {
    (wrapped_count as f64 / db_blocks.len() as f64) * 100.0
  };

  // Calculate safety score (start at 100)
  let mut score: i32 = 100;
  // Deduct 15 points per FFI binding
  score -= (ffi_bindings.len() as i32) * 15;
  // Deduct 10 points per unsafe function boundary
  score -= (unsafe_functions.len() as i32) * 10;
  // Deduct 5 points per exposed unsafe block (inside unsafe function)
  score -= exposed_count * 5;
  // Deduct 2 points per safely wrapped unsafe block
  score -= wrapped_count * 2;

  let final_score = score.clamp(0, 100) as u32;

  let risk_level = if final_score >= 90 {
    "Low".to_string()
  } else if final_score >= 60 {
    "Medium".to_string()
  } else {
    "High".to_string()
  };

  Ok(rkg_core::SafetyProfile {
    target_name: target_display_name,
    unsafe_blocks,
    unsafe_functions,
    ffi_bindings,
    safety_score: final_score,
    risk_level,
    safe_wrapper_percentage,
  })
}

pub fn get_coverage_profile(
  connection: &rusqlite::Connection,
  repository_id: i64,
  target: &str,
) -> Result<rkg_core::CoverageProfile, String> {
  let is_file = (target.ends_with(".py")
    || target.ends_with(".rs")
    || target.ends_with(".fs")
    || target.ends_with(".fsx")
    || target.ends_with(".fsi")
    || target.ends_with(".mojo")
    || target.ends_with(".🔥")
    || target.ends_with(".kt")
    || target.ends_with(".kts")
    || target.ends_with(".swift")
    || target.ends_with(".ipynb")
    || target.contains('/')
    || target.contains('\\'))
    && !target.contains("::");

  let parse_lines = |s: &str| -> std::collections::HashSet<usize> {
    s.split(',')
      .filter_map(|part| part.trim().parse::<usize>().ok())
      .collect()
  };

  if is_file {
    let files =
      rkg_db::list_files_for_repository(connection, repository_id).map_err(|e| e.to_string())?;
    let matched_file = files
      .iter()
      .find(|f| f.path == target)
      .ok_or_else(|| format!("File not found in repository: {target}"))?;

    let db_records = rkg_db::list_symbol_coverage_for_file(connection, matched_file.id)
      .map_err(|e| e.to_string())?;

    if db_records.is_empty() {
      return Err(format!("No coverage data found for file: {target}"));
    }

    let mut test_suites = Vec::new();
    let mut combined_coverable = std::collections::HashSet::new();
    let mut combined_uncovered: Option<std::collections::HashSet<usize>> = None;
    let mut combined_branches_valid = 0;
    let mut combined_branches_covered = 0;

    // Group records by (report_path, test_suite):
    let mut grouped: HashMap<(String, Option<String>), Vec<rkg_db::SymbolCoverageRecord>> =
      HashMap::new();
    for rec in db_records {
      let key = (rec.report_path.clone(), rec.test_suite.clone());
      grouped.entry(key).or_default().push(rec);
    }

    for ((report_path, test_suite), recs) in grouped {
      let mut suite_coverable = std::collections::HashSet::new();
      let mut suite_uncovered = std::collections::HashSet::new();
      let mut suite_branches_valid = 0;
      let mut suite_branches_covered = 0;

      for r in recs {
        suite_coverable.extend(parse_lines(&r.coverable_lines));
        suite_uncovered.extend(parse_lines(&r.uncovered_lines));
        suite_branches_valid += r.branches_valid as usize;
        suite_branches_covered += r.branches_covered as usize;
      }

      combined_coverable.extend(suite_coverable.clone());
      match combined_uncovered {
        None => combined_uncovered = Some(suite_uncovered.clone()),
        Some(ref mut current) => {
          // Intersection: a line is uncovered only if it's uncovered in all runs
          *current = current.intersection(&suite_uncovered).cloned().collect();
        }
      }

      // Branches: take the maximum coverage from any single report/test suite as combined branch coverage, or sum them.
      // Let's take the max of branches covered and valid to be safe.
      if suite_branches_covered > combined_branches_covered {
        combined_branches_covered = suite_branches_covered;
        combined_branches_valid = suite_branches_valid;
      } else if combined_branches_valid == 0 {
        combined_branches_valid = suite_branches_valid;
      }

      let lines_valid = suite_coverable.len();
      let lines_covered = lines_valid.saturating_sub(suite_uncovered.len());
      let mut uncovered_sorted: Vec<usize> = suite_uncovered.into_iter().collect();
      uncovered_sorted.sort();

      test_suites.push(rkg_core::TestSuiteCoverage {
        test_suite,
        report_path,
        lines_valid,
        lines_covered,
        branches_valid: suite_branches_valid,
        branches_covered: suite_branches_covered,
        uncovered_lines: uncovered_sorted,
      });
    }

    let final_uncovered = combined_uncovered.unwrap_or_default();
    let lines_valid = combined_coverable.len();
    let lines_covered = lines_valid.saturating_sub(final_uncovered.len());
    let mut combined_uncovered_sorted: Vec<usize> = final_uncovered.into_iter().collect();
    combined_uncovered_sorted.sort();

    test_suites.sort_by(|a, b| {
      a.report_path
        .cmp(&b.report_path)
        .then(a.test_suite.cmp(&b.test_suite))
    });

    Ok(rkg_core::CoverageProfile {
      target_name: target.to_string(),
      is_file: true,
      lines_valid,
      lines_covered,
      branches_valid: combined_branches_valid,
      branches_covered: combined_branches_covered,
      uncovered_lines: combined_uncovered_sorted,
      test_suites,
    })
  } else {
    // Target is a symbol
    let start_symbols =
      resolve_start_symbols(connection, repository_id, target).map_err(|e| e.to_string())?;
    if start_symbols.is_empty() {
      return Err(format!("Symbol not found: {target}"));
    }
    // We analyze the first matched symbol
    let symbol = &start_symbols[0];

    let db_records =
      rkg_db::list_symbol_coverage_for_symbol(connection, symbol.id).map_err(|e| e.to_string())?;

    if db_records.is_empty() {
      return Err(format!(
        "No coverage data found for symbol: {}",
        symbol.qualified_name
      ));
    }

    let mut test_suites = Vec::new();
    let mut combined_coverable = std::collections::HashSet::new();
    let mut combined_uncovered: Option<std::collections::HashSet<usize>> = None;
    let mut combined_branches_valid = 0;
    let mut combined_branches_covered = 0;

    for r in db_records {
      let suite_coverable = parse_lines(&r.coverable_lines);
      let suite_uncovered = parse_lines(&r.uncovered_lines);

      combined_coverable.extend(suite_coverable.clone());
      match combined_uncovered {
        None => combined_uncovered = Some(suite_uncovered.clone()),
        Some(ref mut current) => {
          *current = current.intersection(&suite_uncovered).cloned().collect();
        }
      }

      if r.branches_covered as usize > combined_branches_covered {
        combined_branches_covered = r.branches_covered as usize;
        combined_branches_valid = r.branches_valid as usize;
      } else if combined_branches_valid == 0 {
        combined_branches_valid = r.branches_valid as usize;
      }

      let mut uncovered_sorted: Vec<usize> = suite_uncovered.into_iter().collect();
      uncovered_sorted.sort();

      test_suites.push(rkg_core::TestSuiteCoverage {
        test_suite: r.test_suite,
        report_path: r.report_path,
        lines_valid: r.lines_valid as usize,
        lines_covered: r.lines_covered as usize,
        branches_valid: r.branches_valid as usize,
        branches_covered: r.branches_covered as usize,
        uncovered_lines: uncovered_sorted,
      });
    }

    let final_uncovered = combined_uncovered.unwrap_or_default();
    let lines_valid = combined_coverable.len();
    let lines_covered = lines_valid.saturating_sub(final_uncovered.len());
    let mut combined_uncovered_sorted: Vec<usize> = final_uncovered.into_iter().collect();
    combined_uncovered_sorted.sort();

    test_suites.sort_by(|a, b| {
      a.report_path
        .cmp(&b.report_path)
        .then(a.test_suite.cmp(&b.test_suite))
    });

    Ok(rkg_core::CoverageProfile {
      target_name: symbol.qualified_name.clone(),
      is_file: false,
      lines_valid,
      lines_covered,
      branches_valid: combined_branches_valid,
      branches_covered: combined_branches_covered,
      uncovered_lines: combined_uncovered_sorted,
      test_suites,
    })
  }
}
