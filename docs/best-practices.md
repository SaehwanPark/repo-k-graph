# Repository Best Practices and Design Guidelines

This document highlights the coding philosophies, patterns, and best practices applied during the Code Quality refactoring. It serves as a guide for future workspace development.

## 1. Don't Repeat Yourself (DRY) in Library Concerns

### Problem
Duplicating setup sequences for underlying resources (e.g. initializing Tree-sitter parsers, configuring bindings) creates bloated source files and increases maintainability overhead when dependencies or configurations change.

### Best Practice
Extract setup boilerplate into focused, private unified utilities that map external library outcomes into domain types.
- **Example**: In `rkg-lang-python`, we centralized 7 identical blocks of Tree-sitter Parser binding and language initialization into a single unified helper:
  ```rust
  fn parse_to_tree(source: &str) -> Result<tree_sitter::Tree, PythonParseError> {
    let mut parser = Parser::new();
    parser
      .set_language(&tree_sitter_python::LANGUAGE.into())
      .map_err(|e| PythonParseError::UnsupportedLanguage(e.to_string()))?;
    parser.parse(source, None).ok_or(PythonParseError::ParseCancelled)
  }
  ```

---

## 2. Maximize Idiomatic Rust Iterators

### Problem
Using manual `for item in iterator` loops to push results into a mutable vector is a common imperative pattern that clutters functions and obscures error propagation.
```rust
// Avoid
let mut results = Vec::new();
for row in rows {
  results.push(row?);
}
Ok(results)
```

### Best Practice
Leverage standard library collections and the `FromIterator` trait. Collecting an iterator of `Result<T, E>` into a `Result<Vec<T>, E>` simplifies error handling and represents idiomatic Rust at its best.
- **Example**: In `rkg-db` queries, we replaced 10 manual loop blocks with a single clean iterator collection:
  ```rust
  // Collect automatically propagates rusqlite::Error or returns the complete Vec
  rows.collect()
  ```
  And where type inference requires guidance:
  ```rust
  let unresolved: Vec<_> = rows.collect::<Result<Vec<_>, _>>()?;
  ```

---

## 3. Function Sizing and Single Responsibility (SRP)

### Problem
Monolithic functions containing hundreds of lines of code (such as the original 450-line `run_index`) are extremely hard to test, read, and maintain. They violate the Clean Code principle of doing one thing and doing it well.

### Best Practice
Decompose massive orchestration functions into clean, single-responsibility helper functions.
- Ensure the main orchestrator reads like a high-level table of contents.
- Keep helper functions private and tightly scoped around an individual sub-task (e.g., separating symbol extraction, call relationship extraction, type reference indexing, and pytest discovery into distinct functions).

---

## 4. Eliminate Nested Blocks with Guard Clauses

### Problem
Deeply nested `if` statements or `if let` blocks create cognitive load and make code harder to scan (the "arrow anti-pattern").
```rust
// Avoid
if condition {
  if let Ok(value) = action() {
     // nested logic
  }
}
```

### Best Practice
Prefer guard clauses (early returns) to handle negative conditions or non-matching states first.
- **Example**: In `index_file_tests`:
  ```rust
  if !is_pytest_file {
    return Ok(());
  }

  if let Ok(extracted_tests) = extract_tests_from_source(content, path) {
    // ...
  }
  ```

---

## 5. Group Arguments to Prevent Monolithic Signatures

### Problem
Passing more than 4 or 5 parameters to a function reduces readability and easily triggers static analysis warnings (e.g., Clippy's `too_many_arguments`).

### Best Practice
When a function requires a large set of related variables, encapsulate them inside a descriptive local parameter structure or state object.
- **Example**: In `main.rs`, we resolved an 8-parameter signature by grouping all variables into a single context structure:
  ```rust
  struct IndexRunSummary<'a> {
    connection: &'a rusqlite::Connection,
    started_run_id: i64,
    discovered_files: &'a [rkg_indexer::DiscoveredFile],
    changed_files: &'a [rkg_indexer::DiscoveredFile],
    deleted_files: &'a [ExistingFileSnapshot],
    parse_summary: &'a rkg_indexer::PythonParseSummary,
    repo_root: &'a std::path::Path,
    force: bool,
  }
  
  fn finish_and_report_index_run(summary: &IndexRunSummary) -> Result<(), String> {
    // ...
  }
  ```

---

## 6. Isolate Side Effects from Pure Resolution

### Problem
Indexing orchestration becomes difficult to reason about when filesystem reads,
database writes, and pure dependency resolution all live inside one command handler.
That shape makes small design-conformance fixes risky because each edit has to
preserve several unrelated concerns at once.

### Best Practice
Keep side effects at the boundary and make the transformation step explicit.
- Read repository metadata in a focused loader.
- Resolve inherited or lockfile-backed values in a pure helper.
- Persist records in a separate database helper.
- Pass repeated indexing state through a typed context instead of long argument lists.

---

## 7. Prefer Named Type Aliases over Clippy Suppressions

### Problem
Tuple-heavy query APIs can trigger `clippy::type_complexity`. Suppressing the lint
keeps the code compiling, but it hides the meaning of each row shape from readers.

### Best Practice
When preserving the existing tuple ABI is important, introduce named type aliases
with concise Rustdoc instead of immediately migrating public APIs to structs.
This keeps callers compatible while documenting field order.

---

## 8. Treat Design Audits as Bounded Implementation Inputs

### Problem
Original design documents often contain a mix of current-phase requirements and
long-term product intent. Implementing every discovered gap during a cleanup PR
turns a quality pass into an unbounded feature project.

### Best Practice
Classify audit findings before editing:
- Implement only bounded gaps that fit existing schema, CLI, and crate boundaries.
- Record larger items in `SPEC.md` Future sections.
- Keep `_workspace/` audit notes explicit about what was fixed and what was deferred.

---

## 9. Keep Documentation and Version Metadata Synchronized

### Problem
Adding a language adapter touches code, CLI behavior, schema facts, tests, and
release state. If only some documents are updated, users and future agents can
misread implemented support as future work or assume stale command coverage.

### Best Practice
Treat release bookkeeping as part of the same change that completes user-visible
language support.
- Update `README.md`, `SPEC.md`, `ARCHITECTURE.md`, `CHANGELOG.md`, and the user
  manual when supported languages, commands, or workflows change.
- Keep `docs/project-proposal.md` as historical intent unless explicitly asked to
  revise the initial proposal.
- Update `[workspace.package] version` and `Cargo.lock` together.
- Run a stale-doc grep for old phase, language, and version claims before
  finalizing.
