use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use clap::Parser;
use rkg_cli::{AndroidCommands, Cli, Commands, DbCommands, McpCommands};
use rkg_db::{
  FinishedIndexRunRecord, NewAndroidComponentRecord, NewAndroidResourceRecord, NewFileRecord,
  NewIndexRunRecord,
};
use rkg_indexer::{
  ExistingFileSnapshot, analyze_python_parse, classify_incremental_diff, detect_repo_root,
  discover_files,
};
use rkg_lang_python::{
  extract_calls_from_source, extract_decorators_from_source, extract_imports_from_source,
  extract_symbols_from_source, extract_tests_from_source, extract_type_references_from_source,
};

fn main() {
  let cli = Cli::parse();
  if let Err(error) = run(cli) {
    eprintln!("error: {error}");
    std::process::exit(1);
  }
}

fn run(cli: Cli) -> Result<(), String> {
  match cli.command {
    Commands::Init => run_init(),
    Commands::Index {
      force,
      changed: _,
      expand,
    } => run_index(force, expand),
    Commands::Files => run_files(),
    Commands::Symbols => run_symbols(),
    Commands::Find { name } => run_find(&name),
    Commands::Show { qualified_name } => run_show(&qualified_name),
    Commands::Imports { path } => run_imports(&path),
    Commands::ImportedBy { path } => run_imported_by(&path),
    Commands::Callees { name } => run_callees(&name),
    Commands::Callers { name } => run_callers(&name),
    Commands::Types { name } => run_types(&name),
    Commands::Decorators { name } => run_decorators(&name),
    Commands::Tests { name } => run_tests(&name),
    Commands::TestDeps { name } => run_test_deps(&name),
    Commands::Fixtures { name } => run_fixtures(&name),
    Commands::Docs { name } => run_docs(&name),
    Commands::DocSearch { query } => run_doc_search(&query),
    Commands::Search { query } => run_search(&query),
    Commands::Impact { symbol, depth } => run_impact(&symbol, depth),
    Commands::Context {
      symbol,
      budget,
      format,
    } => run_context(&symbol, budget, &format),
    Commands::Git { path } => run_git(&path),
    Commands::Cochange { name } => run_cochange(&name),
    Commands::Routes => run_routes(),
    Commands::Model { name } => run_model(&name),
    Commands::Pipeline { name } => run_pipeline(&name),
    Commands::Concurrency { name } => run_concurrency(&name),
    Commands::Safety { target } => run_safety(&target),
    Commands::Coverage { target } => run_coverage(&target),
    Commands::ImportCoverage { path, test_suite } => run_import_coverage(&path, test_suite),
    Commands::Workspace => run_workspace(),
    Commands::Topology => run_topology(),
    Commands::Deps { package } => run_deps(&package),
    Commands::Android { command } => match command {
      AndroidCommands::Components => run_android_components(),
      AndroidCommands::Resources => run_android_resources(),
    },
    Commands::Db { command } => match command {
      DbCommands::Status => run_db_status(),
      DbCommands::Reset => run_db_reset(),
    },
    Commands::Mcp { command } => match command {
      McpCommands::Serve => run_mcp_serve(),
    },
    Commands::Bench {
      config,
      json,
      output,
    } => run_bench(config, json, output),
  }
}

fn run_init() -> Result<(), String> {
  let db_path = default_db_path()?;
  ensure_db_initialized(&db_path)?;
  println!("initialized database at {}", db_path.display());
  Ok(())
}

fn run_db_status() -> Result<(), String> {
  let db_path = default_db_path()?;
  let exists = db_path.exists();
  println!("database path: {}", db_path.display());
  println!("exists: {}", if exists { "yes" } else { "no" });

  if !exists {
    return Ok(());
  }

  let connection = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
  let status = rkg_db::db_status(&connection).map_err(|e| e.to_string())?;
  println!(
    "schema initialized: {}",
    if status.schema_initialized {
      "yes"
    } else {
      "no"
    }
  );
  if status.schema_initialized {
    println!("repositories: {}", status.repositories);
    println!("files: {}", status.files);
    println!("symbols: {}", status.symbols);
    println!("edges: {}", status.edges);
    println!("docs: {}", status.docs);
    println!("tests: {}", status.tests);
    println!("index_runs: {}", status.index_runs);
    println!("generated_symbols: {}", status.generated_symbols);
    println!("android_components: {}", status.android_components);
    println!("android_resources: {}", status.android_resources);
  }

  Ok(())
}

fn run_db_reset() -> Result<(), String> {
  let db_path = default_db_path()?;
  if db_path.exists() {
    fs::remove_file(&db_path).map_err(|e| {
      format!(
        "failed to remove existing database {}: {e}",
        db_path.display()
      )
    })?;
  }
  ensure_db_initialized(&db_path)?;
  println!("reset database at {}", db_path.display());
  Ok(())
}

fn run_index(force: bool, expand: bool) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository =
    rkg_db::upsert_repository(&connection, &repo_root_text).map_err(|e| e.to_string())?;
  let started_run = rkg_db::start_index_run(
    &connection,
    &NewIndexRunRecord {
      repository_id: repository.id,
      status: "running".to_string(),
    },
  )
  .map_err(|e| e.to_string())?;

  let discovered_files = discover_files(&repo_root).map_err(|e| e.to_string())?;
  let existing_files = get_existing_file_snapshots(&connection, repository.id)?;

  let diff = classify_incremental_diff(&discovered_files, &existing_files);
  let changed_files = if force {
    discovered_files.clone()
  } else {
    diff.changed
  };
  let deleted_files = diff.deleted;
  let parse_summary =
    analyze_python_parse(&repo_root, &changed_files).map_err(|e| e.to_string())?;

  let cargo_metadata = load_cargo_workspace_metadata(&repo_root, &discovered_files);
  persist_cargo_workspace_metadata(&connection, repository.id, &cargo_metadata)?;

  let fsharp_metadata = load_fsharp_workspace_metadata(&repo_root, &discovered_files);
  persist_fsharp_workspace_metadata(&connection, repository.id, &fsharp_metadata)?;

  let kotlin_metadata = load_kotlin_workspace_metadata(&repo_root, &discovered_files);
  persist_kotlin_workspace_metadata(&connection, repository.id, &kotlin_metadata)?;

  let swift_metadata = load_swift_workspace_metadata(&repo_root, &discovered_files);
  persist_swift_workspace_metadata(&connection, repository.id, &swift_metadata)?;

  for file in &deleted_files {
    rkg_db::delete_file_by_path(&connection, repository.id, &file.path)
      .map_err(|e| e.to_string())?;
  }

  let kotlin_projects =
    rkg_db::list_kotlin_projects(&connection, repository.id).unwrap_or_default();

  for file in &changed_files {
    index_single_file(
      &IndexFileContext {
        connection: &connection,
        repository_id: repository.id,
        repo_root: &repo_root,
        index_run_id: started_run.id,
        parsed_packages: &cargo_metadata.packages,
        kotlin_projects: &kotlin_projects,
      },
      file,
    )?;
  }

  if expand {
    run_cargo_expand_indexing(
      &connection,
      repository.id,
      &repo_root,
      &cargo_metadata.packages,
    )?;
  }

  rkg_db::resolve_unresolved_edges(&connection, repository.id).map_err(|e| e.to_string())?;
  rkg_db::resolve_direct_call_test_linkages(&connection, repository.id)
    .map_err(|e| e.to_string())?;
  rkg_db::resolve_symbol_doc_linkages(&connection, repository.id).map_err(|e| e.to_string())?;
  rkg_db::resolve_generated_symbols_linkages(&connection, repository.id)
    .map_err(|e| e.to_string())?;

  index_git_history(&connection, repository.id, &repo_root)?;

  finish_and_report_index_run(&IndexRunSummary {
    connection: &connection,
    started_run_id: started_run.id,
    discovered_files: &discovered_files,
    changed_files: &changed_files,
    deleted_files: &deleted_files,
    parse_summary: &parse_summary,
    repo_root: &repo_root,
    force,
  })?;

  Ok(())
}

fn get_existing_file_snapshots(
  connection: &rusqlite::Connection,
  repository_id: i64,
) -> Result<Vec<ExistingFileSnapshot>, String> {
  let files =
    rkg_db::list_files_for_repository(connection, repository_id).map_err(|e| e.to_string())?;
  Ok(
    files
      .into_iter()
      .map(|file| ExistingFileSnapshot {
        path: file.path,
        content_hash: file.content_hash,
      })
      .collect(),
  )
}

struct CargoWorkspaceMetadata {
  packages: Vec<rkg_indexer::cargo_parser::ParsedPackage>,
}

fn load_cargo_workspace_metadata(
  repo_root: &std::path::Path,
  discovered_files: &[rkg_indexer::DiscoveredFile],
) -> CargoWorkspaceMetadata {
  let mut packages = Vec::new();
  let mut workspace_deps = HashMap::new();
  let mut resolved_lock_versions = HashMap::new();

  for file in discovered_files {
    if file.path == "Cargo.lock" || file.path.ends_with("/Cargo.lock") {
      let file_path_abs = repo_root.join(&file.path);
      if let Ok(content) = fs::read_to_string(&file_path_abs) {
        for (name, version) in rkg_indexer::cargo_parser::parse_cargo_lock(&content) {
          resolved_lock_versions.insert(name, version);
        }
      }
    }
  }

  for file in discovered_files {
    if file.path == "Cargo.toml" || file.path.ends_with("/Cargo.toml") {
      let file_path_abs = repo_root.join(&file.path);
      if let Ok(content) = fs::read_to_string(&file_path_abs) {
        let (package, workspace) =
          rkg_indexer::cargo_parser::parse_cargo_toml(&content, &file.path);
        if let Some(package) = package {
          packages.push(package);
        }
        if let Some(workspace) = workspace {
          workspace_deps.extend(workspace.workspace_dependencies);
        }
      }
    }
  }

  CargoWorkspaceMetadata {
    packages: resolve_workspace_dependencies(packages, &workspace_deps, &resolved_lock_versions),
  }
}

fn resolve_workspace_dependencies(
  mut packages: Vec<rkg_indexer::cargo_parser::ParsedPackage>,
  workspace_deps: &HashMap<String, rkg_indexer::cargo_parser::ParsedDependency>,
  resolved_lock_versions: &HashMap<String, String>,
) -> Vec<rkg_indexer::cargo_parser::ParsedPackage> {
  for package in &mut packages {
    for dependency in &mut package.dependencies {
      if dependency.inherits_workspace
        && let Some(workspace_dependency) = workspace_deps.get(&dependency.name)
      {
        dependency.is_workspace_dependency = workspace_dependency.is_workspace_dependency;
        if dependency.version_requirement.is_none() {
          dependency.version_requirement = workspace_dependency.version_requirement.clone();
        }
        for feature in &workspace_dependency.features {
          if !dependency.features.contains(feature) {
            dependency.features.push(feature.clone());
          }
        }
      }

      if !dependency.is_workspace_dependency
        && let Some(exact_version) = resolved_lock_versions.get(&dependency.name)
      {
        dependency.version_requirement = Some(exact_version.clone());
      }
    }
  }

  packages
}

fn persist_cargo_workspace_metadata(
  connection: &rusqlite::Connection,
  repository_id: i64,
  metadata: &CargoWorkspaceMetadata,
) -> Result<(), String> {
  for package in &metadata.packages {
    let package_record = rkg_db::insert_cargo_package(
      connection,
      &rkg_db::NewCargoPackageRecord {
        repository_id,
        name: package.name.clone(),
        manifest_path: package.manifest_path.clone(),
        version: package.version.clone(),
        is_workspace_member: true,
      },
    )
    .map_err(|e| format!("Failed to insert Cargo package {}: {}", package.name, e))?;

    rkg_db::delete_cargo_dependencies_for_package(connection, package_record.id).map_err(|e| {
      format!(
        "Failed to clear Cargo dependencies for {}: {}",
        package.name, e
      )
    })?;

    for dependency in &package.dependencies {
      rkg_db::insert_cargo_dependency(
        connection,
        &rkg_db::NewCargoDependencyRecord {
          package_id: package_record.id,
          name: dependency.name.clone(),
          version_requirement: dependency.version_requirement.clone(),
          is_workspace_dependency: dependency.is_workspace_dependency,
          features: dependency.features.join(","),
          is_dev: dependency.is_dev,
        },
      )
      .map_err(|e| {
        format!(
          "Failed to insert Cargo dependency {} for {}: {}",
          dependency.name, package.name, e
        )
      })?;
    }
  }

  Ok(())
}

struct FSharpWorkspaceMetadata {
  projects: Vec<rkg_indexer::fsharp_parser::ParsedFSharpProject>,
  solution_projects: std::collections::HashSet<String>,
}

fn load_fsharp_workspace_metadata(
  repo_root: &std::path::Path,
  discovered_files: &[rkg_indexer::DiscoveredFile],
) -> FSharpWorkspaceMetadata {
  let mut projects = Vec::new();
  let mut solution_projects = std::collections::HashSet::new();
  let mut global_paket_deps = HashMap::new();
  let mut project_paket_refs = HashMap::new();

  // 1. Scan for paket.dependencies
  for file in discovered_files {
    if file.path == "paket.dependencies" || file.path.ends_with("/paket.dependencies") {
      let file_path_abs = repo_root.join(&file.path);
      if let Ok(content) = fs::read_to_string(&file_path_abs) {
        global_paket_deps.extend(rkg_indexer::fsharp_parser::parse_paket_dependencies(
          &content,
        ));
      }
    }
  }

  // 2. Scan for solution files (.sln) to identify solution membership
  for file in discovered_files {
    if file.path.ends_with(".sln") {
      let file_path_abs = repo_root.join(&file.path);
      if let Ok(content) = fs::read_to_string(&file_path_abs) {
        let parsed_paths = rkg_indexer::fsharp_parser::parse_sln(&content);
        for p in parsed_paths {
          let sln_dir = PathBuf::from(&file.path)
            .parent()
            .unwrap_or_else(|| std::path::Path::new(""))
            .to_path_buf();
          // Normalize the joined path to collapse `..` components so that
          // e.g. `solutions/../src/Lib/Lib.fsproj` becomes `src/Lib/Lib.fsproj`.
          let raw = sln_dir.join(p);
          let normalized = normalize_relative_path(&raw);
          solution_projects.insert(normalized);
        }
      }
    }
  }

  // 3. Scan for paket.references next to projects
  for file in discovered_files {
    if file.path.ends_with("paket.references") {
      let file_path_abs = repo_root.join(&file.path);
      if let Ok(content) = fs::read_to_string(&file_path_abs) {
        let refs = rkg_indexer::fsharp_parser::parse_paket_references(&content);
        let parent_dir = PathBuf::from(&file.path)
          .parent()
          .unwrap_or_else(|| std::path::Path::new(""))
          .to_string_lossy()
          .to_string();
        project_paket_refs.insert(parent_dir, refs);
      }
    }
  }

  // 4. Scan for .fsproj files
  for file in discovered_files {
    if file.path.ends_with(".fsproj") {
      let file_path_abs = repo_root.join(&file.path);
      if let Ok(content) = fs::read_to_string(&file_path_abs) {
        let mut parsed = rkg_indexer::fsharp_parser::parse_fsproj(&content, &file.path);

        // Resolve Paket references if paket.references exists in the same directory
        let parent_dir = PathBuf::from(&file.path)
          .parent()
          .unwrap_or_else(|| std::path::Path::new(""))
          .to_string_lossy()
          .to_string();
        if let Some(refs) = project_paket_refs.get(&parent_dir) {
          for r in refs {
            let version = global_paket_deps.get(r).cloned();
            parsed
              .dependencies
              .push(rkg_indexer::fsharp_parser::ParsedFSharpDependency {
                name: r.clone(),
                dependency_type: "package".to_string(),
                version_requirement: version,
              });
          }
        }

        projects.push(parsed);
      }
    }
  }

  FSharpWorkspaceMetadata {
    projects,
    solution_projects,
  }
}

fn persist_fsharp_workspace_metadata(
  connection: &rusqlite::Connection,
  repository_id: i64,
  metadata: &FSharpWorkspaceMetadata,
) -> Result<(), String> {
  let tx = connection
    .unchecked_transaction()
    .map_err(|e| format!("Failed to start transaction: {e}"))?;

  // Prune stale projects that are no longer present in the current scan.
  let current_paths: Vec<String> = metadata
    .projects
    .iter()
    .map(|p| p.project_path.clone())
    .collect();
  rkg_db::delete_fsharp_projects_not_in_paths(&tx, repository_id, &current_paths)
    .map_err(|e| format!("Failed to prune stale F# projects: {e}"))?;

  for project in &metadata.projects {
    let normalized_proj_path = project.project_path.replace('\\', "/");
    let is_member = metadata
      .solution_projects
      .iter()
      .any(|p| p == &normalized_proj_path || normalized_proj_path.ends_with(p));

    let project_record = rkg_db::insert_fsharp_project(
      &tx,
      &rkg_db::NewFSharpProjectRecord {
        repository_id,
        name: project.name.clone(),
        project_path: project.project_path.clone(),
        target_framework: project.target_framework.clone(),
        is_solution_member: is_member,
      },
    )
    .map_err(|e| format!("Failed to insert F# project {}: {}", project.name, e))?;

    rkg_db::delete_fsharp_dependencies_for_project(&tx, project_record.id).map_err(|e| {
      format!(
        "Failed to clear F# dependencies for {}: {}",
        project.name, e
      )
    })?;

    for dependency in &project.dependencies {
      rkg_db::insert_fsharp_dependency(
        &tx,
        &rkg_db::NewFSharpDependencyRecord {
          project_id: project_record.id,
          name: dependency.name.clone(),
          dependency_type: dependency.dependency_type.clone(),
          version_requirement: dependency.version_requirement.clone(),
        },
      )
      .map_err(|e| {
        format!(
          "Failed to insert F# dependency {} for {}: {}",
          dependency.name, project.name, e
        )
      })?;
    }
  }

  tx.commit()
    .map_err(|e| format!("Failed to commit transaction: {e}"))?;
  Ok(())
}

struct KotlinWorkspaceMetadata {
  projects: Vec<rkg_indexer::kotlin_project_parser::ParsedKotlinProject>,
  solution_projects: std::collections::HashSet<String>,
}

fn load_kotlin_workspace_metadata(
  repo_root: &std::path::Path,
  discovered_files: &[rkg_indexer::DiscoveredFile],
) -> KotlinWorkspaceMetadata {
  let mut projects = Vec::new();
  let mut solution_projects = std::collections::HashSet::new();

  // 1. Scan settings.gradle or settings.gradle.kts to identify submodules
  for file in discovered_files {
    let is_settings = file.path == "settings.gradle"
      || file.path == "settings.gradle.kts"
      || file.path.ends_with("/settings.gradle")
      || file.path.ends_with("/settings.gradle.kts");
    if is_settings {
      let file_path_abs = repo_root.join(&file.path);
      if let Ok(content) = fs::read_to_string(&file_path_abs) {
        for line in content.lines() {
          let line = line.trim();
          let line = if let Some(idx) = line.find("//") {
            &line[..idx]
          } else {
            line
          };
          let line = line.trim();
          if line.starts_with("include") {
            let mut current = String::new();
            let mut in_quotes = false;
            let mut quote_char = ' ';
            for c in line.chars() {
              if c == '"' || c == '\'' {
                if in_quotes {
                  if c == quote_char {
                    in_quotes = false;
                    let clean = current.trim_start_matches(':').trim().to_string();
                    if !clean.is_empty() {
                      solution_projects.insert(clean);
                    }
                    current.clear();
                  } else {
                    current.push(c);
                  }
                } else {
                  in_quotes = true;
                  quote_char = c;
                }
              } else if in_quotes {
                current.push(c);
              }
            }
          }
        }
      }
    }
  }

  // 2. Scan for build.gradle, build.gradle.kts, and pom.xml files
  for file in discovered_files {
    let is_project_file = file.path == "build.gradle"
      || file.path.ends_with("/build.gradle")
      || file.path == "build.gradle.kts"
      || file.path.ends_with("/build.gradle.kts")
      || file.path == "pom.xml"
      || file.path.ends_with("/pom.xml");

    if is_project_file {
      let file_path_abs = repo_root.join(&file.path);
      if let Ok(content) = fs::read_to_string(&file_path_abs) {
        let parsed = if file.path.ends_with("pom.xml") {
          rkg_indexer::kotlin_project_parser::parse_pom(&content, &file.path)
        } else {
          rkg_indexer::kotlin_project_parser::parse_gradle(&content, &file.path)
        };
        projects.push(parsed);
      }
    }
  }

  KotlinWorkspaceMetadata {
    projects,
    solution_projects,
  }
}

fn persist_kotlin_workspace_metadata(
  connection: &rusqlite::Connection,
  repository_id: i64,
  metadata: &KotlinWorkspaceMetadata,
) -> Result<(), String> {
  let tx = connection
    .unchecked_transaction()
    .map_err(|e| format!("Failed to start transaction: {e}"))?;

  let current_paths: Vec<String> = metadata
    .projects
    .iter()
    .map(|p| p.project_path.clone())
    .collect();
  rkg_db::delete_kotlin_projects_not_in_paths(&tx, repository_id, &current_paths)
    .map_err(|e| format!("Failed to prune stale Kotlin projects: {e}"))?;

  for project in &metadata.projects {
    let is_member = metadata.solution_projects.contains(&project.name)
      || metadata
        .solution_projects
        .iter()
        .any(|m| project.project_path.contains(m));

    let project_dir = std::path::Path::new(&project.project_path)
      .parent()
      .map(|p| p.to_string_lossy().to_string())
      .unwrap_or_default();

    let resolved_generated_dirs: Vec<String> = project
      .generated_source_dirs
      .iter()
      .map(|dir| {
        let joined = if project_dir.is_empty() {
          dir.clone()
        } else {
          format!("{}/{}", project_dir, dir)
        };
        joined.replace('\\', "/")
      })
      .collect();

    let generated_source_dirs_opt = if resolved_generated_dirs.is_empty() {
      None
    } else {
      Some(resolved_generated_dirs.join(","))
    };

    let project_record = rkg_db::insert_kotlin_project(
      &tx,
      &rkg_db::NewKotlinProjectRecord {
        repository_id,
        name: project.name.clone(),
        project_path: project.project_path.clone(),
        target_framework: project.target_framework.clone(),
        is_solution_member: is_member,
        generated_source_dirs: generated_source_dirs_opt,
      },
    )
    .map_err(|e| format!("Failed to insert Kotlin project {}: {}", project.name, e))?;

    rkg_db::delete_kotlin_dependencies_for_project(&tx, project_record.id).map_err(|e| {
      format!(
        "Failed to clear Kotlin dependencies for {}: {}",
        project.name, e
      )
    })?;

    for dependency in &project.dependencies {
      rkg_db::insert_kotlin_dependency(
        &tx,
        &rkg_db::NewKotlinDependencyRecord {
          project_id: project_record.id,
          name: dependency.name.clone(),
          dependency_type: dependency.dependency_type.clone(),
          version_requirement: dependency.version_requirement.clone(),
        },
      )
      .map_err(|e| {
        format!(
          "Failed to insert Kotlin dependency {} for {}: {}",
          dependency.name, project.name, e
        )
      })?;
    }
  }

  tx.commit()
    .map_err(|e| format!("Failed to commit transaction: {e}"))?;
  Ok(())
}

struct SwiftWorkspaceMetadata {
  projects: Vec<rkg_indexer::swift_project_parser::ParsedSwiftProject>,
  solution_projects: std::collections::HashSet<String>,
}

fn load_swift_workspace_metadata(
  repo_root: &std::path::Path,
  discovered_files: &[rkg_indexer::DiscoveredFile],
) -> SwiftWorkspaceMetadata {
  let mut projects = Vec::new();
  let mut solution_projects = std::collections::HashSet::new();

  for file in discovered_files {
    let is_package_swift = file.path == "Package.swift" || file.path.ends_with("/Package.swift");

    if is_package_swift {
      let file_path_abs = repo_root.join(&file.path);
      if let Ok(content) = fs::read_to_string(&file_path_abs) {
        let parsed = rkg_indexer::swift_project_parser::parse_package_swift(&content, &file.path);

        solution_projects.insert(parsed.name.clone());
        for dep in &parsed.dependencies {
          if dep.dependency_type == "project" {
            solution_projects.insert(dep.name.clone());
          }
        }
        projects.push(parsed);
      }
    }
  }

  SwiftWorkspaceMetadata {
    projects,
    solution_projects,
  }
}

fn persist_swift_workspace_metadata(
  connection: &rusqlite::Connection,
  repository_id: i64,
  metadata: &SwiftWorkspaceMetadata,
) -> Result<(), String> {
  let tx = connection
    .unchecked_transaction()
    .map_err(|e| format!("Failed to start transaction: {e}"))?;

  let current_paths: Vec<String> = metadata
    .projects
    .iter()
    .map(|p| p.project_path.clone())
    .collect();
  rkg_db::delete_swift_projects_not_in_paths(&tx, repository_id, &current_paths)
    .map_err(|e| format!("Failed to prune stale Swift projects: {e}"))?;

  for project in &metadata.projects {
    let is_member = metadata.solution_projects.contains(&project.name)
      || metadata
        .solution_projects
        .iter()
        .any(|m| project.project_path.contains(m));

    let project_record = rkg_db::insert_swift_project(
      &tx,
      &rkg_db::NewSwiftProjectRecord {
        repository_id,
        name: project.name.clone(),
        project_path: project.project_path.clone(),
        target_framework: project.target_framework.clone(),
        is_solution_member: is_member,
      },
    )
    .map_err(|e| format!("Failed to insert Swift project {}: {}", project.name, e))?;

    rkg_db::delete_swift_dependencies_for_project(&tx, project_record.id).map_err(|e| {
      format!(
        "Failed to clear Swift dependencies for {}: {}",
        project.name, e
      )
    })?;

    for dependency in &project.dependencies {
      rkg_db::insert_swift_dependency(
        &tx,
        &rkg_db::NewSwiftDependencyRecord {
          project_id: project_record.id,
          name: dependency.name.clone(),
          dependency_type: dependency.dependency_type.clone(),
          version_requirement: dependency.version_requirement.clone(),
        },
      )
      .map_err(|e| {
        format!(
          "Failed to insert Swift dependency {} for {}: {}",
          dependency.name, project.name, e
        )
      })?;
    }
  }

  tx.commit()
    .map_err(|e| format!("Failed to commit transaction: {e}"))?;
  Ok(())
}

struct IndexFileContext<'a> {
  connection: &'a rusqlite::Connection,
  repository_id: i64,
  repo_root: &'a std::path::Path,
  index_run_id: i64,
  parsed_packages: &'a [rkg_indexer::cargo_parser::ParsedPackage],
  kotlin_projects: &'a [rkg_db::KotlinProjectRecord],
}

fn index_single_file(
  context: &IndexFileContext,
  file: &rkg_indexer::DiscoveredFile,
) -> Result<(), String> {
  let connection = context.connection;
  let repository_id = context.repository_id;
  let repo_root = context.repo_root;
  let parsed_packages = context.parsed_packages;

  let tx = connection
    .unchecked_transaction()
    .map_err(|e| e.to_string())?;

  let file_record = rkg_db::reindex_file_raw(
    &tx,
    &NewFileRecord {
      repository_id,
      path: file.path.clone(),
      language: file.language.clone(),
      content_hash: Some(file.content_hash.clone()),
      line_count: Some(file.line_count as i64),
      last_index_run_id: Some(context.index_run_id),
    },
  )
  .map_err(|e| e.to_string())?;

  let normalized_path = file.path.replace('\\', "/");
  let parts: Vec<&str> = normalized_path.split('/').collect();
  let mut is_drawable = false;
  for i in 0..parts.len() {
    if parts[i] == "res" && i + 1 < parts.len() {
      let folder = parts[i + 1];
      if folder.starts_with("drawable") {
        is_drawable = true;
      }
    }
  }

  if is_drawable {
    let file_name = std::path::Path::new(&file.path)
      .file_stem()
      .and_then(|s| s.to_str())
      .unwrap_or(&file.path);

    rkg_db::insert_android_resource(
      &tx,
      &NewAndroidResourceRecord {
        file_id: file_record.id,
        name: file_name.to_string(),
        resource_type: "drawable".to_string(),
        value: None,
        start_line: Some(1),
        end_line: Some(1),
      },
    )
    .map_err(|e| e.to_string())?;

    let qname = format!("R.drawable.{}", file_name);
    rkg_db::insert_symbol(
      &tx,
      &rkg_db::NewSymbolRecord {
        file_id: file_record.id,
        name: file_name.to_string(),
        qualified_name: qname,
        kind: "Struct".to_string(),
        start_line: 1,
        end_line: 1,
        start_column: None,
        end_column: None,
      },
    )
    .map_err(|e| e.to_string())?;
  }

  if file.language.as_deref() == Some("python") {
    let file_path_abs = repo_root.join(&file.path);
    let content = fs::read_to_string(&file_path_abs)
      .map_err(|e| format!("failed to read file {}: {e}", file_path_abs.display()))?;

    let module_symbol_id = index_file_symbols(&tx, file_record.id, &file.path, &content)?;

    if let Some(mod_sym_id) = module_symbol_id {
      index_file_imports(&tx, repository_id, &file.path, &content, mod_sym_id)?;
    }

    index_file_calls(&tx, repository_id, &file.path, &content)?;
    index_file_type_references(&tx, repository_id, &file.path, &content)?;
    index_file_decorators(&tx, repository_id, &file.path, &content)?;
    index_file_tests(&tx, repository_id, file_record.id, &file.path, &content)?;
    index_file_docstrings(&tx, repository_id, file_record.id, &file.path, &content)?;
    index_file_routes(&tx, repository_id, file_record.id, &file.path, &content)?;
    index_file_pydantic_models(&tx, repository_id, file_record.id, &file.path, &content)?;
  } else if file.language.as_deref() == Some("rust") {
    let file_path_abs = repo_root.join(&file.path);
    let content = fs::read_to_string(&file_path_abs)
      .map_err(|e| format!("failed to read file {}: {e}", file_path_abs.display()))?;

    let mut active_features = Vec::new();
    if let Some(pkg) =
      rkg_indexer::cargo_parser::find_cargo_package_for_path(parsed_packages, &file.path)
    {
      active_features.push("default".to_string());
      if let Some(feats) = pkg.features.get("default") {
        for feat in feats {
          active_features.push(feat.clone());
        }
      }
    }

    let module_symbol_id =
      index_rust_file_symbols(&tx, file_record.id, &file.path, &content, &active_features)?;
    if let Some(mod_sym_id) = module_symbol_id {
      index_rust_file_imports(
        &tx,
        repository_id,
        &file.path,
        &content,
        mod_sym_id,
        &active_features,
      )?;
    }
    index_rust_file_calls(&tx, repository_id, &file.path, &content, &active_features)?;
    index_rust_file_type_references(&tx, repository_id, &file.path, &content, &active_features)?;
    index_rust_file_implementations(&tx, repository_id, &file.path, &content, &active_features)?;
    index_rust_file_tests(
      &tx,
      repository_id,
      file_record.id,
      &file.path,
      &content,
      &active_features,
    )?;
    index_rust_file_concurrency(
      &tx,
      repository_id,
      file_record.id,
      &file.path,
      &content,
      &active_features,
    )?;
    index_rust_file_safety(
      &tx,
      repository_id,
      file_record.id,
      &file.path,
      &content,
      &active_features,
    )?;
    index_file_routes(&tx, repository_id, file_record.id, &file.path, &content)?;
  } else if file.language.as_deref() == Some("fsharp") {
    let file_path_abs = repo_root.join(&file.path);
    let content = fs::read_to_string(&file_path_abs)
      .map_err(|e| format!("failed to read file {}: {e}", file_path_abs.display()))?;
    let module_symbol_id = index_fsharp_file_symbols(&tx, file_record.id, &file.path, &content)?;
    if let Some(mod_sym_id) = module_symbol_id {
      index_fsharp_file_imports(&tx, repository_id, &file.path, &content, mod_sym_id)?;
    }
    index_fsharp_file_calls(&tx, repository_id, &file.path, &content)?;
    index_fsharp_file_type_references(&tx, repository_id, &file.path, &content)?;
    index_fsharp_file_tests(&tx, repository_id, file_record.id, &file.path, &content)?;
    index_fsharp_file_routes(&tx, repository_id, file_record.id, &file.path, &content)?;
  } else if file.language.as_deref() == Some("mojo") {
    let file_path_abs = repo_root.join(&file.path);
    let content = fs::read_to_string(&file_path_abs)
      .map_err(|e| format!("failed to read file {}: {e}", file_path_abs.display()))?;
    let module_symbol_id = index_mojo_file_symbols(&tx, file_record.id, &file.path, &content)?;
    if let Some(mod_sym_id) = module_symbol_id {
      index_mojo_file_imports(&tx, repository_id, &file.path, &content, mod_sym_id)?;
    }
    index_mojo_file_calls(&tx, repository_id, &file.path, &content)?;
    index_mojo_file_type_references(&tx, repository_id, &file.path, &content)?;
    index_mojo_file_tests(&tx, repository_id, file_record.id, &file.path, &content)?;
  } else if file.language.as_deref() == Some("kotlin") {
    let file_path_abs = repo_root.join(&file.path);
    let content = fs::read_to_string(&file_path_abs)
      .map_err(|e| format!("failed to read file {}: {e}", file_path_abs.display()))?;
    let module_symbol_id = index_kotlin_file_symbols(
      &tx,
      repository_id,
      file_record.id,
      &file.path,
      &content,
      context.kotlin_projects,
    )?;
    if let Some(mod_sym_id) = module_symbol_id {
      index_kotlin_file_imports(&tx, repository_id, &file.path, &content, mod_sym_id)?;
    }
    index_kotlin_file_calls(&tx, repository_id, &file.path, &content)?;
    index_kotlin_file_type_references(&tx, repository_id, &file.path, &content)?;
    index_kotlin_file_inheritance_and_interfaces(&tx, repository_id, &file.path, &content)?;
    index_kotlin_file_annotations(&tx, repository_id, &file.path, &content)?;
    index_kotlin_file_concurrency(&tx, repository_id, file_record.id, &file.path, &content)?;
    index_kotlin_file_routes(&tx, repository_id, file_record.id, &file.path, &content)?;
  } else if file.language.as_deref() == Some("swift") {
    let file_path_abs = repo_root.join(&file.path);
    let content = fs::read_to_string(&file_path_abs)
      .map_err(|e| format!("failed to read file {}: {e}", file_path_abs.display()))?;
    let module_symbol_id = index_swift_file_symbols(&tx, file_record.id, &file.path, &content)?;
    if let Some(mod_sym_id) = module_symbol_id {
      index_swift_file_imports(&tx, repository_id, &file.path, &content, mod_sym_id)?;
    }
    index_swift_file_calls(&tx, repository_id, &file.path, &content)?;
    index_swift_file_type_references(&tx, repository_id, &file.path, &content)?;
    index_swift_file_inheritance_and_interfaces(&tx, repository_id, &file.path, &content)?;
    index_swift_file_attributes(&tx, repository_id, &file.path, &content)?;
    index_swift_file_tests(&tx, repository_id, file_record.id, &file.path, &content)?;
    index_swift_file_concurrency(&tx, file_record.id, &file.path, &content)?;
  } else if file.language.as_deref() == Some("markdown") {
    let file_path_abs = repo_root.join(&file.path);
    let content = fs::read_to_string(&file_path_abs)
      .map_err(|e| format!("failed to read file {}: {e}", file_path_abs.display()))?;
    index_markdown_file(&tx, file_record.id, &content)?;
  } else if file.language.as_deref() == Some("jupyter") {
    let file_path_abs = repo_root.join(&file.path);
    let content = fs::read_to_string(&file_path_abs)
      .map_err(|e| format!("failed to read file {}: {e}", file_path_abs.display()))?;
    index_jupyter_notebook(&tx, repository_id, file_record.id, &file.path, &content)?;
  } else if file.language.as_deref() == Some("xml") {
    let file_path_abs = repo_root.join(&file.path);
    let content = fs::read_to_string(&file_path_abs)
      .map_err(|e| format!("failed to read file {}: {e}", file_path_abs.display()))?;
    index_xml_file(&tx, repository_id, file_record.id, &file.path, &content)?;
  }

  tx.commit().map_err(|e| e.to_string())?;

  Ok(())
}

#[derive(serde::Deserialize)]
struct NotebookJson {
  cells: Vec<CellJson>,
}

#[derive(serde::Deserialize)]
struct CellJson {
  cell_type: String,
  source: SourceField,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum SourceField {
  String(String),
  Array(Vec<String>),
}

fn index_jupyter_notebook(
  connection: &rusqlite::Connection,
  repository_id: i64,
  file_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  let notebook: NotebookJson = serde_json::from_str(content)
    .map_err(|e| format!("failed to parse jupyter notebook JSON: {e}"))?;

  let mut cell_idx = 0;
  for cell in notebook.cells {
    if cell.cell_type == "code" {
      let cell_code = match cell.source {
        SourceField::String(s) => s,
        SourceField::Array(arr) => arr.join(""),
      };

      if cell_code.trim().is_empty() {
        cell_idx += 1;
        continue;
      }

      let virtual_path = format!("{}#cell_{}", path, cell_idx);

      let module_symbol_id = index_file_symbols(connection, file_id, &virtual_path, &cell_code)?;

      if let Some(mod_sym_id) = module_symbol_id {
        index_file_imports(
          connection,
          repository_id,
          &virtual_path,
          &cell_code,
          mod_sym_id,
        )?;
      }

      index_file_calls(connection, repository_id, &virtual_path, &cell_code)?;
      index_file_type_references(connection, repository_id, &virtual_path, &cell_code)?;
      index_file_decorators(connection, repository_id, &virtual_path, &cell_code)?;
      index_file_tests(
        connection,
        repository_id,
        file_id,
        &virtual_path,
        &cell_code,
      )?;
      index_file_docstrings(
        connection,
        repository_id,
        file_id,
        &virtual_path,
        &cell_code,
      )?;
      index_file_routes(
        connection,
        repository_id,
        file_id,
        &virtual_path,
        &cell_code,
      )?;
      index_file_pydantic_models(
        connection,
        repository_id,
        file_id,
        &virtual_path,
        &cell_code,
      )?;

      cell_idx += 1;
    }
  }

  Ok(())
}

fn index_fsharp_file_symbols(
  connection: &rusqlite::Connection,
  file_id: i64,
  path: &str,
  content: &str,
) -> Result<Option<i64>, String> {
  let mut module_symbol_id = None;
  if let Ok(extracted_symbols) = rkg_lang_fsharp::extract_symbols_from_source(content, path) {
    for sym in extracted_symbols {
      let is_module = sym.kind == rkg_core::SymbolKind::Module;
      let symbol_record = rkg_db::insert_symbol(
        connection,
        &rkg_db::NewSymbolRecord {
          file_id,
          name: sym.name,
          qualified_name: sym.qualified_name,
          kind: format!("{:?}", sym.kind),
          start_line: sym.location.start_line as i64,
          end_line: sym.location.end_line as i64,
          start_column: sym.location.start_column.map(|c| c as i64),
          end_column: sym.location.end_column.map(|c| c as i64),
        },
      )
      .map_err(|e| e.to_string())?;

      if is_module {
        module_symbol_id = Some(symbol_record.id);
      }
    }
  }
  Ok(module_symbol_id)
}

fn index_fsharp_file_imports(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
  source_symbol_id: i64,
) -> Result<(), String> {
  if let Ok(extracted_imports) = rkg_lang_fsharp::extract_imports_from_source(content, path) {
    let mut seen = std::collections::HashSet::new();
    for imp in extracted_imports {
      if !seen.insert(imp.target_qualified_name.clone()) {
        continue;
      }

      let target_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &imp.target_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let (target_id, unresolved) = match target_symbol {
        Some(sym) => (Some(sym.id), None),
        None => (None, Some(imp.target_qualified_name.clone())),
      };

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id,
          target_symbol_id: target_id,
          unresolved_target: unresolved,
          kind: "Imports".to_string(),
          confidence: Some(1.0),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_fsharp_file_calls(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  if let Ok(extracted_calls) = rkg_lang_fsharp::extract_calls_from_source(content, path) {
    let mut seen_edges = std::collections::HashSet::new();
    for call in extracted_calls {
      let source_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &call.source_symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      let mut target_symbol_id = None;
      let mut unresolved_target = Some(call.target_name.clone());
      let mut confidence = 0.5;

      let module_name = if let Some(dot_idx) = call.source_symbol_qualified_name.find('.') {
        &call.source_symbol_qualified_name[..dot_idx]
      } else {
        &call.source_symbol_qualified_name
      };

      let local_func_qname = format!("{module_name}.{}", call.target_name);
      let resolved =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &local_func_qname)
          .map_err(|e| e.to_string())?;

      if let Some(sym) = resolved {
        target_symbol_id = Some(sym.id);
        unresolved_target = None;
        confidence = 1.0;
      }

      if !seen_edges.insert((
        source_symbol.id,
        target_symbol_id,
        unresolved_target.clone(),
      )) {
        continue;
      }

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: source_symbol.id,
          target_symbol_id,
          unresolved_target,
          kind: "Calls".to_string(),
          confidence: Some(confidence),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_fsharp_file_type_references(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  if let Ok(extracted_type_refs) =
    rkg_lang_fsharp::extract_type_references_from_source(content, path)
  {
    let mut seen_type_edges = std::collections::HashSet::new();
    for type_ref in extracted_type_refs {
      let source_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &type_ref.source_symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      let mut target_symbol_id = None;
      let mut unresolved_target = Some(type_ref.target_type_name.clone());

      let target_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &type_ref.target_type_name,
      )
      .map_err(|e| e.to_string())?;

      if let Some(sym) = target_symbol {
        target_symbol_id = Some(sym.id);
        unresolved_target = None;
      }

      if !seen_type_edges.insert((
        source_symbol.id,
        target_symbol_id,
        unresolved_target.clone(),
      )) {
        continue;
      }

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: source_symbol.id,
          target_symbol_id,
          unresolved_target,
          kind: "ReferencesType".to_string(),
          confidence: Some(1.0),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_fsharp_file_tests(
  connection: &rusqlite::Connection,
  repository_id: i64,
  file_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  if let Ok(extracted_tests) = rkg_lang_fsharp::extract_tests_from_source(content, path) {
    for t in extracted_tests {
      let kind_str = match t.kind {
        rkg_lang_fsharp::ExtractedTestKind::Class => "Class",
        rkg_lang_fsharp::ExtractedTestKind::Function => "Function",
        rkg_lang_fsharp::ExtractedTestKind::Fixture => "Fixture",
      };

      rkg_db::insert_test(
        connection,
        &rkg_db::NewTestRecord {
          file_id,
          name: t.name.clone(),
          qualified_name: t.qualified_name.clone(),
          kind: kind_str.to_string(),
          is_parametrized: t.is_parametrized,
          framework: "fsharp".to_string(),
          start_line: Some(t.start_line as i64),
          end_line: Some(t.end_line as i64),
        },
      )
      .map_err(|e| e.to_string())?;

      let mut source_symbol =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &t.qualified_name)
          .map_err(|e| e.to_string())?;

      let parent_qname_opt = if source_symbol.is_none() {
        t.qualified_name
          .rfind('.')
          .map(|idx| &t.qualified_name[..idx])
      } else {
        None
      };

      if let Some(parent_qname) = parent_qname_opt {
        source_symbol =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, parent_qname)
            .map_err(|e| e.to_string())?;
      }

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      // 1. Fixture parameters ConfiguredBy edges
      for param in &t.parameters {
        rkg_db::insert_edge(
          connection,
          &rkg_db::NewEdgeRecord {
            source_symbol_id: source_symbol.id,
            target_symbol_id: None,
            unresolved_target: Some(param.clone()),
            kind: "ConfiguredBy".to_string(),
            confidence: Some(1.0),
          },
        )
        .map_err(|e| e.to_string())?;
      }

      // 2. Name similarity TestedBy edges
      let is_test_class_or_fn = t.kind == rkg_lang_fsharp::ExtractedTestKind::Class
        || t.kind == rkg_lang_fsharp::ExtractedTestKind::Function;
      let target_candidate = if is_test_class_or_fn {
        get_test_similarity_target(&t.name)
      } else {
        None
      };

      if let Some(target) = target_candidate {
        rkg_db::insert_edge(
          connection,
          &rkg_db::NewEdgeRecord {
            source_symbol_id: source_symbol.id,
            target_symbol_id: None,
            unresolved_target: Some(target),
            kind: "TestedBy".to_string(),
            confidence: Some(0.8),
          },
        )
        .map_err(|e| e.to_string())?;
      }
    }
  }
  Ok(())
}

fn index_fsharp_file_routes(
  connection: &rusqlite::Connection,
  repository_id: i64,
  file_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  if let Ok(extracted_routes) = rkg_lang_fsharp::extract_routes_from_source(content, path) {
    for r in extracted_routes {
      let handler_symbol =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &r.qualified_name)
          .map_err(|e| e.to_string())?;

      let symbol_id = handler_symbol.as_ref().map(|sym| sym.id);

      rkg_db::insert_route(
        connection,
        &rkg_db::NewRouteRecord {
          file_id,
          symbol_id,
          handler_name: r.handler_name.clone(),
          qualified_name: r.qualified_name.clone(),
          method: r.method.clone(),
          path: r.path.clone(),
          response_model: r.response_model.clone(),
          start_line: Some(r.start_line as i64),
          end_line: Some(r.end_line as i64),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_mojo_file_symbols(
  connection: &rusqlite::Connection,
  file_id: i64,
  path: &str,
  content: &str,
) -> Result<Option<i64>, String> {
  let mut module_symbol_id = None;
  if let Ok(extracted_symbols) = rkg_lang_mojo::extract_symbols_from_source(content, path) {
    for sym in extracted_symbols {
      let is_module = sym.kind == rkg_core::SymbolKind::Module;
      let symbol_record = rkg_db::insert_symbol(
        connection,
        &rkg_db::NewSymbolRecord {
          file_id,
          name: sym.name,
          qualified_name: sym.qualified_name,
          kind: format!("{:?}", sym.kind),
          start_line: sym.location.start_line as i64,
          end_line: sym.location.end_line as i64,
          start_column: sym.location.start_column.map(|c| c as i64),
          end_column: sym.location.end_column.map(|c| c as i64),
        },
      )
      .map_err(|e| e.to_string())?;

      if is_module {
        module_symbol_id = Some(symbol_record.id);
      }
    }
  }
  Ok(module_symbol_id)
}

fn index_kotlin_file_symbols(
  connection: &rusqlite::Connection,
  _repository_id: i64,
  file_id: i64,
  path: &str,
  content: &str,
  kotlin_projects: &[rkg_db::KotlinProjectRecord],
) -> Result<Option<i64>, String> {
  let mut module_symbol_id = None;
  let extracted_symbols = rkg_lang_kotlin::extract_symbols_from_source(content, path)
    .map_err(|e| format!("Kotlin parsing error in {path}: {e}"))?;

  let mut provenance_opt = None;
  for proj in kotlin_projects {
    if let Some(dirs) = &proj.generated_source_dirs {
      for dir in dirs.split(',') {
        let dir = dir.trim();
        if !dir.is_empty() && (path == dir || path.starts_with(&format!("{}/", dir))) {
          let prov = if dir.contains("ksp") {
            "ksp"
          } else if dir.contains("kapt") {
            "kapt"
          } else {
            "generated"
          };
          provenance_opt = Some(prov.to_string());
          break;
        }
      }
    }
    if provenance_opt.is_some() {
      break;
    }
  }

  for sym in extracted_symbols {
    let is_module = sym.kind == rkg_core::SymbolKind::Module;
    let symbol_record = rkg_db::insert_symbol(
      connection,
      &rkg_db::NewSymbolRecord {
        file_id,
        name: sym.name,
        qualified_name: sym.qualified_name,
        kind: format!("{:?}", sym.kind),
        start_line: sym.location.start_line as i64,
        end_line: sym.location.end_line as i64,
        start_column: sym.location.start_column.map(|c| c as i64),
        end_column: sym.location.end_column.map(|c| c as i64),
      },
    )
    .map_err(|e| e.to_string())?;

    if let Some(ref prov) = provenance_opt {
      rkg_db::insert_generated_symbol(connection, symbol_record.id, prov)
        .map_err(|e| e.to_string())?;
    }

    if is_module {
      module_symbol_id = Some(symbol_record.id);
    }
  }
  Ok(module_symbol_id)
}

fn index_swift_file_symbols(
  connection: &rusqlite::Connection,
  file_id: i64,
  path: &str,
  content: &str,
) -> Result<Option<i64>, String> {
  let mut module_symbol_id = None;
  let extracted_symbols = rkg_lang_swift::extract_symbols_from_source(content, path)
    .map_err(|e| format!("Swift parsing error in {path}: {e}"))?;
  for sym in extracted_symbols {
    let is_module = sym.kind == rkg_core::SymbolKind::Module;
    let symbol_record = rkg_db::insert_symbol(
      connection,
      &rkg_db::NewSymbolRecord {
        file_id,
        name: sym.name,
        qualified_name: sym.qualified_name,
        kind: format!("{:?}", sym.kind),
        start_line: sym.location.start_line as i64,
        end_line: sym.location.end_line as i64,
        start_column: sym.location.start_column.map(|c| c as i64),
        end_column: sym.location.end_column.map(|c| c as i64),
      },
    )
    .map_err(|e| e.to_string())?;

    if is_module {
      module_symbol_id = Some(symbol_record.id);
    }
  }
  Ok(module_symbol_id)
}

fn index_swift_file_imports(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
  source_symbol_id: i64,
) -> Result<(), String> {
  if let Ok(extracted_imports) = rkg_lang_swift::extract_imports_from_source(content, path) {
    let mut seen = std::collections::HashSet::new();
    for imp in extracted_imports {
      if !seen.insert(imp.target_qualified_name.clone()) {
        continue;
      }

      let target_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &imp.target_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let (target_id, unresolved) = match target_symbol {
        Some(sym) => (Some(sym.id), None),
        None => (None, Some(imp.target_qualified_name.clone())),
      };

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id,
          target_symbol_id: target_id,
          unresolved_target: unresolved,
          kind: "Imports".to_string(),
          confidence: Some(1.0),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

#[allow(clippy::collapsible_if)]
fn index_swift_file_calls(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  let imports = rkg_lang_swift::extract_imports_from_source(content, path).unwrap_or_default();
  let mut import_map = std::collections::HashMap::new();
  for imp in &imports {
    if let Some(alias) = &imp.alias_name {
      import_map.insert(alias.clone(), imp.target_qualified_name.clone());
    } else {
      if let Some(last_dot) = imp.target_qualified_name.rfind('.') {
        let simple_name = &imp.target_qualified_name[last_dot + 1..];
        if simple_name != "*" {
          import_map.insert(simple_name.to_string(), imp.target_qualified_name.clone());
        }
      } else {
        import_map.insert(
          imp.target_qualified_name.clone(),
          imp.target_qualified_name.clone(),
        );
      }
    }
  }

  if let Ok(extracted_calls) = rkg_lang_swift::extract_calls_from_source(content, path) {
    let mut seen_edges = std::collections::HashSet::new();
    for call in extracted_calls {
      let source_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &call.source_symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      let mut target_symbol_id = None;
      let mut unresolved_target = Some(call.target_name.clone());
      let mut confidence = 0.5;

      // 1. Try to resolve via imports (including alias)
      if let Some(qname) = import_map.get(&call.target_name) {
        let mut resolved =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, qname)
            .map_err(|e| e.to_string())?;
        if resolved.is_none() {
          if let Some(last_dot) = qname.rfind('.') {
            let pkg = &qname[..last_dot];
            let name = &qname[last_dot + 1..];
            let colons_qname = format!("{pkg}::{name}");
            resolved =
              rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &colons_qname)
                .map_err(|e| e.to_string())?;
          }
        }
        if let Some(sym) = resolved {
          target_symbol_id = Some(sym.id);
          unresolved_target = None;
          confidence = 1.0;
        }
      }

      // 2. Try to resolve via self-call using class scope
      if target_symbol_id.is_none() && call.is_self_call {
        if let Some(last_dot) = call.source_symbol_qualified_name.rfind('.') {
          let class_prefix = &call.source_symbol_qualified_name[..last_dot];
          let local_qname = format!("{class_prefix}.{}", call.target_name);
          let resolved =
            rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &local_qname)
              .map_err(|e| e.to_string())?;
          if let Some(sym) = resolved {
            target_symbol_id = Some(sym.id);
            unresolved_target = None;
            confidence = 0.9;
          }
        }
      }

      // 3. Try fallback to local class method or sibling method in class
      if target_symbol_id.is_none() {
        if let Some(colons_idx) = call.source_symbol_qualified_name.find("::") {
          let class_scope = &call.source_symbol_qualified_name[..colons_idx + 2];
          let suffix = &call.source_symbol_qualified_name[colons_idx + 2..];
          if let Some(last_dot) = suffix.rfind('.') {
            let class_path = &suffix[..last_dot];
            let sibling_qname = format!("{class_scope}{class_path}.{}", call.target_name);
            let resolved =
              rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &sibling_qname)
                .map_err(|e| e.to_string())?;
            if let Some(sym) = resolved {
              target_symbol_id = Some(sym.id);
              unresolved_target = None;
              confidence = 0.8;
            }
          }
        }
      }

      // 4. Try local module-level lookup
      if target_symbol_id.is_none() {
        let module_name = if let Some(colons_idx) = call.source_symbol_qualified_name.find("::") {
          &call.source_symbol_qualified_name[..colons_idx]
        } else {
          &call.source_symbol_qualified_name
        };

        let local_qname = format!("{module_name}::{}", call.target_name);
        let resolved =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &local_qname)
            .map_err(|e| e.to_string())?;
        if let Some(sym) = resolved {
          target_symbol_id = Some(sym.id);
          unresolved_target = None;
          confidence = 0.7;
        }
      }

      // 5. Try simple name fallback (unique and not a Module)
      if target_symbol_id.is_none() && !import_map.contains_key(&call.target_name) {
        let mut matches =
          rkg_db::lookup_symbols_by_name(connection, repository_id, &call.target_name)
            .map_err(|e| e.to_string())?;
        matches.retain(|m| m.kind != "Module");
        if matches.len() == 1 {
          target_symbol_id = Some(matches[0].id);
          unresolved_target = None;
          confidence = 0.6;
        }
      }

      if !seen_edges.insert((
        source_symbol.id,
        target_symbol_id,
        unresolved_target.clone(),
      )) {
        continue;
      }

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: source_symbol.id,
          target_symbol_id,
          unresolved_target,
          kind: "Calls".to_string(),
          confidence: Some(confidence),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

#[allow(clippy::collapsible_if)]
fn index_swift_file_type_references(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  let imports = rkg_lang_swift::extract_imports_from_source(content, path).unwrap_or_default();
  let mut import_map = std::collections::HashMap::new();
  for imp in &imports {
    if let Some(alias) = &imp.alias_name {
      import_map.insert(alias.clone(), imp.target_qualified_name.clone());
    } else {
      if let Some(last_dot) = imp.target_qualified_name.rfind('.') {
        let simple_name = &imp.target_qualified_name[last_dot + 1..];
        if simple_name != "*" {
          import_map.insert(simple_name.to_string(), imp.target_qualified_name.clone());
        }
      } else {
        import_map.insert(
          imp.target_qualified_name.clone(),
          imp.target_qualified_name.clone(),
        );
      }
    }
  }

  if let Ok(extracted_type_refs) =
    rkg_lang_swift::extract_type_references_from_source(content, path)
  {
    let mut seen_type_edges = std::collections::HashSet::new();
    for type_ref in extracted_type_refs {
      let source_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &type_ref.source_symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      let mut target_symbol_id = None;
      let mut unresolved_target = Some(type_ref.target_type_name.clone());

      // 1. Try to resolve via imports (including alias)
      if let Some(qname) = import_map.get(&type_ref.target_type_name) {
        let mut resolved =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, qname)
            .map_err(|e| e.to_string())?;
        if resolved.is_none() {
          if let Some(last_dot) = qname.rfind('.') {
            let pkg = &qname[..last_dot];
            let name = &qname[last_dot + 1..];
            let colons_qname = format!("{pkg}::{name}");
            resolved =
              rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &colons_qname)
                .map_err(|e| e.to_string())?;
          }
        }
        if let Some(sym) = resolved {
          target_symbol_id = Some(sym.id);
          unresolved_target = None;
        }
      }

      // 2. Try qualified lookup under same package/module
      if target_symbol_id.is_none() {
        let module_name = if let Some(colons_idx) = type_ref.source_symbol_qualified_name.find("::")
        {
          &type_ref.source_symbol_qualified_name[..colons_idx]
        } else {
          &type_ref.source_symbol_qualified_name
        };

        let local_qname = format!("{module_name}::{}", type_ref.target_type_name);
        let resolved =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &local_qname)
            .map_err(|e| e.to_string())?;

        if let Some(sym) = resolved {
          target_symbol_id = Some(sym.id);
          unresolved_target = None;
        }
      }

      // 3. Try fallback to simple name match if unique and not a Module
      if target_symbol_id.is_none() && !import_map.contains_key(&type_ref.target_type_name) {
        let mut matches =
          rkg_db::lookup_symbols_by_name(connection, repository_id, &type_ref.target_type_name)
            .map_err(|e| e.to_string())?;
        matches.retain(|m| m.kind != "Module");
        if matches.len() == 1 {
          target_symbol_id = Some(matches[0].id);
          unresolved_target = None;
        }
      }

      if !seen_type_edges.insert((
        source_symbol.id,
        target_symbol_id,
        unresolved_target.clone(),
      )) {
        continue;
      }

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: source_symbol.id,
          target_symbol_id,
          unresolved_target,
          kind: "ReferencesType".to_string(),
          confidence: Some(1.0),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_swift_file_inheritance_and_interfaces(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  if let Ok(extracted_inheritance) = rkg_lang_swift::extract_inheritance_from_source(content, path)
  {
    let mut seen_edges = std::collections::HashSet::new();
    for inh in extracted_inheritance {
      let subclass_sym =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &inh.subclass_name)
          .map_err(|e| e.to_string())?;

      let Some(subclass_sym) = subclass_sym else {
        continue;
      };

      let mut super_sym = None;
      if let Some(colons_idx) = inh.subclass_name.find("::") {
        let module_prefix = &inh.subclass_name[..colons_idx];
        let local_qname = format!("{module_prefix}::{}", inh.supertype_name);
        super_sym =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &local_qname)
            .map_err(|e| e.to_string())?;
      }

      if super_sym.is_none() {
        let mut matches =
          rkg_db::lookup_symbols_by_name(connection, repository_id, &inh.supertype_name)
            .map_err(|e| e.to_string())?;
        matches.retain(|m| m.kind != "Module");
        if matches.len() == 1 {
          super_sym = Some(matches[0].clone());
        }
      }

      let (target_id, unresolved) = match super_sym {
        Some(sym) => (Some(sym.id), None),
        None => (None, Some(inh.supertype_name.clone())),
      };

      if !seen_edges.insert((subclass_sym.id, target_id, unresolved.clone())) {
        continue;
      }

      let edge_kind = if inh.is_class_extends {
        "Extends".to_string()
      } else {
        "Implements".to_string()
      };

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: subclass_sym.id,
          target_symbol_id: target_id,
          unresolved_target: unresolved,
          kind: edge_kind,
          confidence: Some(1.0),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_swift_file_attributes(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  if let Ok(extracted_attrs) = rkg_lang_swift::extract_attributes_from_source(content, path) {
    let mut seen_edges = std::collections::HashSet::new();
    for attr in extracted_attrs {
      let target_sym = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &attr.target_symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let Some(target_sym) = target_sym else {
        continue;
      };

      if !seen_edges.insert((target_sym.id, attr.attribute_name.clone())) {
        continue;
      }

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: target_sym.id,
          target_symbol_id: None,
          unresolved_target: Some(attr.attribute_name.clone()),
          kind: "ModifiedWith".to_string(),
          confidence: Some(1.0),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_swift_file_tests(
  connection: &rusqlite::Connection,
  repository_id: i64,
  file_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  if let Ok(extracted_tests) = rkg_lang_swift::extract_tests_from_source(content, path) {
    for t in extracted_tests {
      let kind_str = match t.kind {
        rkg_lang_swift::ExtractedTestKind::Class => "Class",
        rkg_lang_swift::ExtractedTestKind::Function => "Function",
        rkg_lang_swift::ExtractedTestKind::Fixture => "Fixture",
      };

      rkg_db::insert_test(
        connection,
        &rkg_db::NewTestRecord {
          file_id,
          name: t.name.clone(),
          qualified_name: t.qualified_name.clone(),
          kind: kind_str.to_string(),
          is_parametrized: t.is_parametrized,
          framework: "XCTest".to_string(),
          start_line: Some(t.start_line as i64),
          end_line: Some(t.end_line as i64),
        },
      )
      .map_err(|e| e.to_string())?;

      let mut source_symbol =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &t.qualified_name)
          .map_err(|e| e.to_string())?;

      let parent_qname_opt = if source_symbol.is_none() {
        t.qualified_name
          .rfind('.')
          .map(|idx| &t.qualified_name[..idx])
      } else {
        None
      };

      if let Some(parent_qname) = parent_qname_opt {
        source_symbol =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, parent_qname)
            .map_err(|e| e.to_string())?;
      }

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      for param in &t.parameters {
        rkg_db::insert_edge(
          connection,
          &rkg_db::NewEdgeRecord {
            source_symbol_id: source_symbol.id,
            target_symbol_id: None,
            unresolved_target: Some(param.clone()),
            kind: "ConfiguredBy".to_string(),
            confidence: Some(1.0),
          },
        )
        .map_err(|e| e.to_string())?;
      }
    }
  }
  Ok(())
}

fn index_swift_file_concurrency(
  connection: &rusqlite::Connection,
  file_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  if let Ok((spawns, channels, selects)) =
    rkg_lang_swift::extract_concurrency_from_source(content, path)
  {
    for s in spawns {
      rkg_db::insert_concurrency_spawn(
        connection,
        &rkg_db::NewConcurrencySpawnRecord {
          file_id,
          source_symbol_qualified_name: s.source_symbol_qualified_name,
          spawn_kind: s.spawn_kind,
          target_name: s.target_name,
          start_line: s.start_line as i64,
          end_line: s.end_line as i64,
        },
      )
      .map_err(|e| e.to_string())?;
    }

    for c in channels {
      rkg_db::insert_concurrency_channel(
        connection,
        &rkg_db::NewConcurrencyChannelRecord {
          file_id,
          source_symbol_qualified_name: c.source_symbol_qualified_name,
          channel_kind: c.channel_kind,
          tx_name: c.tx_name,
          rx_name: c.rx_name,
          start_line: c.start_line as i64,
          end_line: c.end_line as i64,
        },
      )
      .map_err(|e| e.to_string())?;
    }

    for s in selects {
      rkg_db::insert_concurrency_select(
        connection,
        &rkg_db::NewConcurrencySelectRecord {
          file_id,
          source_symbol_qualified_name: s.source_symbol_qualified_name,
          start_line: s.start_line as i64,
          end_line: s.end_line as i64,
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_kotlin_file_imports(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
  source_symbol_id: i64,
) -> Result<(), String> {
  if let Ok(extracted_imports) = rkg_lang_kotlin::extract_imports_from_source(content, path) {
    let mut seen = std::collections::HashSet::new();
    for imp in extracted_imports {
      if !seen.insert(imp.target_qualified_name.clone()) {
        continue;
      }

      let target_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &imp.target_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let (target_id, unresolved) = match target_symbol {
        Some(sym) => (Some(sym.id), None),
        None => (None, Some(imp.target_qualified_name.clone())),
      };

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id,
          target_symbol_id: target_id,
          unresolved_target: unresolved,
          kind: "Imports".to_string(),
          confidence: Some(1.0),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

#[allow(clippy::collapsible_if)]
fn index_kotlin_file_calls(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  let imports = rkg_lang_kotlin::extract_imports_from_source(content, path).unwrap_or_default();
  let mut import_map = std::collections::HashMap::new();
  for imp in &imports {
    if let Some(alias) = &imp.alias_name {
      import_map.insert(alias.clone(), imp.target_qualified_name.clone());
    } else {
      if let Some(last_dot) = imp.target_qualified_name.rfind('.') {
        let simple_name = &imp.target_qualified_name[last_dot + 1..];
        if simple_name != "*" {
          import_map.insert(simple_name.to_string(), imp.target_qualified_name.clone());
        }
      } else {
        import_map.insert(
          imp.target_qualified_name.clone(),
          imp.target_qualified_name.clone(),
        );
      }
    }
  }

  if let Ok(extracted_calls) = rkg_lang_kotlin::extract_calls_from_source(content, path) {
    let mut seen_edges = std::collections::HashSet::new();
    for call in extracted_calls {
      let source_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &call.source_symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      let mut target_symbol_id = None;
      let mut unresolved_target = Some(call.target_name.clone());
      let mut confidence = 0.5;

      // 1. Try to resolve via imports (including alias)
      if let Some(qname) = import_map.get(&call.target_name) {
        let mut resolved =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, qname)
            .map_err(|e| e.to_string())?;
        if resolved.is_none() {
          if let Some(last_dot) = qname.rfind('.') {
            let pkg = &qname[..last_dot];
            let name = &qname[last_dot + 1..];
            let colons_qname = format!("{pkg}::{name}");
            resolved =
              rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &colons_qname)
                .map_err(|e| e.to_string())?;
          }
        }
        if let Some(sym) = resolved {
          target_symbol_id = Some(sym.id);
          unresolved_target = None;
          confidence = 1.0;
        }
      }

      // 2. Try to resolve via self-call using class scope
      if target_symbol_id.is_none() && call.is_self_call {
        if let Some(colons_idx) = call.source_symbol_qualified_name.find("::") {
          let class_part = &call.source_symbol_qualified_name[colons_idx + 2..];
          if let Some(last_dot_idx) = class_part.rfind('.') {
            let class_name = &class_part[..last_dot_idx];
            let module_name = &call.source_symbol_qualified_name[..colons_idx];
            let method_name = &call.target_name;
            let local_method_qname = format!("{module_name}::{class_name}.{method_name}");

            let resolved = rkg_db::lookup_symbol_by_qualified_name(
              connection,
              repository_id,
              &local_method_qname,
            )
            .map_err(|e| e.to_string())?;

            if let Some(sym) = resolved {
              target_symbol_id = Some(sym.id);
              unresolved_target = None;
              confidence = 1.0;
            } else {
              unresolved_target = Some(format!("{class_name}.{method_name}"));
              confidence = 0.8;
            }
          }
        }
      }

      // 3. Try to resolve within local package/module scope
      if target_symbol_id.is_none() {
        let module_name = if let Some(colons_idx) = call.source_symbol_qualified_name.find("::") {
          &call.source_symbol_qualified_name[..colons_idx]
        } else {
          &call.source_symbol_qualified_name
        };

        let local_qname = format!("{module_name}::{}", call.target_name);
        let resolved =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &local_qname)
            .map_err(|e| e.to_string())?;

        if let Some(sym) = resolved {
          target_symbol_id = Some(sym.id);
          unresolved_target = None;
          confidence = 1.0;
        }
      }

      // 4. Try fallback to simple name match if unique and not a Module
      if target_symbol_id.is_none() {
        let mut matches =
          rkg_db::lookup_symbols_by_name(connection, repository_id, &call.target_name)
            .map_err(|e| e.to_string())?;
        matches.retain(|m| m.kind != "Module");
        if matches.len() == 1 {
          target_symbol_id = Some(matches[0].id);
          unresolved_target = None;
          confidence = 0.8;
        }
      }

      if !seen_edges.insert((
        source_symbol.id,
        target_symbol_id,
        unresolved_target.clone(),
      )) {
        continue;
      }

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: source_symbol.id,
          target_symbol_id,
          unresolved_target,
          kind: "Calls".to_string(),
          confidence: Some(confidence),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

#[allow(clippy::collapsible_if)]
fn index_kotlin_file_type_references(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  let imports = rkg_lang_kotlin::extract_imports_from_source(content, path).unwrap_or_default();
  let mut import_map = std::collections::HashMap::new();
  for imp in &imports {
    if let Some(alias) = &imp.alias_name {
      import_map.insert(alias.clone(), imp.target_qualified_name.clone());
    } else {
      if let Some(last_dot) = imp.target_qualified_name.rfind('.') {
        let simple_name = &imp.target_qualified_name[last_dot + 1..];
        if simple_name != "*" {
          import_map.insert(simple_name.to_string(), imp.target_qualified_name.clone());
        }
      } else {
        import_map.insert(
          imp.target_qualified_name.clone(),
          imp.target_qualified_name.clone(),
        );
      }
    }
  }

  if let Ok(extracted_type_refs) =
    rkg_lang_kotlin::extract_type_references_from_source(content, path)
  {
    let mut seen_type_edges = std::collections::HashSet::new();
    for type_ref in extracted_type_refs {
      let mut source_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &type_ref.source_symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      if source_symbol.is_none() {
        let mut current_name = type_ref.source_symbol_qualified_name.clone();
        while let Some(last_dot_idx) = current_name.rfind('.') {
          current_name = current_name[..last_dot_idx].to_string();
          if let Some(sym) =
            rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &current_name)
              .map_err(|e| e.to_string())?
          {
            source_symbol = Some(sym);
            break;
          }
        }
      }

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      let mut target_symbol_id = None;
      let mut unresolved_target = Some(type_ref.target_type_name.clone());

      // 1. Try to resolve via imports (including alias)
      if let Some(qname) = import_map.get(&type_ref.target_type_name) {
        let mut resolved =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, qname)
            .map_err(|e| e.to_string())?;
        if resolved.is_none() {
          if let Some(last_dot) = qname.rfind('.') {
            let pkg = &qname[..last_dot];
            let name = &qname[last_dot + 1..];
            let colons_qname = format!("{pkg}::{name}");
            resolved =
              rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &colons_qname)
                .map_err(|e| e.to_string())?;
          }
        }
        if let Some(sym) = resolved {
          target_symbol_id = Some(sym.id);
          unresolved_target = None;
        }
      }

      // 2. Try qualified lookup under same package/module
      if target_symbol_id.is_none() {
        let module_name = if let Some(colons_idx) = type_ref.source_symbol_qualified_name.find("::")
        {
          &type_ref.source_symbol_qualified_name[..colons_idx]
        } else {
          &type_ref.source_symbol_qualified_name
        };

        let local_qname = format!("{module_name}::{}", type_ref.target_type_name);
        let resolved =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &local_qname)
            .map_err(|e| e.to_string())?;

        if let Some(sym) = resolved {
          target_symbol_id = Some(sym.id);
          unresolved_target = None;
        }
      }

      // 3. Try fallback to simple name match if unique and not a Module
      if target_symbol_id.is_none() {
        let mut matches =
          rkg_db::lookup_symbols_by_name(connection, repository_id, &type_ref.target_type_name)
            .map_err(|e| e.to_string())?;
        matches.retain(|m| m.kind != "Module");
        if matches.len() == 1 {
          target_symbol_id = Some(matches[0].id);
          unresolved_target = None;
        }
      }

      if !seen_type_edges.insert((
        source_symbol.id,
        target_symbol_id,
        unresolved_target.clone(),
      )) {
        continue;
      }

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: source_symbol.id,
          target_symbol_id,
          unresolved_target,
          kind: "ReferencesType".to_string(),
          confidence: Some(1.0),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

#[allow(clippy::collapsible_if)]
fn index_kotlin_file_inheritance_and_interfaces(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  let imports = rkg_lang_kotlin::extract_imports_from_source(content, path).unwrap_or_default();
  let mut import_map = std::collections::HashMap::new();
  for imp in &imports {
    if let Some(alias) = &imp.alias_name {
      import_map.insert(alias.clone(), imp.target_qualified_name.clone());
    } else {
      if let Some(last_dot) = imp.target_qualified_name.rfind('.') {
        let simple_name = &imp.target_qualified_name[last_dot + 1..];
        if simple_name != "*" {
          import_map.insert(simple_name.to_string(), imp.target_qualified_name.clone());
        }
      } else {
        import_map.insert(
          imp.target_qualified_name.clone(),
          imp.target_qualified_name.clone(),
        );
      }
    }
  }

  if let Ok(extracted_inheritance) = rkg_lang_kotlin::extract_inheritance_from_source(content, path)
  {
    let module_name = if path.ends_with(".kt") {
      let normalized = path.replace('\\', "/");
      normalized
        .strip_suffix(".kt")
        .unwrap_or(&normalized)
        .replace('/', ".")
    } else {
      path.replace(['/', '\\'], ".")
    };

    // Find the package name if possible or fall back (hoisted outside loop)
    let parent_qname =
      if let Ok(symbols) = rkg_lang_kotlin::extract_symbols_from_source(content, path) {
        if let Some(mod_sym) = symbols
          .iter()
          .find(|s| s.kind == rkg_core::SymbolKind::Module)
        {
          mod_sym.qualified_name.clone()
        } else {
          module_name.clone()
        }
      } else {
        module_name.clone()
      };

    let mut seen_edges = std::collections::HashSet::new();
    for inh in extracted_inheritance {
      let subclass_qname = format!("{parent_qname}::{}", inh.subclass_name);
      let subclass_sym =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &subclass_qname)
          .map_err(|e| e.to_string())?;

      let Some(subclass_sym) = subclass_sym else {
        continue;
      };

      let mut super_sym = None;

      // 1. Try to resolve via imports (including alias)
      if let Some(qname) = import_map.get(&inh.supertype_name) {
        let mut resolved =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, qname)
            .map_err(|e| e.to_string())?;
        if resolved.is_none() {
          if let Some(last_dot) = qname.rfind('.') {
            let pkg = &qname[..last_dot];
            let name = &qname[last_dot + 1..];
            let colons_qname = format!("{pkg}::{name}");
            resolved =
              rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &colons_qname)
                .map_err(|e| e.to_string())?;
          }
        }
        if let Some(sym) = resolved {
          super_sym = Some(sym);
        }
      }

      // 2. Try to resolve via same package/module
      if super_sym.is_none() {
        let superclass_qname = format!("{parent_qname}::{}", inh.supertype_name);
        super_sym =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &superclass_qname)
            .map_err(|e| e.to_string())?;
      }

      // 3. Try fallback to simple name match if unique and not a Module
      if super_sym.is_none() {
        let mut matches =
          rkg_db::lookup_symbols_by_name(connection, repository_id, &inh.supertype_name)
            .map_err(|e| e.to_string())?;
        matches.retain(|m| m.kind != "Module");
        if matches.len() == 1 {
          super_sym = Some(matches[0].clone());
        }
      }

      let (target_id, unresolved) = match super_sym {
        Some(sym) => (Some(sym.id), None),
        None => (None, Some(inh.supertype_name.clone())),
      };

      if !seen_edges.insert((subclass_sym.id, target_id, unresolved.clone())) {
        continue;
      }

      let edge_kind = if inh.is_class_extends {
        "Extends".to_string()
      } else {
        "Implements".to_string()
      };

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: subclass_sym.id,
          target_symbol_id: target_id,
          unresolved_target: unresolved,
          kind: edge_kind,
          confidence: Some(1.0),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_kotlin_file_annotations(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  if let Ok(extracted_ann) = rkg_lang_kotlin::extract_annotations_from_source(content, path) {
    let mut seen_edges = std::collections::HashSet::new();
    for ann in extracted_ann {
      let source_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &ann.target_symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      let mut target_symbol_id = None;
      let mut unresolved_target = Some(ann.annotation_name.clone());

      let target_symbol =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &ann.annotation_name)
          .map_err(|e| e.to_string())?;

      if let Some(sym) = target_symbol {
        target_symbol_id = Some(sym.id);
        unresolved_target = None;
      } else {
        let matches =
          rkg_db::lookup_symbols_by_name(connection, repository_id, &ann.annotation_name)
            .map_err(|e| e.to_string())?;
        if matches.len() == 1 {
          target_symbol_id = Some(matches[0].id);
          unresolved_target = None;
        }
      }

      if !seen_edges.insert((
        source_symbol.id,
        target_symbol_id,
        unresolved_target.clone(),
      )) {
        continue;
      }

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: source_symbol.id,
          target_symbol_id,
          unresolved_target,
          kind: "ModifiedWith".to_string(),
          confidence: Some(1.0),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_kotlin_file_routes(
  connection: &rusqlite::Connection,
  repository_id: i64,
  file_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  if let Ok(extracted_routes) = rkg_lang_kotlin::extract_ktor_routes_from_source(content, path) {
    for r in extracted_routes {
      let mut symbol_id = None;
      let matched_symbol =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &r.qualified_name)
          .map_err(|e| e.to_string())?;
      if let Some(sym) = matched_symbol {
        symbol_id = Some(sym.id);
      }

      rkg_db::insert_route(
        connection,
        &rkg_db::NewRouteRecord {
          file_id,
          symbol_id,
          handler_name: r.handler_name,
          qualified_name: r.qualified_name,
          method: r.method,
          path: r.path,
          response_model: None,
          start_line: Some(r.start_line as i64),
          end_line: Some(r.end_line as i64),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_kotlin_file_concurrency(
  connection: &rusqlite::Connection,
  repository_id: i64,
  file_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  if let Ok((edges, spawns, channels, selects)) =
    rkg_lang_kotlin::extract_concurrency_from_source(content, path)
  {
    for edge in edges {
      let source_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &edge.source_symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      let mut target_symbol_id = None;
      let mut unresolved_target = Some(edge.target_name.clone());
      let mut confidence = 0.8;

      if edge.kind == "Spawns" {
        let mut matches =
          rkg_db::lookup_symbols_by_name(connection, repository_id, &edge.target_name)
            .map_err(|e| e.to_string())?;
        matches.retain(|m| m.kind != "Module");
        if matches.len() == 1 {
          target_symbol_id = Some(matches[0].id);
          unresolved_target = None;
          confidence = 1.0;
        } else if !matches.is_empty() {
          confidence = 0.7;
        }
      } else if edge.kind == "SendsTo" {
        let resolved =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &edge.target_name)
            .map_err(|e| e.to_string())?;
        if let Some(sym) = resolved {
          target_symbol_id = Some(sym.id);
          unresolved_target = None;
          confidence = 1.0;
        }
      }

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: source_symbol.id,
          target_symbol_id,
          unresolved_target,
          kind: edge.kind.clone(),
          confidence: Some(confidence),
        },
      )
      .map_err(|e| e.to_string())?;
    }

    for spawn in spawns {
      rkg_db::insert_concurrency_spawn(
        connection,
        &rkg_db::NewConcurrencySpawnRecord {
          file_id,
          source_symbol_qualified_name: spawn.source_symbol_qualified_name,
          spawn_kind: spawn.spawn_kind,
          target_name: spawn.target_name,
          start_line: spawn.start_line as i64,
          end_line: spawn.end_line as i64,
        },
      )
      .map_err(|e| e.to_string())?;
    }

    for channel in channels {
      rkg_db::insert_concurrency_channel(
        connection,
        &rkg_db::NewConcurrencyChannelRecord {
          file_id,
          source_symbol_qualified_name: channel.source_symbol_qualified_name,
          channel_kind: channel.channel_kind,
          tx_name: channel.tx_name,
          rx_name: channel.rx_name,
          start_line: channel.start_line as i64,
          end_line: channel.end_line as i64,
        },
      )
      .map_err(|e| e.to_string())?;
    }

    for select in selects {
      rkg_db::insert_concurrency_select(
        connection,
        &rkg_db::NewConcurrencySelectRecord {
          file_id,
          source_symbol_qualified_name: select.source_symbol_qualified_name,
          start_line: select.start_line as i64,
          end_line: select.end_line as i64,
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }

  Ok(())
}

fn index_mojo_file_imports(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
  source_symbol_id: i64,
) -> Result<(), String> {
  if let Ok(extracted_imports) = rkg_lang_mojo::extract_imports_from_source(content, path) {
    let mut seen = std::collections::HashSet::new();
    for imp in extracted_imports {
      if !seen.insert(imp.target_qualified_name.clone()) {
        continue;
      }

      let target_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &imp.target_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let (target_id, unresolved) = match target_symbol {
        Some(sym) => (Some(sym.id), None),
        None => (None, Some(imp.target_qualified_name.clone())),
      };

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id,
          target_symbol_id: target_id,
          unresolved_target: unresolved,
          kind: "Imports".to_string(),
          confidence: Some(1.0),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_mojo_file_calls(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  if let Ok(extracted_calls) = rkg_lang_mojo::extract_calls_from_source(content, path) {
    let mut seen_edges = std::collections::HashSet::new();
    for call in extracted_calls {
      let source_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &call.source_symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      let mut target_symbol_id = None;
      let mut unresolved_target = Some(call.target_name.clone());
      let mut confidence = 0.5;

      if call.is_self_call {
        if let Some(colons_idx) = call.source_symbol_qualified_name.find("::") {
          let class_part = &call.source_symbol_qualified_name[colons_idx + 2..];
          if let Some(dot_idx) = class_part.find('.') {
            let class_name = &class_part[..dot_idx];
            let module_name = &call.source_symbol_qualified_name[..colons_idx];
            let method_name = call.method_name.as_deref().unwrap_or("");
            let local_method_qname = format!("{module_name}::{class_name}.{method_name}");

            let resolved = rkg_db::lookup_symbol_by_qualified_name(
              connection,
              repository_id,
              &local_method_qname,
            )
            .map_err(|e| e.to_string())?;

            if let Some(sym) = resolved {
              target_symbol_id = Some(sym.id);
              unresolved_target = None;
              confidence = 1.0;
            } else {
              unresolved_target = Some(format!("{class_name}.{method_name}"));
              confidence = 0.8;
            }
          }
        }
      } else if call.method_name.is_none() {
        let module_name = if let Some(colons_idx) = call.source_symbol_qualified_name.find("::") {
          &call.source_symbol_qualified_name[..colons_idx]
        } else {
          &call.source_symbol_qualified_name
        };
        let local_func_qname = format!("{module_name}::{}", call.target_name);

        let resolved =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &local_func_qname)
            .map_err(|e| e.to_string())?;

        if let Some(sym) = resolved {
          target_symbol_id = Some(sym.id);
          unresolved_target = None;
          confidence = 1.0;
        }

        if target_symbol_id.is_none() {
          confidence = 0.7;
        }
      }

      if !seen_edges.insert((
        source_symbol.id,
        target_symbol_id,
        unresolved_target.clone(),
      )) {
        continue;
      }

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: source_symbol.id,
          target_symbol_id,
          unresolved_target,
          kind: "Calls".to_string(),
          confidence: Some(confidence),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_mojo_file_type_references(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  if let Ok(extracted_type_refs) = rkg_lang_mojo::extract_type_references_from_source(content, path)
  {
    let mut seen_type_edges = std::collections::HashSet::new();
    for type_ref in extracted_type_refs {
      let source_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &type_ref.source_symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      let mut target_symbol_id = None;
      let mut unresolved_target = Some(type_ref.target_type_name.clone());

      let target_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &type_ref.target_type_name,
      )
      .map_err(|e| e.to_string())?;

      if let Some(sym) = target_symbol {
        target_symbol_id = Some(sym.id);
        unresolved_target = None;
      }

      if !seen_type_edges.insert((
        source_symbol.id,
        target_symbol_id,
        unresolved_target.clone(),
      )) {
        continue;
      }

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: source_symbol.id,
          target_symbol_id,
          unresolved_target,
          kind: "ReferencesType".to_string(),
          confidence: Some(1.0),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_mojo_file_tests(
  connection: &rusqlite::Connection,
  repository_id: i64,
  file_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  if let Ok(extracted_tests) = rkg_lang_mojo::extract_tests_from_source(content, path) {
    for t in extracted_tests {
      let kind_str = match t.kind {
        rkg_lang_mojo::ExtractedTestKind::Class => "Class",
        rkg_lang_mojo::ExtractedTestKind::Function => "Function",
        rkg_lang_mojo::ExtractedTestKind::Fixture => "Fixture",
      };

      rkg_db::insert_test(
        connection,
        &rkg_db::NewTestRecord {
          file_id,
          name: t.name.clone(),
          qualified_name: t.qualified_name.clone(),
          kind: kind_str.to_string(),
          is_parametrized: t.is_parametrized,
          framework: "mojo".to_string(),
          start_line: Some(t.start_line as i64),
          end_line: Some(t.end_line as i64),
        },
      )
      .map_err(|e| e.to_string())?;

      let mut source_symbol =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &t.qualified_name)
          .map_err(|e| e.to_string())?;

      let parent_qname_opt = if source_symbol.is_none() {
        t.qualified_name
          .rfind('.')
          .map(|idx| &t.qualified_name[..idx])
      } else {
        None
      };

      if let Some(parent_qname) = parent_qname_opt {
        source_symbol =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, parent_qname)
            .map_err(|e| e.to_string())?;
      }

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      for param in &t.parameters {
        rkg_db::insert_edge(
          connection,
          &rkg_db::NewEdgeRecord {
            source_symbol_id: source_symbol.id,
            target_symbol_id: None,
            unresolved_target: Some(param.clone()),
            kind: "ConfiguredBy".to_string(),
            confidence: Some(1.0),
          },
        )
        .map_err(|e| e.to_string())?;
      }

      let is_test_class_or_fn = t.kind == rkg_lang_mojo::ExtractedTestKind::Class
        || t.kind == rkg_lang_mojo::ExtractedTestKind::Function;
      let target_candidate = if is_test_class_or_fn {
        get_test_similarity_target(&t.name)
      } else {
        None
      };

      if let Some(target) = target_candidate {
        rkg_db::insert_edge(
          connection,
          &rkg_db::NewEdgeRecord {
            source_symbol_id: source_symbol.id,
            target_symbol_id: None,
            unresolved_target: Some(target),
            kind: "TestedBy".to_string(),
            confidence: Some(0.8),
          },
        )
        .map_err(|e| e.to_string())?;
      }
    }
  }
  Ok(())
}

fn index_rust_file_symbols(
  connection: &rusqlite::Connection,
  file_id: i64,
  path: &str,
  content: &str,
  active_features: &[String],
) -> Result<Option<i64>, String> {
  let mut module_symbol_id = None;
  if let Ok(extracted_symbols) =
    rkg_lang_rust::extract_symbols_from_source(content, path, active_features)
  {
    for sym in extracted_symbols {
      let is_module = sym.kind == rkg_core::SymbolKind::Module;
      let symbol_record = rkg_db::insert_symbol(
        connection,
        &rkg_db::NewSymbolRecord {
          file_id,
          name: sym.name,
          qualified_name: sym.qualified_name,
          kind: format!("{:?}", sym.kind),
          start_line: sym.location.start_line as i64,
          end_line: sym.location.end_line as i64,
          start_column: sym.location.start_column.map(|c| c as i64),
          end_column: sym.location.end_column.map(|c| c as i64),
        },
      )
      .map_err(|e| e.to_string())?;

      if is_module {
        module_symbol_id = Some(symbol_record.id);
      }
    }
  }
  Ok(module_symbol_id)
}

fn index_rust_file_imports(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
  source_symbol_id: i64,
  active_features: &[String],
) -> Result<(), String> {
  if let Ok(extracted_imports) =
    rkg_lang_rust::extract_imports_from_source(content, path, active_features)
  {
    let mut seen = std::collections::HashSet::new();
    for imp in extracted_imports {
      if !seen.insert(imp.target_qualified_name.clone()) {
        continue;
      }
      let mut target_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &imp.target_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      if target_symbol.is_none() {
        target_symbol = if let Some(last_colons_idx) = imp.target_qualified_name.rfind("::") {
          let parent = &imp.target_qualified_name[..last_colons_idx];
          let child = &imp.target_qualified_name[last_colons_idx + 2..];
          let qname_colons = format!("{parent}::{child}");
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &qname_colons)
            .map_err(|e| e.to_string())?
        } else {
          None
        };
      }

      if target_symbol.is_none() {
        let qname_src = format!("src::{}", imp.target_qualified_name);
        target_symbol =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &qname_src)
            .map_err(|e| e.to_string())?;
      }

      if target_symbol.is_none() && imp.target_qualified_name.starts_with("crate::") {
        let qname_crate = format!("src::{}", &imp.target_qualified_name[7..]);
        target_symbol =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &qname_crate)
            .map_err(|e| e.to_string())?;
      }

      let (target_id, unresolved) = match target_symbol {
        Some(sym) => (Some(sym.id), None),
        None => (None, Some(imp.target_qualified_name.clone())),
      };

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id,
          target_symbol_id: target_id,
          unresolved_target: unresolved,
          kind: "Imports".to_string(),
          confidence: Some(1.0),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_rust_file_calls(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
  active_features: &[String],
) -> Result<(), String> {
  if let Ok(extracted_calls) =
    rkg_lang_rust::extract_calls_from_source(content, path, active_features)
  {
    let mut seen_edges = std::collections::HashSet::new();
    for call in extracted_calls {
      let source_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &call.source_symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      let mut target_symbol_id = None;
      let mut unresolved_target = Some(call.target_name.clone());
      let mut confidence = 0.5;

      if call.is_self_call {
        if let Some(colons_idx) = call.source_symbol_qualified_name.find("::") {
          let struct_part = &call.source_symbol_qualified_name[colons_idx + 2..];
          if let Some(inner_colons) = struct_part.find("::") {
            let struct_name = &struct_part[..inner_colons];
            let module_name = &call.source_symbol_qualified_name[..colons_idx];
            let method_name = call.method_name.as_deref().unwrap_or("");
            let local_method_qname = format!("{module_name}::{struct_name}::{method_name}");

            let resolved = rkg_db::lookup_symbol_by_qualified_name(
              connection,
              repository_id,
              &local_method_qname,
            )
            .map_err(|e| e.to_string())?;

            if let Some(sym) = resolved {
              target_symbol_id = Some(sym.id);
              unresolved_target = None;
              confidence = 1.0;
            } else {
              unresolved_target = Some(format!("{struct_name}::{method_name}"));
              confidence = 0.8;
            }
          }
        }
      } else if call.method_name.is_none() {
        let module_name = if let Some(colons_idx) = call.source_symbol_qualified_name.find("::") {
          &call.source_symbol_qualified_name[..colons_idx]
        } else {
          &call.source_symbol_qualified_name
        };
        let local_func_qname = format!("{module_name}::{}", call.target_name);

        let resolved =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &local_func_qname)
            .map_err(|e| e.to_string())?;

        if let Some(sym) = resolved {
          target_symbol_id = Some(sym.id);
          unresolved_target = None;
          confidence = 1.0;
        }

        if target_symbol_id.is_none() {
          confidence = 0.7;
        }
      }

      if !seen_edges.insert((
        source_symbol.id,
        target_symbol_id,
        unresolved_target.clone(),
      )) {
        continue;
      }

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: source_symbol.id,
          target_symbol_id,
          unresolved_target,
          kind: "Calls".to_string(),
          confidence: Some(confidence),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_rust_file_type_references(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
  active_features: &[String],
) -> Result<(), String> {
  if let Ok(extracted_type_refs) =
    rkg_lang_rust::extract_type_references_from_source(content, path, active_features)
  {
    let mut seen_type_edges = std::collections::HashSet::new();
    for type_ref in extracted_type_refs {
      let source_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &type_ref.source_symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      let mut target_symbol_id = None;
      let mut unresolved_target = Some(type_ref.target_type_name.clone());

      let mut target_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &type_ref.target_type_name,
      )
      .map_err(|e| e.to_string())?;

      if target_symbol.is_none() {
        target_symbol = if let Some(last_colons_idx) = type_ref.target_type_name.rfind("::") {
          let parent = &type_ref.target_type_name[..last_colons_idx];
          let child = &type_ref.target_type_name[last_colons_idx + 2..];
          let qname_colons = format!("{parent}::{child}");
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &qname_colons)
            .map_err(|e| e.to_string())?
        } else {
          None
        };
      }

      if let Some(sym) = target_symbol {
        target_symbol_id = Some(sym.id);
        unresolved_target = None;
      }

      if !seen_type_edges.insert((
        source_symbol.id,
        target_symbol_id,
        unresolved_target.clone(),
      )) {
        continue;
      }

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: source_symbol.id,
          target_symbol_id,
          unresolved_target,
          kind: "ReferencesType".to_string(),
          confidence: Some(1.0),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_rust_file_implementations(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
  active_features: &[String],
) -> Result<(), String> {
  if let Ok(extracted_impls) =
    rkg_lang_rust::extract_implementations_from_source(content, path, active_features)
  {
    let mut seen_edges = std::collections::HashSet::new();
    let module_name = if path.ends_with("/mod.rs") {
      let normalized = path.replace('\\', "/");
      normalized[..normalized.len() - 7].replace('/', "::")
    } else if path == "mod.rs" {
      "".to_string()
    } else {
      let normalized = path.replace('\\', "/");
      normalized
        .strip_suffix(".rs")
        .unwrap_or(&normalized)
        .replace('/', "::")
    };

    for imp in extracted_impls {
      let Some(trait_name) = imp.trait_name else {
        continue;
      };

      let struct_qname = if module_name.is_empty() {
        imp.struct_name.clone()
      } else {
        format!("{module_name}::{}", imp.struct_name)
      };

      let struct_sym =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &struct_qname)
          .map_err(|e| e.to_string())?;

      let Some(struct_sym) = struct_sym else {
        continue;
      };

      let trait_qname = if module_name.is_empty() {
        trait_name.clone()
      } else {
        format!("{module_name}::{trait_name}")
      };

      let mut trait_sym =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &trait_qname)
          .map_err(|e| e.to_string())?;

      if trait_sym.is_none() {
        let matches = rkg_db::lookup_symbols_by_name(connection, repository_id, &trait_name)
          .map_err(|e| e.to_string())?;
        if matches.len() == 1 {
          trait_sym = Some(matches[0].clone());
        }
      }

      let (target_id, unresolved) = match trait_sym {
        Some(sym) => (Some(sym.id), None),
        None => (None, Some(trait_name.clone())),
      };

      if !seen_edges.insert((struct_sym.id, target_id, unresolved.clone())) {
        continue;
      }

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: struct_sym.id,
          target_symbol_id: target_id,
          unresolved_target: unresolved,
          kind: "Implements".to_string(),
          confidence: Some(1.0),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_rust_file_tests(
  connection: &rusqlite::Connection,
  repository_id: i64,
  file_id: i64,
  path: &str,
  content: &str,
  active_features: &[String],
) -> Result<(), String> {
  if let Ok(extracted_tests) =
    rkg_lang_rust::extract_tests_from_source(content, path, active_features)
  {
    for t in extracted_tests {
      let kind_str = match t.kind {
        rkg_lang_rust::ExtractedTestKind::Class => "Class",
        rkg_lang_rust::ExtractedTestKind::Function => "Function",
        rkg_lang_rust::ExtractedTestKind::Fixture => "Fixture",
      };

      rkg_db::insert_test(
        connection,
        &rkg_db::NewTestRecord {
          file_id,
          name: t.name.clone(),
          qualified_name: t.qualified_name.clone(),
          kind: kind_str.to_string(),
          is_parametrized: t.is_parametrized,
          framework: "cargo".to_string(),
          start_line: Some(t.start_line as i64),
          end_line: Some(t.end_line as i64),
        },
      )
      .map_err(|e| e.to_string())?;

      let source_symbol =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &t.qualified_name)
          .map_err(|e| e.to_string())?;

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      // 1. Fixture parameters ConfiguredBy edges (for rstest)
      for param in &t.parameters {
        rkg_db::insert_edge(
          connection,
          &rkg_db::NewEdgeRecord {
            source_symbol_id: source_symbol.id,
            target_symbol_id: None,
            unresolved_target: Some(param.clone()),
            kind: "ConfiguredBy".to_string(),
            confidence: Some(1.0),
          },
        )
        .map_err(|e| e.to_string())?;
      }

      // 2. Name similarity TestedBy edges
      let is_test_class_or_fn = t.kind == rkg_lang_rust::ExtractedTestKind::Class
        || t.kind == rkg_lang_rust::ExtractedTestKind::Function;
      let target_candidate = if is_test_class_or_fn {
        get_test_similarity_target(&t.name)
      } else {
        None
      };

      if let Some(target) = target_candidate {
        rkg_db::insert_edge(
          connection,
          &rkg_db::NewEdgeRecord {
            source_symbol_id: source_symbol.id,
            target_symbol_id: None,
            unresolved_target: Some(target),
            kind: "TestedBy".to_string(),
            confidence: Some(0.8),
          },
        )
        .map_err(|e| e.to_string())?;
      }
    }
  }
  Ok(())
}

fn index_rust_file_concurrency(
  connection: &rusqlite::Connection,
  repository_id: i64,
  file_id: i64,
  path: &str,
  content: &str,
  active_features: &[String],
) -> Result<(), String> {
  if let Ok(extracted_concurrency) =
    rkg_lang_rust::extract_concurrency_from_source(content, path, active_features)
  {
    for edge in extracted_concurrency {
      let source_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &edge.source_symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      let mut target_symbol_id = None;
      let mut unresolved_target = Some(edge.target_name.clone());
      let mut confidence = 0.8;

      if edge.kind == "Spawns" {
        let matches = rkg_db::lookup_symbols_by_name(connection, repository_id, &edge.target_name)
          .map_err(|e| e.to_string())?;
        if matches.len() == 1 {
          target_symbol_id = Some(matches[0].id);
          unresolved_target = None;
          confidence = 1.0;
        } else if !matches.is_empty() {
          confidence = 0.7;
        }
      } else if edge.kind == "SendsTo" {
        let resolved =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &edge.target_name)
            .map_err(|e| e.to_string())?;
        if let Some(sym) = resolved {
          target_symbol_id = Some(sym.id);
          unresolved_target = None;
          confidence = 1.0;
        }
      }

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: source_symbol.id,
          target_symbol_id,
          unresolved_target,
          kind: edge.kind.clone(),
          confidence: Some(confidence),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }

  if let Ok(extracted) =
    rkg_lang_rust::extract_concurrency_topology_from_source(content, path, active_features)
  {
    for s in extracted.spawns {
      rkg_db::insert_concurrency_spawn(
        connection,
        &rkg_db::NewConcurrencySpawnRecord {
          file_id,
          source_symbol_qualified_name: s.source_symbol_qualified_name,
          spawn_kind: s.spawn_kind,
          target_name: s.target_name,
          start_line: s.start_line as i64,
          end_line: s.end_line as i64,
        },
      )
      .map_err(|e| e.to_string())?;
    }

    for c in extracted.channels {
      rkg_db::insert_concurrency_channel(
        connection,
        &rkg_db::NewConcurrencyChannelRecord {
          file_id,
          source_symbol_qualified_name: c.source_symbol_qualified_name,
          channel_kind: c.channel_kind,
          tx_name: c.tx_name,
          rx_name: c.rx_name,
          start_line: c.start_line as i64,
          end_line: c.end_line as i64,
        },
      )
      .map_err(|e| e.to_string())?;
    }

    for s in extracted.selects {
      rkg_db::insert_concurrency_select(
        connection,
        &rkg_db::NewConcurrencySelectRecord {
          file_id,
          source_symbol_qualified_name: s.source_symbol_qualified_name,
          start_line: s.start_line as i64,
          end_line: s.end_line as i64,
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_rust_file_safety(
  connection: &rusqlite::Connection,
  _repository_id: i64,
  file_id: i64,
  path: &str,
  content: &str,
  active_features: &[String],
) -> Result<(), String> {
  if let Ok(extracted) =
    rkg_lang_rust::extract_safety_metadata_from_source(content, path, active_features)
  {
    for b in extracted.unsafe_blocks {
      rkg_db::insert_rust_unsafe_block(
        connection,
        &rkg_db::NewRustUnsafeBlockRecord {
          file_id,
          source_symbol_qualified_name: b.source_symbol_qualified_name,
          start_line: b.start_line as i64,
          end_line: b.end_line as i64,
        },
      )
      .map_err(|e| e.to_string())?;
    }

    for f in extracted.unsafe_functions {
      rkg_db::insert_rust_unsafe_function(
        connection,
        &rkg_db::NewRustUnsafeFunctionRecord {
          file_id,
          qualified_name: f.qualified_name,
          start_line: f.start_line as i64,
          end_line: f.end_line as i64,
        },
      )
      .map_err(|e| e.to_string())?;
    }

    for f in extracted.ffi_bindings {
      rkg_db::insert_rust_ffi_binding(
        connection,
        &rkg_db::NewRustFFIBindingRecord {
          file_id,
          source_symbol_qualified_name: f.source_symbol_qualified_name,
          foreign_item_name: f.foreign_item_name,
          abi: f.abi,
          start_line: f.start_line as i64,
          end_line: f.end_line as i64,
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_file_symbols(
  connection: &rusqlite::Connection,
  file_id: i64,
  path: &str,
  content: &str,
) -> Result<Option<i64>, String> {
  let mut module_symbol_id = None;
  if let Ok(extracted_symbols) = extract_symbols_from_source(content, path) {
    for sym in extracted_symbols {
      let is_module = sym.kind == rkg_core::SymbolKind::Module;
      let symbol_record = rkg_db::insert_symbol(
        connection,
        &rkg_db::NewSymbolRecord {
          file_id,
          name: sym.name,
          qualified_name: sym.qualified_name,
          kind: format!("{:?}", sym.kind),
          start_line: sym.location.start_line as i64,
          end_line: sym.location.end_line as i64,
          start_column: sym.location.start_column.map(|c| c as i64),
          end_column: sym.location.end_column.map(|c| c as i64),
        },
      )
      .map_err(|e| e.to_string())?;

      if is_module {
        module_symbol_id = Some(symbol_record.id);
      }
    }
  }
  Ok(module_symbol_id)
}

fn index_file_imports(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
  source_symbol_id: i64,
) -> Result<(), String> {
  if let Ok(extracted_imports) = extract_imports_from_source(content, path) {
    let mut seen = std::collections::HashSet::new();
    for imp in extracted_imports {
      if !seen.insert(imp.target_qualified_name.clone()) {
        continue;
      }
      let mut target_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &imp.target_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      if target_symbol.is_none() {
        target_symbol = if let Some(last_dot_idx) = imp.target_qualified_name.rfind('.') {
          let parent = &imp.target_qualified_name[..last_dot_idx];
          let child = &imp.target_qualified_name[last_dot_idx + 1..];
          let qname_colons = format!("{parent}::{child}");
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &qname_colons)
            .map_err(|e| e.to_string())?
        } else {
          None
        };
      }

      let (target_id, unresolved) = match target_symbol {
        Some(sym) => (Some(sym.id), None),
        None => (None, Some(imp.target_qualified_name.clone())),
      };

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id,
          target_symbol_id: target_id,
          unresolved_target: unresolved,
          kind: "Imports".to_string(),
          confidence: Some(1.0),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_file_calls(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  if let Ok(extracted_calls) = extract_calls_from_source(content, path) {
    let mut seen_edges = std::collections::HashSet::new();
    for call in extracted_calls {
      let source_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &call.source_symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      let mut target_symbol_id = None;
      let mut unresolved_target = Some(call.target_name.clone());
      let mut confidence = 0.5;

      if call.is_self_call {
        if let Some(colons_idx) = call.source_symbol_qualified_name.find("::") {
          let class_part = &call.source_symbol_qualified_name[colons_idx + 2..];
          if let Some(dot_idx) = class_part.find('.') {
            let class_name = &class_part[..dot_idx];
            let module_name = &call.source_symbol_qualified_name[..colons_idx];
            let method_name = call.method_name.as_deref().unwrap_or("");
            let local_method_qname = format!("{module_name}::{class_name}.{method_name}");

            let resolved = rkg_db::lookup_symbol_by_qualified_name(
              connection,
              repository_id,
              &local_method_qname,
            )
            .map_err(|e| e.to_string())?;

            if let Some(sym) = resolved {
              target_symbol_id = Some(sym.id);
              unresolved_target = None;
              confidence = 1.0;
            } else {
              unresolved_target = Some(format!("{class_name}.{method_name}"));
              confidence = 0.8;
            }
          }
        }
      } else if call.method_name.is_none() {
        let module_name = if let Some(colons_idx) = call.source_symbol_qualified_name.find("::") {
          &call.source_symbol_qualified_name[..colons_idx]
        } else {
          &call.source_symbol_qualified_name
        };
        let local_func_qname = format!("{module_name}::{}", call.target_name);

        let resolved =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &local_func_qname)
            .map_err(|e| e.to_string())?;

        if let Some(sym) = resolved {
          target_symbol_id = Some(sym.id);
          unresolved_target = None;
          confidence = 1.0;
        }

        if target_symbol_id.is_none() {
          confidence = 0.7;
        }
      }

      if !seen_edges.insert((
        source_symbol.id,
        target_symbol_id,
        unresolved_target.clone(),
        call.ordering,
      )) {
        continue;
      }

      rkg_db::insert_edge_with_pipeline_metadata(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: source_symbol.id,
          target_symbol_id,
          unresolved_target,
          kind: "Calls".to_string(),
          confidence: Some(confidence),
        },
        call.ordering.map(|o| o as i64),
        call.placeholders.clone(),
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_file_type_references(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  if let Ok(extracted_type_refs) = extract_type_references_from_source(content, path) {
    let mut seen_type_edges = std::collections::HashSet::new();
    for type_ref in extracted_type_refs {
      let mut source_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &type_ref.source_symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      if source_symbol.is_none() {
        let mut current_name = type_ref.source_symbol_qualified_name.clone();
        while let Some(last_dot_idx) = current_name.rfind('.') {
          current_name = current_name[..last_dot_idx].to_string();
          if let Some(sym) =
            rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &current_name)
              .map_err(|e| e.to_string())?
          {
            source_symbol = Some(sym);
            break;
          }
        }
      }

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      let mut target_symbol_id = None;
      let mut unresolved_target = Some(type_ref.target_type_name.clone());

      let mut target_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &type_ref.target_type_name,
      )
      .map_err(|e| e.to_string())?;

      if target_symbol.is_none() {
        target_symbol = if let Some(last_dot_idx) = type_ref.target_type_name.rfind('.') {
          let parent = &type_ref.target_type_name[..last_dot_idx];
          let child = &type_ref.target_type_name[last_dot_idx + 1..];
          let qname_colons = format!("{parent}::{child}");
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &qname_colons)
            .map_err(|e| e.to_string())?
        } else {
          None
        };
      }

      if let Some(sym) = target_symbol {
        target_symbol_id = Some(sym.id);
        unresolved_target = None;
      }

      if !seen_type_edges.insert((
        source_symbol.id,
        target_symbol_id,
        unresolved_target.clone(),
      )) {
        continue;
      }

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: source_symbol.id,
          target_symbol_id,
          unresolved_target,
          kind: "ReferencesType".to_string(),
          confidence: Some(1.0),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_file_decorators(
  connection: &rusqlite::Connection,
  repository_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  if let Ok(extracted_decs) = extract_decorators_from_source(content, path) {
    let mut seen_dec_edges = std::collections::HashSet::new();
    for dec in extracted_decs {
      let source_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &dec.source_symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      let mut target_symbol_id = None;
      let mut unresolved_target = Some(dec.decorator_name.clone());

      let mut target_symbol =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &dec.decorator_name)
          .map_err(|e| e.to_string())?;

      if target_symbol.is_none() {
        target_symbol = if let Some(last_dot_idx) = dec.decorator_name.rfind('.') {
          let parent = &dec.decorator_name[..last_dot_idx];
          let child = &dec.decorator_name[last_dot_idx + 1..];
          let qname_colons = format!("{parent}::{child}");
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &qname_colons)
            .map_err(|e| e.to_string())?
        } else {
          None
        };
      }

      if let Some(sym) = target_symbol {
        target_symbol_id = Some(sym.id);
        unresolved_target = None;
      }

      if !seen_dec_edges.insert((
        source_symbol.id,
        target_symbol_id,
        unresolved_target.clone(),
      )) {
        continue;
      }

      rkg_db::insert_edge(
        connection,
        &rkg_db::NewEdgeRecord {
          source_symbol_id: source_symbol.id,
          target_symbol_id,
          unresolved_target,
          kind: "ModifiedWith".to_string(),
          confidence: Some(1.0),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn get_test_similarity_target(name: &str) -> Option<String> {
  let candidate = if name.starts_with("test_") {
    name.strip_prefix("test_")
  } else if name.starts_with("test") {
    name.strip_prefix("test")
  } else if name.starts_with("Test") {
    name.strip_prefix("Test")
  } else if name.ends_with("_test") {
    name.strip_suffix("_test")
  } else if name.ends_with("Test") {
    name.strip_suffix("Test")
  } else {
    None
  };

  candidate.map(|c| c.to_string()).filter(|c| !c.is_empty())
}

fn index_file_tests(
  connection: &rusqlite::Connection,
  repository_id: i64,
  file_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  let is_pytest_file = {
    let p = std::path::Path::new(path);
    if let Some(filename) = p.file_name().and_then(|n| n.to_str()) {
      filename == "conftest.py"
        || (filename.ends_with(".py")
          && (filename.starts_with("test_")
            || filename
              .strip_suffix(".py")
              .is_some_and(|stem| stem.ends_with("_test"))))
    } else {
      false
    }
  };

  if !is_pytest_file {
    return Ok(());
  }

  if let Ok(extracted_tests) = extract_tests_from_source(content, path) {
    for t in extracted_tests {
      let kind_str = match t.kind {
        rkg_lang_python::ExtractedTestKind::Class => "Class",
        rkg_lang_python::ExtractedTestKind::Function => "Function",
        rkg_lang_python::ExtractedTestKind::Fixture => "Fixture",
      };

      rkg_db::insert_test(
        connection,
        &rkg_db::NewTestRecord {
          file_id,
          name: t.name.clone(),
          qualified_name: t.qualified_name.clone(),
          kind: kind_str.to_string(),
          is_parametrized: t.is_parametrized,
          framework: "pytest".to_string(),
          start_line: Some(t.start_line as i64),
          end_line: Some(t.end_line as i64),
        },
      )
      .map_err(|e| e.to_string())?;

      let source_symbol =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &t.qualified_name)
          .map_err(|e| e.to_string())?;

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      // 1. Fixture parameters ConfiguredBy edges
      for param in &t.parameters {
        rkg_db::insert_edge(
          connection,
          &rkg_db::NewEdgeRecord {
            source_symbol_id: source_symbol.id,
            target_symbol_id: None,
            unresolved_target: Some(param.clone()),
            kind: "ConfiguredBy".to_string(),
            confidence: Some(1.0),
          },
        )
        .map_err(|e| e.to_string())?;
      }

      // 2. Name similarity TestedBy edges
      let is_test_class_or_fn = t.kind == rkg_lang_python::ExtractedTestKind::Class
        || t.kind == rkg_lang_python::ExtractedTestKind::Function;
      let target_candidate = if is_test_class_or_fn {
        get_test_similarity_target(&t.name)
      } else {
        None
      };

      if let Some(target) = target_candidate {
        rkg_db::insert_edge(
          connection,
          &rkg_db::NewEdgeRecord {
            source_symbol_id: source_symbol.id,
            target_symbol_id: None,
            unresolved_target: Some(target),
            kind: "TestedBy".to_string(),
            confidence: Some(0.8),
          },
        )
        .map_err(|e| e.to_string())?;
      }
    }
  }
  Ok(())
}

fn parse_markdown_to_sections(content: &str) -> Vec<(Option<String>, String, usize, usize)> {
  let mut sections = Vec::new();
  let mut current_title: Option<String> = None;
  let mut current_body = Vec::new();
  let mut current_start = 1;

  let lines: Vec<&str> = content.lines().collect();
  let total_lines = lines.len();

  for (idx, line) in lines.iter().enumerate() {
    let line_num = idx + 1;
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
      let hashes_count = trimmed.chars().take_while(|c| *c == '#').count();
      let rest = &trimmed[hashes_count..];
      if rest.starts_with(' ') || rest.is_empty() {
        let body_text = current_body.join("\n");
        if !body_text.trim().is_empty() || current_title.is_some() {
          sections.push((
            current_title.clone(),
            body_text,
            current_start,
            line_num.saturating_sub(1).max(current_start),
          ));
        }

        current_title = Some(rest.trim().to_string());
        current_body = Vec::new();
        current_start = line_num;
        continue;
      }
    }
    current_body.push(*line);
  }

  let body_text = current_body.join("\n");
  if !body_text.trim().is_empty() || current_title.is_some() {
    sections.push((
      current_title,
      body_text,
      current_start,
      total_lines.max(current_start),
    ));
  }

  sections
}

fn index_markdown_file(
  connection: &rusqlite::Connection,
  file_id: i64,
  content: &str,
) -> Result<(), String> {
  let sections = parse_markdown_to_sections(content);
  for (title, body, start_line, end_line) in sections {
    rkg_db::insert_doc(
      connection,
      &rkg_db::NewDocRecord {
        file_id,
        symbol_id: None,
        title,
        body,
        start_line: Some(start_line as i64),
        end_line: Some(end_line as i64),
        source_kind: "Markdown".to_string(),
      },
    )
    .map_err(|e| e.to_string())?;
  }
  Ok(())
}

fn index_file_docstrings(
  connection: &rusqlite::Connection,
  repository_id: i64,
  file_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  if let Ok(extracted_docs) = rkg_lang_python::extract_docstrings_from_source(content, path) {
    for doc in extracted_docs {
      let symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &doc.symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let symbol_id = symbol.map(|s| s.id);

      rkg_db::insert_doc(
        connection,
        &rkg_db::NewDocRecord {
          file_id,
          symbol_id,
          title: None,
          body: doc.text,
          start_line: Some(doc.start_line as i64),
          end_line: Some(doc.end_line as i64),
          source_kind: "Docstring".to_string(),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn index_file_routes(
  connection: &rusqlite::Connection,
  repository_id: i64,
  file_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  let extracted = if path.ends_with(".rs") {
    match rkg_lang_rust::extract_routes_and_dependencies_from_source(content, path, &[]) {
      Ok((routes, deps)) => {
        let rust_routes: Vec<rkg_lang_python::ExtractedRoute> = routes
          .into_iter()
          .map(|r| rkg_lang_python::ExtractedRoute {
            handler_name: r.handler_name,
            qualified_name: r.qualified_name,
            method: r.method,
            path: r.path,
            response_model: r.response_model,
            start_line: r.start_line,
            start_column: r.start_column,
            end_line: r.end_line,
            end_column: r.end_column,
          })
          .collect();
        Some((rust_routes, deps))
      }
      Err(e) => return Err(e),
    }
  } else if path.ends_with(".py") {
    rkg_lang_python::extract_routes_and_dependencies_from_source(content, path).ok()
  } else {
    None
  };

  if let Some((extracted_routes, extracted_deps)) = extracted {
    for r in extracted_routes {
      let handler_symbol =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &r.qualified_name)
          .map_err(|e| e.to_string())?;

      let symbol_id = handler_symbol.as_ref().map(|sym| sym.id);

      rkg_db::insert_route(
        connection,
        &rkg_db::NewRouteRecord {
          file_id,
          symbol_id,
          handler_name: r.handler_name.clone(),
          qualified_name: r.qualified_name.clone(),
          method: r.method.clone(),
          path: r.path.clone(),
          response_model: r.response_model.clone(),
          start_line: Some(r.start_line as i64),
          end_line: Some(r.end_line as i64),
        },
      )
      .map_err(|e| e.to_string())?;
    }

    for (handler_qname, dep_name) in extracted_deps {
      let source_symbol =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &handler_qname)
          .map_err(|e| e.to_string())?;

      if let Some(source_sym) = source_symbol {
        rkg_db::insert_edge(
          connection,
          &rkg_db::NewEdgeRecord {
            source_symbol_id: source_sym.id,
            target_symbol_id: None,
            unresolved_target: Some(dep_name),
            kind: "ConfiguredBy".to_string(),
            confidence: Some(1.0),
          },
        )
        .map_err(|e| e.to_string())?;
      }
    }
  }
  Ok(())
}

fn index_file_pydantic_models(
  connection: &rusqlite::Connection,
  repository_id: i64,
  file_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  if let Ok(extracted_models) = rkg_lang_python::extract_pydantic_models_from_source(content, path)
  {
    for m in extracted_models {
      let class_symbol =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &m.qualified_name)
          .map_err(|e| e.to_string())?;

      let symbol_id = class_symbol.as_ref().map(|sym| sym.id);

      let model_record = rkg_db::insert_pydantic_model(
        connection,
        &rkg_db::NewPydanticModelRecord {
          file_id,
          symbol_id,
          name: m.name.clone(),
          qualified_name: m.qualified_name.clone(),
          start_line: m.start_line as i64,
          end_line: m.end_line as i64,
        },
      )
      .map_err(|e| e.to_string())?;

      for f in m.fields {
        rkg_db::insert_pydantic_field(
          connection,
          &rkg_db::NewPydanticFieldRecord {
            model_id: model_record.id,
            name: f.name.clone(),
            type_annotation: f.type_annotation.clone(),
            default_value: f.default_value.clone(),
            is_required: f.is_required,
          },
        )
        .map_err(|e| e.to_string())?;
      }

      for v in m.validators {
        rkg_db::insert_pydantic_validator(
          connection,
          &rkg_db::NewPydanticValidatorRecord {
            model_id: model_record.id,
            name: v.name.clone(),
            validator_type: v.validator_type.clone(),
            target_fields: v.target_fields.join(","),
          },
        )
        .map_err(|e| e.to_string())?;
      }
    }
  }
  Ok(())
}

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
  let finished = rkg_db::finish_index_run(
    summary.connection,
    summary.started_run_id,
    "completed",
    &FinishedIndexRunRecord {
      files_scanned: summary.discovered_files.len() as i64,
      files_changed: summary.changed_files.len() as i64,
      files_deleted: summary.deleted_files.len() as i64,
      error_message: None,
    },
  )
  .map_err(|e| e.to_string())?;

  let unchanged_count = summary
    .discovered_files
    .len()
    .saturating_sub(summary.changed_files.len());

  println!("indexed repository: {}", summary.repo_root.display());
  println!("index run id: {}", finished.id);
  println!("files scanned: {}", finished.files_scanned);
  println!("files changed: {}", finished.files_changed);
  println!("files unchanged: {}", unchanged_count);
  println!("files deleted: {}", finished.files_deleted);
  let rust_files_parsed = summary
    .changed_files
    .iter()
    .filter(|f| f.language.as_deref() == Some("rust"))
    .count();
  println!("rust files parsed: {}", rust_files_parsed);
  println!(
    "python files parsed: {}",
    summary.parse_summary.files_parsed
  );
  println!(
    "python files with syntax errors: {}",
    summary.parse_summary.files_with_syntax_errors
  );
  println!(
    "python syntax errors: {}",
    summary.parse_summary.syntax_error_count
  );
  println!(
    "python parser internal errors: {}",
    summary.parse_summary.internal_error_count
  );
  if !summary.parse_summary.issues.is_empty() {
    println!("python parse issues:");
    for issue in &summary.parse_summary.issues {
      if issue.start_line > 0 {
        println!(
          "- {}:{}:{} {}",
          issue.path, issue.start_line, issue.start_column, issue.message
        );
      } else {
        println!("- {} {}", issue.path, issue.message);
      }
    }
  }
  if summary.force {
    println!("mode: force");
  } else {
    println!("mode: incremental");
  }

  Ok(())
}

fn run_files() -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository =
    rkg_db::upsert_repository(&connection, &repo_root_text).map_err(|e| e.to_string())?;

  let files =
    rkg_db::list_files_for_repository(&connection, repository.id).map_err(|e| e.to_string())?;
  for file in files {
    println!("{}", file.path);
  }

  Ok(())
}

fn run_symbols() -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let symbols =
    rkg_db::list_symbols_for_repository(&connection, repository.id).map_err(|e| e.to_string())?;

  for sym in symbols {
    println!("{} [{}]", sym.qualified_name, sym.kind);
  }

  Ok(())
}

fn run_find(name: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let symbols =
    rkg_db::lookup_symbols_by_name(&connection, repository.id, name).map_err(|e| e.to_string())?;

  for sym in symbols {
    let mut stmt = connection
      .prepare("SELECT path FROM files WHERE id = ?1")
      .map_err(|e| e.to_string())?;
    let path: String = stmt
      .query_row([sym.file_id], |row| row.get(0))
      .map_err(|e| e.to_string())?;

    println!(
      "{} [{}] ({}:{}-{})",
      sym.qualified_name, sym.kind, path, sym.start_line, sym.end_line
    );
  }

  Ok(())
}

fn run_show(qualified_name: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let symbol = rkg_db::lookup_symbol_by_qualified_name(&connection, repository.id, qualified_name)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("symbol not found: {qualified_name}"))?;

  let mut stmt = connection
    .prepare("SELECT path FROM files WHERE id = ?1")
    .map_err(|e| e.to_string())?;
  let path_rel: String = stmt
    .query_row([symbol.file_id], |row| row.get(0))
    .map_err(|e| e.to_string())?;

  println!("Symbol: {} [{}]", symbol.qualified_name, symbol.kind);
  if let Ok(Some(prov)) = rkg_db::get_symbol_provenance(&connection, symbol.id) {
    println!("Provenance: {}", prov);
  }
  println!(
    "File: {} (lines {}-{})",
    path_rel, symbol.start_line, symbol.end_line
  );
  println!("----------------------------------------");

  let lines = load_symbol_source_lines(&repo_root, &path_rel, &symbol.qualified_name)?;

  let start_idx = (symbol.start_line as usize).saturating_sub(1);
  let end_idx = (symbol.end_line as usize).min(lines.len());

  if start_idx < lines.len() && start_idx < end_idx {
    for line in &lines[start_idx..end_idx] {
      println!("{line}");
    }
  }
  println!("----------------------------------------");

  Ok(())
}

fn load_symbol_source_lines(
  repo_root: &std::path::Path,
  path_rel: &str,
  qualified_name: &str,
) -> Result<Vec<String>, String> {
  let file_path = repo_root.join(path_rel);
  let content = fs::read_to_string(&file_path)
    .map_err(|e| format!("failed to read file {}: {e}", file_path.display()))?;

  if !path_rel.ends_with(".ipynb") {
    return Ok(content.lines().map(|line| line.to_string()).collect());
  }

  let Some(cell_idx_pos) = qualified_name.find("#cell_") else {
    return Ok(Vec::new());
  };

  let rest = &qualified_name[cell_idx_pos + 6..];
  let end_of_num = rest.find(':').unwrap_or(rest.len());
  let Ok(cell_idx) = rest[..end_of_num].parse::<usize>() else {
    return Ok(Vec::new());
  };
  let Ok(notebook) = serde_json::from_str::<NotebookJson>(&content) else {
    return Ok(Vec::new());
  };
  let Some(cell) = notebook.cells.get(cell_idx) else {
    return Ok(Vec::new());
  };

  let cell_code = match &cell.source {
    SourceField::String(source) => source.clone(),
    SourceField::Array(lines) => lines.join(""),
  };

  Ok(cell_code.lines().map(|line| line.to_string()).collect())
}

fn run_imports(path: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let path_normalized = path.replace('\\', "/");

  let imports = rkg_query::get_imports_for_file(&connection, repository.id, &path_normalized)
    .map_err(|e| e.to_string())?;

  if imports.is_empty() {
    println!("No imports found for file: {}", path_normalized);
  } else {
    for (qname, file_path, unresolved) in imports {
      if unresolved.is_some() {
        println!("{} [Unresolved]", qname);
      } else {
        let fpath = file_path.unwrap_or_else(|| "unknown".to_string());
        println!("{} [Resolved] (File: {})", qname, fpath);
      }
    }
  }

  Ok(())
}

fn run_imported_by(path: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let path_normalized = path.replace('\\', "/");

  let imported_by =
    rkg_query::get_imported_by_for_file(&connection, repository.id, &path_normalized)
      .map_err(|e| e.to_string())?;

  if imported_by.is_empty() {
    println!("No files import: {}", path_normalized);
  } else {
    for (qname, file_path) in imported_by {
      println!("{} ({})", qname, file_path);
    }
  }

  Ok(())
}

fn run_callees(name: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let callees =
    rkg_query::get_callees(&connection, repository.id, name).map_err(|e| e.to_string())?;

  if callees.is_empty() {
    println!("No callees found for symbol: {}", name);
  } else {
    for (target_name, target_kind, target_file_path, confidence) in callees {
      let conf_str = confidence
        .map(|c| format!(" (Confidence: {:.1})", c))
        .unwrap_or_default();
      if let Some(file) = target_file_path {
        let kind = target_kind.unwrap_or_else(|| "unknown".to_string());
        println!(
          "{} [Resolved] [{}] (File: {}){}",
          target_name, kind, file, conf_str
        );
      } else {
        println!("{} [Unresolved]{}", target_name, conf_str);
      }
    }
  }

  Ok(())
}

fn run_callers(name: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let callers =
    rkg_query::get_callers(&connection, repository.id, name).map_err(|e| e.to_string())?;

  if callers.is_empty() {
    println!("No callers found for symbol: {}", name);
  } else {
    for (caller_name, caller_kind, caller_file_path, confidence) in callers {
      let conf_str = confidence
        .map(|c| format!(" (Confidence: {:.1})", c))
        .unwrap_or_default();
      println!(
        "{} [{}] (File: {}){}",
        caller_name, caller_kind, caller_file_path, conf_str
      );
    }
  }

  Ok(())
}

fn run_types(name: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let refs =
    rkg_query::get_type_references(&connection, repository.id, name).map_err(|e| e.to_string())?;

  let referencers =
    rkg_query::get_type_referencers(&connection, repository.id, name).map_err(|e| e.to_string())?;

  if refs.is_empty() && referencers.is_empty() {
    println!("No type references or referencers found for: {}", name);
    return Ok(());
  }

  if !refs.is_empty() {
    println!("Types referenced by {}:", name);
    for (target_name, target_kind, target_file_path, confidence) in &refs {
      let conf_str = confidence
        .map(|c| format!(" (Confidence: {:.1})", c))
        .unwrap_or_default();
      if let Some(file) = target_file_path {
        let kind = target_kind.as_deref().unwrap_or("unknown");
        println!(
          "  - {} [Resolved] [{}] (File: {}){}",
          target_name, kind, file, conf_str
        );
      } else {
        println!("  - {} [Unresolved]{}", target_name, conf_str);
      }
    }
  }

  if !referencers.is_empty() {
    if !refs.is_empty() {
      println!();
    }
    println!("Symbols referencing type {}:", name);
    for (caller_name, caller_kind, caller_file_path, confidence) in &referencers {
      let conf_str = confidence
        .map(|c| format!(" (Confidence: {:.1})", c))
        .unwrap_or_default();
      println!(
        "  - {} [{}] (File: {}){}",
        caller_name, caller_kind, caller_file_path, conf_str
      );
    }
  }

  Ok(())
}

fn run_decorators(name: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let decorators =
    rkg_query::get_decorators(&connection, repository.id, name).map_err(|e| e.to_string())?;

  let decorated_symbols = rkg_query::get_decorated_symbols(&connection, repository.id, name)
    .map_err(|e| e.to_string())?;

  if decorators.is_empty() && decorated_symbols.is_empty() {
    println!("No decorators or decorated symbols found for: {}", name);
    return Ok(());
  }

  if !decorators.is_empty() {
    println!("Decorators modifying {}:", name);
    for (target_name, target_kind, target_file_path, confidence) in &decorators {
      let conf_str = confidence
        .map(|c| format!(" (Confidence: {:.1})", c))
        .unwrap_or_default();
      if let Some(file) = target_file_path {
        let kind = target_kind.as_deref().unwrap_or("unknown");
        println!(
          "  - {} [Resolved] [{}] (File: {}){}",
          target_name, kind, file, conf_str
        );
      } else {
        println!("  - {} [Unresolved]{}", target_name, conf_str);
      }
    }
  }

  if !decorated_symbols.is_empty() {
    if !decorators.is_empty() {
      println!();
    }
    println!("Symbols decorated by {}:", name);
    for (caller_name, caller_kind, caller_file_path, confidence) in &decorated_symbols {
      let conf_str = confidence
        .map(|c| format!(" (Confidence: {:.1})", c))
        .unwrap_or_default();
      println!(
        "  - {} [{}] (File: {}){}",
        caller_name, caller_kind, caller_file_path, conf_str
      );
    }
  }

  Ok(())
}

fn run_tests(name: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let tests =
    rkg_query::get_tests_for_symbol(&connection, repository.id, name).map_err(|e| e.to_string())?;

  if tests.is_empty() {
    println!("No tests found for symbol: {}", name);
  } else {
    println!("Tests testing {}:", name);
    for (test_name, test_file_path, confidence) in tests {
      let conf_str = confidence
        .map(|c| format!(" (Confidence: {:.1})", c))
        .unwrap_or_default();
      println!("  - {} (File: {}){}", test_name, test_file_path, conf_str);
    }
  }

  Ok(())
}

fn run_test_deps(name: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let deps =
    rkg_query::get_test_deps(&connection, repository.id, name).map_err(|e| e.to_string())?;

  let fixtures = rkg_query::get_fixtures_for_test(&connection, repository.id, name)
    .map_err(|e| e.to_string())?;

  if deps.is_empty() && fixtures.is_empty() {
    println!("No dependencies or fixtures found for test: {}", name);
    return Ok(());
  }

  if !deps.is_empty() {
    println!("Implementation symbols tested by {}:", name);
    for (target_name, target_kind, target_file_path, confidence) in &deps {
      let conf_str = confidence
        .map(|c| format!(" (Confidence: {:.1})", c))
        .unwrap_or_default();
      if let Some(file) = target_file_path {
        let kind = target_kind.as_deref().unwrap_or("unknown");
        println!(
          "  - {} [Resolved] [{}] (File: {}){}",
          target_name, kind, file, conf_str
        );
      } else {
        println!("  - {} [Unresolved]{}", target_name, conf_str);
      }
    }
  }

  if !fixtures.is_empty() {
    if !deps.is_empty() {
      println!();
    }
    println!("Fixtures used by {}:", name);
    for (fixture_name, fixture_file_path) in &fixtures {
      if let Some(file) = fixture_file_path {
        println!("  - {} [Resolved] (File: {})", fixture_name, file);
      } else {
        println!("  - {} [Unresolved]", fixture_name);
      }
    }
  }

  Ok(())
}

fn run_fixtures(name: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let fixtures = rkg_query::get_fixtures_for_test(&connection, repository.id, name)
    .map_err(|e| e.to_string())?;

  if fixtures.is_empty() {
    println!("No fixtures found for test: {}", name);
  } else {
    println!("Fixtures used by {}:", name);
    for (fixture_name, fixture_file_path) in fixtures {
      if let Some(file) = fixture_file_path {
        println!("  - {} [Resolved] (File: {})", fixture_name, file);
      } else {
        println!("  - {} [Unresolved]", fixture_name);
      }
    }
  }

  Ok(())
}

fn run_docs(name: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let docs =
    rkg_query::get_docs_for_symbol(&connection, repository.id, name).map_err(|e| e.to_string())?;

  if docs.is_empty() {
    println!("No documentation found for symbol: {}", name);
  } else {
    println!("Documentation for {}:", name);
    for doc in docs {
      let mut stmt = connection
        .prepare("SELECT path FROM files WHERE id = ?1")
        .map_err(|e| e.to_string())?;
      let path: String = stmt
        .query_row([doc.file_id], |row| row.get(0))
        .map_err(|e| e.to_string())?;

      let lines_str = if let (Some(start), Some(end)) = (doc.start_line, doc.end_line) {
        format!(" (lines {}-{})", start, end)
      } else {
        String::new()
      };

      let title_str = if let Some(t) = &doc.title {
        format!(" - Heading: {t}")
      } else {
        String::new()
      };

      println!(
        "--- [{}] {}{}{} ---",
        doc.source_kind, path, lines_str, title_str
      );
      println!("{}", doc.body.trim());
    }
    println!("----------------------------------------");
  }

  Ok(())
}

fn run_doc_search(query: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let results =
    rkg_query::search_docs_fts(&connection, repository.id, query).map_err(|e| e.to_string())?;

  if results.is_empty() {
    println!("No documentation matches found for query: {}", query);
  } else {
    println!("Documentation search results for \"{}\":", query);
    for (doc, path) in results {
      let lines_str = if let (Some(start), Some(end)) = (doc.start_line, doc.end_line) {
        format!(" (lines {}-{})", start, end)
      } else {
        String::new()
      };

      let title_str = if let Some(t) = &doc.title {
        format!(" - Heading: {t}")
      } else {
        String::new()
      };

      println!(
        "--- [{}] {}{}{} ---",
        doc.source_kind, path, lines_str, title_str
      );
      let body_trimmed = doc.body.trim();
      let lines: Vec<&str> = body_trimmed.lines().collect();
      if lines.len() > 5 {
        for line in &lines[..5] {
          println!("{line}");
        }
        println!("...");
      } else {
        println!("{}", body_trimmed);
      }
    }
    println!("----------------------------------------");
  }

  Ok(())
}

fn run_search(query: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let symbols =
    rkg_query::search_symbols_fts(&connection, repository.id, query).map_err(|e| e.to_string())?;

  let docs =
    rkg_query::search_docs_fts(&connection, repository.id, query).map_err(|e| e.to_string())?;

  if symbols.is_empty() && docs.is_empty() {
    println!(
      "No search matches found in symbols or documentation for query: \"{}\"",
      query
    );
    return Ok(());
  }

  println!("================================================================================");
  println!(
    "                       SEARCH RESULTS FOR \"{}\"",
    query.to_uppercase()
  );
  println!("================================================================================");

  if !symbols.is_empty() {
    println!("\n  ● MATCHING SYMBOLS:");
    println!("  -------------------");
    for (sym, path) in &symbols {
      let lines = format!("(lines {}-{})", sym.start_line, sym.end_line);
      println!(
        "  - [{:<10}] {} :: {} {}",
        sym.kind.to_uppercase(),
        path,
        sym.qualified_name,
        lines
      );
    }
  }

  if !docs.is_empty() {
    println!("\n  ● MATCHING DOCUMENTATION:");
    println!("  -------------------------");
    for (doc, path) in &docs {
      let lines_str = if let (Some(start), Some(end)) = (doc.start_line, doc.end_line) {
        format!("(lines {start}-{end})")
      } else {
        String::new()
      };

      let heading_str = if let Some(title) = &doc.title {
        format!(" - Heading: {title}")
      } else {
        String::new()
      };

      println!(
        "  - [{:<10}] {} {} {}",
        doc.source_kind.to_uppercase(),
        path,
        lines_str,
        heading_str
      );

      let cleaned_body = clean_docstring_for_excerpt(&doc.body);
      let lines: Vec<&str> = cleaned_body.lines().collect();
      if !lines.is_empty() {
        print!("      ↳ excerpt: ");
        if lines.len() > 1 {
          println!("\"{}\" ...", lines[0].trim());
        } else {
          println!("\"{}\"", lines[0].trim());
        }
      }
    }
  }

  println!("================================================================================");
  Ok(())
}

fn clean_docstring_for_excerpt(body: &str) -> String {
  let trimmed = body.trim();
  let mut cleaned = trimmed;
  if (cleaned.starts_with("\"\"\"") && cleaned.ends_with("\"\"\"")
    || cleaned.starts_with("'''") && cleaned.ends_with("'''"))
    && cleaned.len() >= 6
  {
    cleaned = &cleaned[3..cleaned.len() - 3];
  } else if ((cleaned.starts_with('"') && cleaned.ends_with('"'))
    || (cleaned.starts_with('\'') && cleaned.ends_with('\'')))
    && cleaned.len() >= 2
  {
    cleaned = &cleaned[1..cleaned.len() - 1];
  }
  cleaned.trim().to_string()
}

fn get_outgoing_calls(
  connection: &rusqlite::Connection,
  symbol_id: i64,
) -> Result<Vec<String>, String> {
  let mut stmt = connection
    .prepare("SELECT DISTINCT unresolved_target FROM edges WHERE source_symbol_id = ?1 AND kind = 'Calls' AND unresolved_target IS NOT NULL")
    .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map([symbol_id], |row| row.get::<_, String>(0))
    .map_err(|e| e.to_string())?;

  let mut targets = Vec::new();
  for t in rows.flatten() {
    targets.push(t);
  }

  let mut stmt2 = connection
    .prepare("SELECT DISTINCT s.name FROM edges e INNER JOIN symbols s ON e.target_symbol_id = s.id WHERE e.source_symbol_id = ?1 AND e.kind = 'Calls'")
    .map_err(|e| e.to_string())?;
  let rows2 = stmt2
    .query_map([symbol_id], |row| row.get::<_, String>(0))
    .map_err(|e| e.to_string())?;
  for t in rows2.flatten() {
    targets.push(t);
  }

  Ok(targets)
}

fn get_callers(connection: &rusqlite::Connection, symbol_id: i64) -> Result<Vec<i64>, String> {
  let mut stmt = connection
    .prepare(
      "SELECT DISTINCT source_symbol_id FROM edges WHERE target_symbol_id = ?1 AND kind = 'Calls'",
    )
    .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map([symbol_id], |row| row.get::<_, i64>(0))
    .map_err(|e| e.to_string())?;

  let mut callers = Vec::new();
  for c in rows.flatten() {
    callers.push(c);
  }

  Ok(callers)
}

fn lookup_symbol_by_id(
  connection: &rusqlite::Connection,
  id: i64,
) -> Result<Option<rkg_db::SymbolRecord>, String> {
  let mut stmt = connection
    .prepare("SELECT id, file_id, name, qualified_name, kind, start_line, end_line, start_column, end_column FROM symbols WHERE id = ?1")
    .map_err(|e| e.to_string())?;

  let mut rows = stmt
    .query_map([id], |row| {
      Ok(rkg_db::SymbolRecord {
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
    .map_err(|e| e.to_string())?;

  if let Some(r) = rows.next() {
    let sym = r.map_err(|e| e.to_string())?;
    Ok(Some(sym))
  } else {
    Ok(None)
  }
}

struct CallRecord {
  target_name: String,
  target_qname: Option<String>,
  ordering: Option<i64>,
  placeholders: Option<String>,
}

fn get_outgoing_calls_with_records(
  connection: &rusqlite::Connection,
  symbol_id: i64,
) -> Result<Vec<CallRecord>, String> {
  let mut records = Vec::new();

  let mut stmt = connection
    .prepare("SELECT unresolved_target, ordering, placeholders FROM edges WHERE source_symbol_id = ?1 AND kind = 'Calls' AND unresolved_target IS NOT NULL")
    .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map([symbol_id], |row| {
      Ok((
        row.get::<_, String>(0)?,
        row.get::<_, Option<i64>>(1)?,
        row.get::<_, Option<String>>(2)?,
      ))
    })
    .map_err(|e| e.to_string())?;
  for (name, ordering, placeholders) in rows.flatten() {
    records.push(CallRecord {
      target_name: name,
      target_qname: None,
      ordering,
      placeholders,
    });
  }

  let mut stmt2 = connection
    .prepare("SELECT s.name, s.qualified_name, e.ordering, e.placeholders FROM edges e INNER JOIN symbols s ON e.target_symbol_id = s.id WHERE e.source_symbol_id = ?1 AND e.kind = 'Calls'")
    .map_err(|e| e.to_string())?;
  let rows2 = stmt2
    .query_map([symbol_id], |row| {
      Ok((
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, Option<i64>>(2)?,
        row.get::<_, Option<String>>(3)?,
      ))
    })
    .map_err(|e| e.to_string())?;

  for (name, qname, ordering, placeholders) in rows2.flatten() {
    records.push(CallRecord {
      target_name: name,
      target_qname: Some(qname),
      ordering,
      placeholders,
    });
  }

  Ok(records)
}

fn run_concurrency(symbol_name: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let report = rkg_query::get_concurrency_topology(&connection, repository.id, symbol_name)?;

  println!(
    "Concurrency Topology for symbol: {}\n",
    report.target_symbol
  );

  println!("[Spawns]");
  if report.spawns.is_empty() {
    println!("  (No async/thread tasks spawned)");
  } else {
    for s in report.spawns {
      let target = s.target_name.as_deref().unwrap_or("closure/block");
      println!(
        "  - Spawns task executing `{}` via `{}` at {}:{}",
        target, s.spawn_kind, s.location.file_path, s.location.start_line
      );
    }
  }
  println!();

  println!("[Channels]");
  if report.channels.is_empty() {
    println!("  (No channels created)");
  } else {
    for c in report.channels {
      println!(
        "  - Created `{}` channel with tx `{}` and rx `{}` at {}:{}",
        c.channel_kind, c.tx_name, c.rx_name, c.location.file_path, c.location.start_line
      );
    }
  }
  println!();

  println!("[Selects]");
  if report.selects.is_empty() {
    println!("  (No concurrency select blocks found)");
  } else {
    for s in report.selects {
      println!(
        "  - Concurrency select block at {}:{}",
        s.location.file_path, s.location.start_line
      );
    }
  }

  Ok(())
}

fn run_safety(target: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let profile = rkg_query::get_safety_profile(&connection, repository.id, target)?;

  println!("=========================================");
  println!("Memory Safety & FFI Risk Profile");
  println!("=========================================");
  println!("Target:        {}", profile.target_name);
  println!("Safety Score:  {}/100", profile.safety_score);
  println!("Risk Level:    {}", profile.risk_level);
  println!("Safe Wrappers: {:.1}%", profile.safe_wrapper_percentage);
  println!("=========================================");
  println!();

  println!("[Unsafe Functions & Trait Boundaries]");
  if profile.unsafe_functions.is_empty() {
    println!("  (No unsafe functions or trait boundaries found)");
  } else {
    for f in &profile.unsafe_functions {
      println!(
        "  - {} at {}:{}",
        f.qualified_name, f.location.file_path, f.location.start_line
      );
    }
  }
  println!();

  println!("[Unsafe Code Blocks]");
  if profile.unsafe_blocks.is_empty() {
    println!("  (No unsafe blocks found)");
  } else {
    for b in &profile.unsafe_blocks {
      let mut is_wrapped = true;
      for f in &profile.unsafe_functions {
        if b.location.file_path == f.location.file_path
          && b.location.start_line >= f.location.start_line
          && b.location.end_line <= f.location.end_line
        {
          is_wrapped = false;
          break;
        }
      }
      let status_str = if is_wrapped {
        "SAFELY WRAPPED"
      } else {
        "EXPOSED"
      };
      println!(
        "  - [{}] block in {} at {}:{}",
        status_str, b.source_symbol_qualified_name, b.location.file_path, b.location.start_line
      );
    }
  }
  println!();

  println!("[FFI / Foreign Interface Bindings]");
  if profile.ffi_bindings.is_empty() {
    println!("  (No FFI bindings found)");
  } else {
    for f in &profile.ffi_bindings {
      println!(
        "  - [{}] Foreign item `{}` in {} at {}:{}",
        f.abi,
        f.foreign_item_name,
        f.source_symbol_qualified_name,
        f.location.file_path,
        f.location.start_line
      );
    }
  }

  Ok(())
}

fn run_import_coverage(report_path_str: &str, test_suite: Option<String>) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let report_content = fs::read_to_string(report_path_str)
    .map_err(|e| format!("failed to read coverage report: {e}"))?;

  let parsed_report = rkg_indexer::coverage_parser::parse_coverage_report(&report_content)?;

  let final_test_suite = test_suite.or(parsed_report.test_suite);

  // Clear existing coverage for this report + test suite combination:
  rkg_db::delete_symbol_coverage_by_report(
    &connection,
    report_path_str,
    final_test_suite.as_deref(),
  )
  .map_err(|e| e.to_string())?;

  let indexed_files =
    rkg_db::list_files_for_repository(&connection, repository.id).map_err(|e| e.to_string())?;

  let mut symbols_updated = 0;
  let mut files_updated = 0;

  for file_cov in parsed_report.file_coverages {
    let norm_report_path = file_cov.file_path.replace('\\', "/");

    let matched_file = indexed_files.iter().find(|f| {
      let norm_db_path = f.path.replace('\\', "/");
      norm_report_path == norm_db_path
        || norm_report_path.ends_with(&format!("/{}", norm_db_path))
        || norm_db_path.ends_with(&format!("/{}", norm_report_path))
    });

    if let Some(file_rec) = matched_file {
      let file_symbols =
        rkg_db::lookup_symbols_by_file_id(&connection, file_rec.id).map_err(|e| e.to_string())?;

      if file_symbols.is_empty() {
        continue;
      }

      files_updated += 1;

      for sym in file_symbols {
        let mut lines_valid = 0;
        let mut lines_covered = 0;
        let mut coverable_lines = Vec::new();
        let mut uncovered_lines = Vec::new();

        for (&line_num, &hits) in &file_cov.line_hits {
          if line_num >= sym.start_line as usize && line_num <= sym.end_line as usize {
            lines_valid += 1;
            coverable_lines.push(line_num);
            if hits > 0 {
              lines_covered += 1;
            } else {
              uncovered_lines.push(line_num);
            }
          }
        }

        if lines_valid > 0 {
          coverable_lines.sort();
          uncovered_lines.sort();

          let coverable_str = coverable_lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join(",");
          let uncovered_str = uncovered_lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join(",");

          // Calculate branch coverage for the symbol
          let mut branches_valid = 0;
          let mut branches_covered = 0;
          for (&line_num, &(cov, tot)) in &file_cov.line_branches {
            if line_num >= sym.start_line as usize && line_num <= sym.end_line as usize {
              branches_valid += tot as i64;
              branches_covered += cov as i64;
            }
          }

          let db_rec = rkg_db::NewSymbolCoverageRecord {
            file_id: file_rec.id,
            symbol_id: sym.id,
            report_path: report_path_str.to_string(),
            test_suite: final_test_suite.clone(),
            lines_valid: lines_valid as i64,
            lines_covered: lines_covered as i64,
            branches_valid,
            branches_covered,
            coverable_lines: coverable_str,
            uncovered_lines: uncovered_str,
          };

          rkg_db::insert_symbol_coverage(&connection, &db_rec).map_err(|e| e.to_string())?;
          symbols_updated += 1;
        }
      }
    }
  }

  println!(
    "Imported coverage from `{}` (test suite: {}): matched {} files, updated {} symbols.",
    report_path_str,
    final_test_suite.as_deref().unwrap_or("default"),
    files_updated,
    symbols_updated
  );

  Ok(())
}

fn run_coverage(target: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let profile = rkg_query::get_coverage_profile(&connection, repository.id, target)?;

  println!("=========================================");
  println!("Coverage Profile");
  println!("=========================================");
  println!("Target:        {}", profile.target_name);
  println!(
    "Type:          {}",
    if profile.is_file { "File" } else { "Symbol" }
  );
  println!("=========================================");

  let line_rate = if profile.lines_valid > 0 {
    (profile.lines_covered as f64 / profile.lines_valid as f64) * 100.0
  } else {
    0.0
  };
  println!(
    "Statement Coverage: {:.1}% ({}/{} lines)",
    line_rate, profile.lines_covered, profile.lines_valid
  );

  if profile.branches_valid > 0 {
    let branch_rate = (profile.branches_covered as f64 / profile.branches_valid as f64) * 100.0;
    println!(
      "Branch Coverage:    {:.1}% ({}/{} branches)",
      branch_rate, profile.branches_covered, profile.branches_valid
    );
  } else {
    println!("Branch Coverage:    N/A (no branch data present)");
  }

  println!("=========================================");
  println!();

  // Print uncovered line spans
  println!("[Uncovered Line Spans]");
  if profile.uncovered_lines.is_empty() {
    println!("  (No uncovered lines - 100% coverage!)");
  } else {
    let mut spans = Vec::new();
    let mut start = None;
    let mut prev = None;

    for &line in &profile.uncovered_lines {
      match (start, prev) {
        (None, None) => {
          start = Some(line);
          prev = Some(line);
        }
        (Some(s), Some(p)) => {
          if line == p + 1 {
            prev = Some(line);
          } else {
            if s == p {
              spans.push(format!("{}", s));
            } else {
              spans.push(format!("{}-{}", s, p));
            }
            start = Some(line);
            prev = Some(line);
          }
        }
        _ => {}
      }
    }

    if let (Some(s), Some(p)) = (start, prev) {
      if s == p {
        spans.push(format!("{}", s));
      } else {
        spans.push(format!("{}-{}", s, p));
      }
    }

    println!("  {}", spans.join(", "));
  }
  println!();

  // Print test breakdown
  println!("[Breakdown by Report / Test Suite]");
  if profile.test_suites.is_empty() {
    println!("  (No report breakdowns found)");
  } else {
    for suite in &profile.test_suites {
      let suite_line_rate = if suite.lines_valid > 0 {
        (suite.lines_covered as f64 / suite.lines_valid as f64) * 100.0
      } else {
        0.0
      };

      let suite_name = suite.test_suite.as_deref().unwrap_or("default");
      print!(
        "  - [{}] via `{}`\n    Lines: {:.1}% ({}/{})",
        suite_name, suite.report_path, suite_line_rate, suite.lines_covered, suite.lines_valid
      );

      if suite.branches_valid > 0 {
        let suite_branch_rate =
          (suite.branches_covered as f64 / suite.branches_valid as f64) * 100.0;
        println!(
          ", Branches: {:.1}% ({}/{})",
          suite_branch_rate, suite.branches_covered, suite.branches_valid
        );
      } else {
        println!(", Branches: N/A");
      }
    }
  }

  Ok(())
}

fn run_pipeline(symbol_name: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let resolved_symbols = rkg_query::resolve_start_symbols(&connection, repository.id, symbol_name)
    .map_err(|e| e.to_string())?;

  if resolved_symbols.is_empty() {
    println!("No matching symbols found for: {}", symbol_name);
    return Ok(());
  }

  println!("Pipeline / Functional Flow Analysis for: {}\n", symbol_name);

  for symbol in &resolved_symbols {
    let mut pipeline_scopes = Vec::new();

    // Check if symbol itself has outgoing calls
    let outgoing_calls = get_outgoing_calls(&connection, symbol.id)?;
    if !outgoing_calls.is_empty() {
      pipeline_scopes.push(symbol.clone());
    }

    // Find any other symbols P that call this symbol
    let callers = get_callers(&connection, symbol.id)?;
    for caller_id in callers {
      if let Some(caller_sym) = lookup_symbol_by_id(&connection, caller_id)? {
        let exists = pipeline_scopes.iter().any(|s| s.id == caller_sym.id);
        if !exists {
          pipeline_scopes.push(caller_sym);
        }
      }
    }

    if pipeline_scopes.is_empty() {
      println!(
        "No pipeline or functional flow detected for symbol: {}",
        symbol.qualified_name
      );
      continue;
    }

    for scope in &pipeline_scopes {
      let mut stmt = connection
        .prepare("SELECT path FROM files WHERE id = ?1")
        .map_err(|e| e.to_string())?;
      let path_rel: String = stmt
        .query_row([scope.file_id], |row| row.get(0))
        .map_err(|e| e.to_string())?;

      let file_path = repo_root.join(&path_rel);
      let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("failed to read file {}: {e}", file_path.display()))?;
      let lines: Vec<&str> = content.lines().collect();

      let start_idx = (scope.start_line as usize).saturating_sub(1);
      let end_idx = (scope.end_line as usize).min(lines.len());
      let scope_source = if start_idx < lines.len() && start_idx < end_idx {
        lines[start_idx..end_idx].join("\n")
      } else {
        String::new()
      };

      let all_calls = get_outgoing_calls_with_records(&connection, scope.id)?;

      let mut steps = Vec::new();
      for call in all_calls {
        let name = call.target_name;
        if name == "map"
          || name == "filter"
          || name == "reduce"
          || name == "partial"
          || name == "functools.partial"
          || name == "pipe"
          || name == "print"
          || name == "len"
          || name == "list"
          || name == "dict"
          || name == "set"
          || name == "bind"
          || name == "then"
          || name == "flatMap"
          || name == "apply"
          || name.ends_with(".bind")
          || name.ends_with(".then")
          || name.ends_with(".flatMap")
          || name.ends_with(".apply")
        {
          continue;
        }
        let ord = call
          .ordering
          .unwrap_or_else(|| scope_source.find(&name).map(|p| p as i64).unwrap_or(9999));
        steps.push((ord, name, call.target_qname, call.placeholders));
      }

      steps.sort_by_key(|s| s.0);
      steps.dedup_by(|a, b| a.1 == b.1);

      if steps.is_empty() {
        continue;
      }

      println!("=========================================");
      println!("Pipeline: {} (in {})", scope.qualified_name, path_rel);
      println!("=========================================");

      let mut target_index = None;
      for (i, (_pos, name, _qname, _phs)) in steps.iter().enumerate() {
        if name == &symbol.name || name == &symbol.qualified_name {
          target_index = Some(i);
        }
      }

      for (i, (_pos, name, qname, placeholders)) in steps.iter().enumerate() {
        let mut qname_display = qname.as_deref().unwrap_or(name).to_string();
        if let Some(phs) = placeholders {
          qname_display.push_str(&format!(" [unresolved placeholders: {}]", phs));
        }

        if Some(i) == target_index {
          println!("  [{}] Step {}: {} [TARGET]", i + 1, i + 1, qname_display);
        } else if let Some(target_index) = target_index {
          if i <= target_index {
            println!("  [-] Step {}: {}", i + 1, qname_display);
            continue;
          }
          println!(
            "  [!] Step {}: {} [FAIL-FAST BLAST RADIUS (BYPASSED ON FAILURE)]",
            i + 1,
            qname_display
          );
        } else {
          println!("  [-] Step {}: {}", i + 1, qname_display);
        }
      }
      println!();
    }
  }

  Ok(())
}

fn run_impact(symbol: &str, depth: usize) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let results = rkg_query::analyze_impact(&connection, repository.id, symbol, depth)
    .map_err(|e| e.to_string())?;

  if results.target_symbols.is_empty() {
    println!("No matching symbols found for: {}", symbol);
    return Ok(());
  }

  println!("Target Symbol(s) matched:");
  for sym in &results.target_symbols {
    let mut stmt = connection
      .prepare("SELECT path FROM files WHERE id = ?1")
      .map_err(|e| e.to_string())?;
    let path: String = stmt
      .query_row([sym.file_id], |row| row.get(0))
      .map_err(|e| e.to_string())?;

    println!(
      "  - {} [{}] (File: {}, Lines: {}-{})",
      sym.qualified_name, sym.kind, path, sym.start_line, sym.end_line
    );
  }
  println!();

  println!("=========================================");
  println!("DOWNSTREAM IMPACT (Blast Radius / Forward)");
  println!("=========================================");
  let mut has_downstream = false;
  for d in 1..=depth {
    let depth_nodes: Vec<_> = results
      .downstream_nodes
      .iter()
      .filter(|n| n.depth == d)
      .collect();
    if !depth_nodes.is_empty() {
      has_downstream = true;
      println!("Depth {}:", d);
      for node in depth_nodes {
        let edge_info = results
          .downstream_edges
          .iter()
          .find(|e| e.target_symbol_id == node.symbol.id);
        let rel_str = if let Some(edge) = edge_info {
          format!(" via {}", edge.kind)
        } else {
          String::new()
        };
        println!(
          "  -> {} [{}] (File: {}, Lines: {}-{}){}",
          node.symbol.qualified_name,
          node.symbol.kind,
          node.file_path,
          node.symbol.start_line,
          node.symbol.end_line,
          rel_str
        );
      }
    }
  }
  if !has_downstream {
    println!("  None");
  }
  println!();

  println!("=========================================");
  println!("UPSTREAM IMPACT (Affected Code / Backward)");
  println!("=========================================");
  let mut has_upstream = false;
  for d in 1..=depth {
    let depth_nodes: Vec<_> = results
      .upstream_nodes
      .iter()
      .filter(|n| n.depth == d)
      .collect();
    if !depth_nodes.is_empty() {
      has_upstream = true;
      println!("Depth {}:", d);
      for node in depth_nodes {
        let edge_info = results
          .upstream_edges
          .iter()
          .find(|e| e.source_symbol_id == node.symbol.id);
        let rel_str = if let Some(edge) = edge_info {
          format!(" via {}", edge.kind)
        } else {
          String::new()
        };
        println!(
          "  <- {} [{}] (File: {}, Lines: {}-{}){}",
          node.symbol.qualified_name,
          node.symbol.kind,
          node.file_path,
          node.symbol.start_line,
          node.symbol.end_line,
          rel_str
        );
      }
    }
  }
  if !has_upstream {
    println!("  None");
  }
  println!();

  println!("=========================================");
  println!("AFFECTED TESTS");
  println!("=========================================");
  if results.affected_tests.is_empty() {
    println!("  None");
  } else {
    for test in &results.affected_tests {
      let conf_str = test
        .confidence
        .map(|c| format!(" (Confidence: {:.1})", c))
        .unwrap_or_default();
      println!(
        "  - {} [{}] (File: {}) -> Tests {}{}",
        test.test_qualified_name,
        test.test_kind,
        test.file_path,
        test.linked_symbol_qualified_name,
        conf_str
      );
    }
  }
  println!();

  println!("=========================================");
  println!("AFFECTED DOCUMENTATION");
  println!("=========================================");
  if results.affected_docs.is_empty() {
    println!("  None");
  } else {
    for doc in &results.affected_docs {
      let title_str = if let Some(t) = &doc.title {
        format!(": Heading \"{}\"", t)
      } else {
        ": Docstring".to_string()
      };
      let lines_str = if let (Some(start), Some(end)) = (doc.start_line, doc.end_line) {
        format!(" (Lines: {}-{})", start, end)
      } else {
        String::new()
      };
      println!(
        "  - File: {}{}{} -> Documents {}",
        doc.file_path, title_str, lines_str, doc.linked_symbol_qualified_name
      );
    }
  }
  println!();

  Ok(())
}

fn run_context(symbol: &str, budget: Option<usize>, format: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let pack = rkg_query::pack_context(
    &connection,
    repository.id,
    &repo_root,
    symbol,
    budget,
    format,
  )
  .map_err(|e| e.to_string())?;

  if pack.symbols.is_empty() {
    println!("No matching symbols found for: {}", symbol);
    return Ok(());
  }

  let formatted = if format == "json" {
    rkg_query::format_context_pack_json(&pack, &repo_root)
  } else {
    rkg_query::format_context_pack_markdown(&pack, &repo_root)
  };

  println!("{}", formatted);

  Ok(())
}

fn index_git_history(
  connection: &rusqlite::Connection,
  repository_id: i64,
  repo_root: &std::path::Path,
) -> Result<(), String> {
  // Use limit = 0 to fetch the full Git history by default
  let commits = match rkg_indexer::extract_git_history(repo_root, 0) {
    Ok(c) => c,
    Err(e) => {
      eprintln!("warning: git metadata extraction skipped: {e}");
      return Ok(());
    }
  };
  rkg_db::clear_git_metadata(connection, repository_id).map_err(|e| e.to_string())?;

  let symbols =
    rkg_db::list_symbols_for_repository(connection, repository_id).map_err(|e| e.to_string())?;

  let mut symbols_by_file: std::collections::HashMap<String, Vec<rkg_indexer::SymbolLineRange>> =
    std::collections::HashMap::new();
  for sym in symbols {
    let mut stmt = connection
      .prepare("SELECT path FROM files WHERE id = ?1")
      .map_err(|e| e.to_string())?;
    let path: String = stmt
      .query_row([sym.file_id], |row| row.get(0))
      .map_err(|e| e.to_string())?;
    symbols_by_file
      .entry(path)
      .or_default()
      .push(rkg_indexer::SymbolLineRange {
        qualified_name: sym.qualified_name,
        start_line: sym.start_line as usize,
        end_line: sym.end_line as usize,
      });
  }

  for dc in commits {
    let commit_id = rkg_db::insert_git_commit(connection, repository_id, &dc.commit)
      .map_err(|e| e.to_string())?;

    for file_path in &dc.modified_files {
      let _ = rkg_db::insert_git_commit_file(connection, commit_id, file_path);

      if let Some(file_syms) = symbols_by_file.get(file_path) {
        let changed = rkg_indexer::extract_symbol_changes_in_commit(
          repo_root,
          &dc.commit.hash,
          file_path,
          file_syms,
        );
        if let Ok(changed_qnames) = changed {
          for qname in changed_qnames {
            let _ = rkg_db::insert_git_commit_symbol(connection, commit_id, &qname);
          }
        }
      }
    }
  }

  Ok(())
}

fn run_git(path: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let meta = rkg_query::get_git_metadata_for_file(&connection, repository.id, path)
    .map_err(|e| e.to_string())?;

  println!("Git Metadata for {}:", path);
  println!("=========================================");
  println!("File Churn: {} commits", meta.churn);

  if let Some(commit) = meta.last_commit {
    println!("Last Modified:");
    println!("  Commit:  {}", commit.hash);
    println!(
      "  Author:  {} <{}>",
      commit.author_name, commit.author_email
    );
    println!("  Date:    {}", commit.date);
    println!("  Message: {}", commit.message);
  }

  println!("\nAuthor Frequency:");
  if meta.author_frequency.is_empty() {
    println!("  No author history recorded.");
  } else {
    let total_commits: usize = meta.author_frequency.iter().map(|(_, count)| count).sum();
    for (author, count) in &meta.author_frequency {
      let pct = if total_commits > 0 {
        (*count as f64 / total_commits as f64) * 100.0
      } else {
        0.0
      };
      println!("  {}: {} commits ({:.1}%)", author, count, pct);
    }
  }

  Ok(())
}

fn run_routes() -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found; run 'rkg index' first".to_string())?;

  let routes =
    rkg_db::lookup_routes_for_repository(&connection, repository.id).map_err(|e| e.to_string())?;

  if routes.is_empty() {
    println!("No routes found in the repository. Run 'rkg index' first.");
    return Ok(());
  }

  println!(
    "{:<8} {:<30} {:<45} {:<25} {:<30}",
    "METHOD", "PATH", "HANDLER", "RESPONSE MODEL", "DEPENDENCIES"
  );
  println!("{}", "-".repeat(143));

  for r in routes {
    let deps_records =
      rkg_db::lookup_fixtures_for_test(&connection, repository.id, &r.qualified_name)
        .map_err(|e| e.to_string())?;

    let deps = deps_records
      .into_iter()
      .map(|(name, _path)| {
        if let Some(colons_idx) = name.find("::") {
          name[colons_idx + 2..].to_string()
        } else {
          name
        }
      })
      .collect::<Vec<_>>()
      .join(", ");

    let response_model = r.response_model.as_deref().unwrap_or("-");
    let deps_str = if deps.is_empty() { "-" } else { &deps };

    println!(
      "{:<8} {:<30} {:<45} {:<25} {:<30}",
      r.method, r.path, r.qualified_name, response_model, deps_str
    );
  }

  Ok(())
}

fn run_model(name: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found; run 'rkg index' first".to_string())?;

  let model = rkg_db::lookup_pydantic_model_by_name(&connection, repository.id, name)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("Pydantic model not found: {}", name))?;

  let mut file_stmt = connection
    .prepare("SELECT path FROM files WHERE id = ?1")
    .map_err(|e| e.to_string())?;
  let file_path: String = file_stmt
    .query_row([model.file_id], |row| row.get(0))
    .map_err(|e| e.to_string())?;

  println!("Model: {} (defined in {})", model.qualified_name, file_path);
  println!(
    "{}",
    "=".repeat(model.qualified_name.len() + file_path.len() + 14)
  );

  let fields =
    rkg_db::lookup_pydantic_fields_for_model(&connection, model.id).map_err(|e| e.to_string())?;
  println!("\nFields:");
  if fields.is_empty() {
    println!("  (No fields found)");
  } else {
    for f in &fields {
      let req_str = if f.is_required {
        "required"
      } else {
        "optional"
      };
      let default_str = if let Some(ref d) = f.default_value {
        format!(" = {}", d)
      } else {
        "".to_string()
      };
      println!(
        "  - {}: {}{} ({})",
        f.name, f.type_annotation, default_str, req_str
      );
    }
  }

  let validators = rkg_db::lookup_pydantic_validators_for_model(&connection, model.id)
    .map_err(|e| e.to_string())?;
  println!("\nValidators:");
  if validators.is_empty() {
    println!("  (No validators found)");
  } else {
    for v in &validators {
      let targets = if v.target_fields.is_empty() {
        "model validator".to_string()
      } else {
        format!("validates: {}", v.target_fields)
      };
      println!("  - {} ({})", v.name, targets);
    }
  }

  let type_refs = rkg_query::get_type_references(&connection, repository.id, &model.qualified_name)
    .map_err(|e| e.to_string())?;

  println!("\nDependencies:");
  let mut has_deps = false;
  for (ref_name, _, ref_file_path, _) in type_refs {
    if let Some(file) = ref_file_path.filter(|_| {
      rkg_db::lookup_pydantic_model_by_name(&connection, repository.id, &ref_name)
        .map(|opt| opt.is_some())
        .unwrap_or(false)
    }) {
      println!("  - {} (defined in {})", ref_name, file);
      has_deps = true;
    }
  }
  if !has_deps {
    println!("  (No model dependencies)");
  }

  Ok(())
}

fn run_cochange(name: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found".to_string())?;

  let analysis =
    rkg_query::analyze_cochanges(&connection, repository.id, name).map_err(|e| e.to_string())?;

  println!("Co-change Analysis for '{}':", analysis.target);
  println!("=========================================");
  println!("Churn: {} commits", analysis.churn);

  if !analysis.symbol_cochanges.is_empty() {
    println!("\nSymbols Changed Together:");
    for record in &analysis.symbol_cochanges {
      println!(
        "  {}: {} times ({:.1}% co-change rate)",
        record.name, record.count, record.rate
      );
    }
  }

  println!("\nFiles Changed Together:");
  if analysis.file_cochanges.is_empty() {
    println!("  No co-changing files found.");
  } else {
    for record in &analysis.file_cochanges {
      println!(
        "  {}: {} times ({:.1}% co-change rate)",
        record.name, record.count, record.rate
      );
    }
  }

  Ok(())
}

fn run_mcp_serve() -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found. Please run `rkg index` first.".to_string())?;

  rkg_mcp::run_mcp_server(&connection, repository.id, &repo_root)
}

fn default_db_path() -> Result<PathBuf, String> {
  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  Ok(current_dir.join(".rkg").join("rkg.db"))
}

fn ensure_db_initialized(db_path: &PathBuf) -> Result<(), String> {
  let _ = open_db_connection(db_path)?;
  Ok(())
}

fn open_db_connection(db_path: &PathBuf) -> Result<rusqlite::Connection, String> {
  let parent = db_path
    .parent()
    .ok_or_else(|| "database path has no parent directory".to_string())?;
  fs::create_dir_all(parent).map_err(|e| {
    format!(
      "failed to create database directory {}: {e}",
      parent.display()
    )
  })?;
  let connection = rkg_db::open_or_create(db_path).map_err(|e| e.to_string())?;
  Ok(connection)
}

fn run_workspace() -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found; run 'rkg index' first".to_string())?;

  let packages =
    rkg_db::list_cargo_packages(&connection, repository.id).map_err(|e| e.to_string())?;

  let fsharp_projects =
    rkg_db::list_fsharp_projects(&connection, repository.id).map_err(|e| e.to_string())?;

  let kotlin_projects =
    rkg_db::list_kotlin_projects(&connection, repository.id).map_err(|e| e.to_string())?;

  let swift_projects =
    rkg_db::list_swift_projects(&connection, repository.id).map_err(|e| e.to_string())?;

  if packages.is_empty()
    && fsharp_projects.is_empty()
    && kotlin_projects.is_empty()
    && swift_projects.is_empty()
  {
    println!(
      "No Cargo packages, F# projects, Kotlin projects, or Swift projects found in the repository workspace. Run 'rkg index' first."
    );
    return Ok(());
  }

  if !packages.is_empty() {
    println!(
      "===================================================================================================="
    );
    println!(
      "                                       CARGO PACKAGES                                               "
    );
    println!(
      "===================================================================================================="
    );
    println!(
      "{:<25} {:<50} {:<10} {:<30}",
      "PACKAGE", "MANIFEST PATH", "VERSION", "ACTIVE FEATURES"
    );
    println!("{}", "-".repeat(120));

    for pkg in packages {
      let active_features = match fs::read_to_string(repo_root.join(&pkg.manifest_path)) {
        Ok(content) => {
          let (pkg_opt, _) =
            rkg_indexer::cargo_parser::parse_cargo_toml(&content, &pkg.manifest_path);
          if let Some(parsed) = pkg_opt {
            let mut active = vec!["default".to_string()];
            if let Some(deps) = parsed.features.get("default") {
              for dep in deps {
                active.push(dep.clone());
              }
            }
            active.join(", ")
          } else {
            "default".to_string()
          }
        }
        Err(_) => "default".to_string(),
      };

      println!(
        "{:<25} {:<50} {:<10} {:<30}",
        pkg.name, pkg.manifest_path, pkg.version, active_features
      );
    }
    println!();
  }

  if !fsharp_projects.is_empty() {
    println!(
      "===================================================================================================="
    );
    println!(
      "                                        F# PROJECTS                                                 "
    );
    println!(
      "===================================================================================================="
    );
    println!(
      "{:<25} {:<50} {:<15} {:<20}",
      "PROJECT", "PROJECT PATH", "FRAMEWORK", "SOLUTION MEMBER"
    );
    println!("{}", "-".repeat(110));

    for proj in fsharp_projects {
      let tf = proj.target_framework.as_deref().unwrap_or("-");
      let sol_member = if proj.is_solution_member { "yes" } else { "no" };
      println!(
        "{:<25} {:<50} {:<15} {:<20}",
        proj.name, proj.project_path, tf, sol_member
      );
    }
    println!();
  }

  if !kotlin_projects.is_empty() {
    println!(
      "===================================================================================================="
    );
    println!(
      "                                       KOTLIN PROJECTS                                              "
    );
    println!(
      "===================================================================================================="
    );
    println!(
      "{:<25} {:<50} {:<15} {:<20}",
      "PROJECT", "PROJECT PATH", "FRAMEWORK", "SOLUTION MEMBER"
    );
    println!("{}", "-".repeat(110));

    for proj in kotlin_projects {
      let tf = proj.target_framework.as_deref().unwrap_or("-");
      let sol_member = if proj.is_solution_member { "yes" } else { "no" };
      println!(
        "{:<25} {:<50} {:<15} {:<20}",
        proj.name, proj.project_path, tf, sol_member
      );
    }
  }

  if !swift_projects.is_empty() {
    println!(
      "===================================================================================================="
    );
    println!(
      "                                        SWIFT PROJECTS                                              "
    );
    println!(
      "===================================================================================================="
    );
    println!(
      "{:<25} {:<50} {:<15} {:<20}",
      "PROJECT", "PROJECT PATH", "FRAMEWORK", "SOLUTION MEMBER"
    );
    println!("{}", "-".repeat(110));

    for proj in swift_projects {
      let tf = proj.target_framework.as_deref().unwrap_or("-");
      let sol_member = if proj.is_solution_member { "yes" } else { "no" };
      println!(
        "{:<25} {:<50} {:<15} {:<20}",
        proj.name, proj.project_path, tf, sol_member
      );
    }
    println!();
  }

  Ok(())
}

fn run_topology() -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found; run 'rkg index' first".to_string())?;

  let mut stmt = connection
    .prepare(
      "SELECT s_src.qualified_name, s_src.kind, e.unresolved_target, s_tgt.qualified_name, e.kind
     FROM edges e
     INNER JOIN symbols s_src ON e.source_symbol_id = s_src.id
     INNER JOIN files f ON s_src.file_id = f.id
     LEFT JOIN symbols s_tgt ON e.target_symbol_id = s_tgt.id
     WHERE f.repository_id = ?1 AND e.kind IN ('Spawns', 'SendsTo')
     ORDER BY e.kind DESC, s_src.qualified_name ASC",
    )
    .map_err(|e| e.to_string())?;

  let rows = stmt
    .query_map([repository.id], |row| {
      let source_qname: String = row.get(0)?;
      let _source_kind: String = row.get(1)?;
      let unresolved_target: Option<String> = row.get(2)?;
      let resolved_target: Option<String> = row.get(3)?;
      let kind: String = row.get(4)?;

      let target = resolved_target
        .or(unresolved_target)
        .unwrap_or_else(|| "unknown".to_string());
      Ok((source_qname, target, kind))
    })
    .map_err(|e| e.to_string())?;

  let mut edges = Vec::new();
  for r in rows {
    edges.push(r.map_err(|e| e.to_string())?);
  }

  if edges.is_empty() {
    println!(
      "No concurrency topology found. Run 'rkg index' on source files containing supported spawn or channel patterns."
    );
    return Ok(());
  }

  println!(
    "===================================================================================================="
  );
  println!(
    "                                WORKSPACE CONCURRENCY TOPOLOGY                                     "
  );
  println!(
    "===================================================================================================="
  );
  println!();

  let spawn_edges: Vec<_> = edges
    .iter()
    .filter(|(_, _, kind)| kind == "Spawns")
    .collect();
  if !spawn_edges.is_empty() {
    println!(
      "┌──────────────────────────────────────────────────────────────────────────────────────────────────┐"
    );
    println!(
      "│ 🧵 SPAWNING TOPOLOGY (Tasks & Threads)                                                           │"
    );
    println!(
      "├──────────────────────────────────────────────────────────────────────────────────────────────────┤"
    );
    for (src, tgt, _) in spawn_edges {
      println!("│  {:<45}  ==[Spawns]==>  {:<40} │", src, tgt);
    }
    println!(
      "└──────────────────────────────────────────────────────────────────────────────────────────────────┘"
    );
    println!();
  }

  let channel_edges: Vec<_> = edges
    .iter()
    .filter(|(_, _, kind)| kind == "SendsTo")
    .collect();
  if !channel_edges.is_empty() {
    println!(
      "┌──────────────────────────────────────────────────────────────────────────────────────────────────┐"
    );
    println!(
      "│ ✉️  CHANNEL PIPELINES & DATAFLOW PATHWAYS                                                         │"
    );
    println!(
      "├──────────────────────────────────────────────────────────────────────────────────────────────────┤"
    );
    for (src, tgt, _) in channel_edges {
      println!("│  {:<45}  ──[SendsTo]──>  {:<40} │", src, tgt);
    }
    println!(
      "└──────────────────────────────────────────────────────────────────────────────────────────────────┘"
    );
    println!();
  }

  Ok(())
}

fn run_deps(package_name: &str) -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found; run 'rkg index' first".to_string())?;

  let packages =
    rkg_db::list_cargo_packages(&connection, repository.id).map_err(|e| e.to_string())?;
  let fsharp_projects =
    rkg_db::list_fsharp_projects(&connection, repository.id).map_err(|e| e.to_string())?;
  let kotlin_projects =
    rkg_db::list_kotlin_projects(&connection, repository.id).map_err(|e| e.to_string())?;
  let swift_projects =
    rkg_db::list_swift_projects(&connection, repository.id).map_err(|e| e.to_string())?;

  let target_pkg = packages.iter().find(|p| p.name == package_name);
  let target_fs_proj = fsharp_projects.iter().find(|p| p.name == package_name);
  let target_kt_proj = kotlin_projects.iter().find(|p| p.name == package_name);
  let target_sw_proj = swift_projects.iter().find(|p| p.name == package_name);

  if target_pkg.is_none()
    && target_fs_proj.is_none()
    && target_kt_proj.is_none()
    && target_sw_proj.is_none()
  {
    return Err(format!(
      "package or project '{}' not found in workspace",
      package_name
    ));
  }

  if let Some(pkg) = target_pkg {
    let deps = rkg_db::list_cargo_dependencies(&connection, pkg.id).map_err(|e| e.to_string())?;

    println!("Package: {} ({})", pkg.name, pkg.version);
    println!("Manifest: {}", pkg.manifest_path);
    println!();

    if deps.is_empty() {
      println!("No dependencies found.");
      return Ok(());
    }

    println!(
      "{:<25} {:<20} {:<15} {:<10} {:<40}",
      "DEPENDENCY", "VERSION REQUIREMENT", "TYPE", "DEV", "FEATURES"
    );
    println!("{}", "-".repeat(115));

    for dep in deps {
      let dep_type = if dep.is_workspace_dependency {
        "Internal"
      } else {
        "External"
      };

      let dev_str = if dep.is_dev { "yes" } else { "no" };
      let req = dep.version_requirement.as_deref().unwrap_or("*");
      let feats_str = if dep.features.is_empty() {
        "-"
      } else {
        &dep.features
      };

      println!(
        "{:<25} {:<20} {:<15} {:<10} {:<40}",
        dep.name, req, dep_type, dev_str, feats_str
      );
    }
  } else if let Some(proj) = target_fs_proj {
    let deps = rkg_db::list_fsharp_dependencies(&connection, proj.id).map_err(|e| e.to_string())?;

    let tf = proj.target_framework.as_deref().unwrap_or("-");
    println!("Project: {} (Framework: {})", proj.name, tf);
    println!("Path: {}", proj.project_path);
    println!();

    if deps.is_empty() {
      println!("No dependencies found.");
      return Ok(());
    }

    println!(
      "{:<35} {:<30} {:<25}",
      "DEPENDENCY", "VERSION REQUIREMENT", "TYPE"
    );
    println!("{}", "-".repeat(90));

    for dep in deps {
      let dep_type = if dep.dependency_type == "project" {
        "Project Reference"
      } else {
        "NuGet Package"
      };

      let req = dep.version_requirement.as_deref().unwrap_or("*");

      println!("{:<35} {:<30} {:<25}", dep.name, req, dep_type);
    }
  } else if let Some(proj) = target_kt_proj {
    let deps = rkg_db::list_kotlin_dependencies(&connection, proj.id).map_err(|e| e.to_string())?;

    let tf = proj.target_framework.as_deref().unwrap_or("-");
    println!("Project: {} (Framework: {})", proj.name, tf);
    println!("Path: {}", proj.project_path);
    println!();

    if deps.is_empty() {
      println!("No dependencies found.");
      return Ok(());
    }

    println!(
      "{:<35} {:<30} {:<25}",
      "DEPENDENCY", "VERSION REQUIREMENT", "TYPE"
    );
    println!("{}", "-".repeat(90));

    for dep in deps {
      let dep_type = if dep.dependency_type == "project" {
        "Project Reference"
      } else {
        "Maven/Gradle Dependency"
      };

      let req = dep.version_requirement.as_deref().unwrap_or("*");

      println!("{:<35} {:<30} {:<25}", dep.name, req, dep_type);
    }
  } else if let Some(proj) = target_sw_proj {
    let deps = rkg_db::list_swift_dependencies(&connection, proj.id).map_err(|e| e.to_string())?;

    let tf = proj.target_framework.as_deref().unwrap_or("-");
    println!("Project: {} (Framework: {})", proj.name, tf);
    println!("Path: {}", proj.project_path);
    println!();

    if deps.is_empty() {
      println!("No dependencies found.");
      return Ok(());
    }

    println!(
      "{:<35} {:<30} {:<25}",
      "DEPENDENCY", "VERSION REQUIREMENT", "TYPE"
    );
    println!("{}", "-".repeat(90));

    for dep in deps {
      let dep_type = if dep.dependency_type == "project" {
        "Project Reference"
      } else {
        "SPM Package"
      };

      let req = dep.version_requirement.as_deref().unwrap_or("*");

      println!("{:<35} {:<30} {:<25}", dep.name, req, dep_type);
    }
  }

  Ok(())
}

fn check_edge_exists(
  connection: &rusqlite::Connection,
  source_id: i64,
  target_id: Option<i64>,
  unresolved: Option<&str>,
  kind: &str,
) -> Result<bool, String> {
  let mut stmt = connection
    .prepare(
      "SELECT COUNT(*) FROM edges 
       WHERE source_symbol_id = ?1 AND kind = ?2 
       AND (target_symbol_id = ?3 OR (target_symbol_id IS NULL AND ?3 IS NULL))
       AND (unresolved_target = ?4 OR (unresolved_target IS NULL AND ?4 IS NULL))",
    )
    .map_err(|e| e.to_string())?;

  let count: i64 = stmt
    .query_row((source_id, kind, target_id, unresolved), |row| row.get(0))
    .map_err(|e| e.to_string())?;

  Ok(count > 0)
}

/// Normalizes a relative `Path` by walking its components and collapsing `..`
/// segments, producing a canonical forward-slash string without touching the FS.
/// For example `solutions/../src/Lib/Lib.fsproj` → `src/Lib/Lib.fsproj`.
fn normalize_relative_path(path: &std::path::Path) -> String {
  use std::path::Component;
  let mut parts: Vec<&str> = Vec::new();
  for component in path.components() {
    match component {
      Component::ParentDir => {
        if let Some(last) = parts.last() {
          if *last == ".." {
            parts.push("..");
          } else {
            parts.pop();
          }
        } else {
          parts.push("..");
        }
      }
      Component::Normal(s) => {
        parts.push(s.to_str().unwrap_or(""));
      }
      Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
    }
  }
  parts.join("/")
}

fn module_name_from_path(file_path: &str) -> String {
  let normalized = file_path.replace('\\', "/");
  let path_without_mod = if normalized.ends_with("/mod.rs") {
    &normalized[..normalized.len() - 7]
  } else if normalized == "mod.rs" {
    ""
  } else {
    normalized.strip_suffix(".rs").unwrap_or(&normalized)
  };
  path_without_mod.replace('/', "::")
}

fn find_file_id_for_expanded_symbol(
  connection: &rusqlite::Connection,
  repository_id: i64,
  entry_file_id: i64,
  qname: &str,
  manifest_dir: &str,
) -> i64 {
  let mut subparts_opt = None;
  if let Some(idx) = qname.find("src::lib::") {
    subparts_opt = Some(&qname[idx + 10..]);
  } else if let Some(idx) = qname.find("src::main::") {
    subparts_opt = Some(&qname[idx + 11..]);
  }

  if let Some(first_sub) = subparts_opt.and_then(|s| s.split("::").next()) {
    let path1 = if manifest_dir.is_empty() {
      format!("src/{first_sub}.rs")
    } else {
      format!("{manifest_dir}/src/{first_sub}.rs")
    };
    let path2 = if manifest_dir.is_empty() {
      format!("src/{first_sub}/mod.rs")
    } else {
      format!("{manifest_dir}/src/{first_sub}/mod.rs")
    };
    if let Ok(Some(f)) = rkg_db::lookup_file_by_path(connection, repository_id, &path1) {
      return f.id;
    }
    if let Ok(Some(f)) = rkg_db::lookup_file_by_path(connection, repository_id, &path2) {
      return f.id;
    }
  }
  entry_file_id
}

fn run_cargo_expand_indexing(
  connection: &rusqlite::Connection,
  repository_id: i64,
  repo_root: &std::path::Path,
  parsed_packages: &[rkg_indexer::cargo_parser::ParsedPackage],
) -> Result<(), String> {
  println!("Running post-macro AST extraction via `cargo expand`...");

  for pkg in parsed_packages {
    let manifest_path_abs = repo_root.join(&pkg.manifest_path);
    let Some(manifest_dir) = manifest_path_abs.parent() else {
      continue;
    };

    let expand_check = std::process::Command::new("cargo")
      .arg("expand")
      .arg("--version")
      .output();
    if expand_check.is_err() {
      println!(
        "Warning: `cargo expand` is not installed or failed to run. Skipping post-macro extraction."
      );
      return Ok(());
    }

    let mut active_features = vec!["default".to_string()];
    if let Some(feats) = pkg.features.get("default") {
      for feat in feats {
        active_features.push(feat.clone());
      }
    }

    println!("Expanding package {}...", pkg.name);
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("expand").arg("--theme=none");
    cmd.current_dir(manifest_dir);

    let output = cmd
      .output()
      .map_err(|e| format!("Failed to run `cargo expand`: {e}"))?;
    if !output.status.success() {
      let stderr = String::from_utf8_lossy(&output.stderr);
      println!(
        "Warning: `cargo expand` failed for package {}: {}. Skipping.",
        pkg.name,
        stderr.trim()
      );
      continue;
    }

    let expanded_source = String::from_utf8_lossy(&output.stdout).to_string();

    let mut entry_rel = format!(
      "{}/src/lib.rs",
      pkg
        .manifest_path
        .rfind('/')
        .map(|idx| &pkg.manifest_path[..idx])
        .unwrap_or("")
    );
    if entry_rel.starts_with('/') {
      entry_rel = entry_rel[1..].to_string();
    }
    if entry_rel.is_empty() {
      entry_rel = "src/lib.rs".to_string();
    }

    let mut entry_path_abs = repo_root.join(&entry_rel);
    if !entry_path_abs.exists() {
      let mut main_rel = format!(
        "{}/src/main.rs",
        pkg
          .manifest_path
          .rfind('/')
          .map(|idx| &pkg.manifest_path[..idx])
          .unwrap_or("")
      );
      if main_rel.starts_with('/') {
        main_rel = main_rel[1..].to_string();
      }
      if main_rel.is_empty() {
        main_rel = "src/main.rs".to_string();
      }
      entry_path_abs = repo_root.join(&main_rel);
      entry_rel = main_rel;
    }

    if !entry_path_abs.exists() {
      println!(
        "Warning: Could not find entry point src/lib.rs or src/main.rs for package {}. Skipping.",
        pkg.name
      );
      continue;
    }

    let entry_file = rkg_db::lookup_file_by_path(connection, repository_id, &entry_rel)
      .map_err(|e| e.to_string())?;
    let Some(entry_file) = entry_file else {
      println!(
        "Warning: Entry point file {} not indexed. Skipping.",
        entry_rel
      );
      continue;
    };

    let extracted_symbols =
      rkg_lang_rust::extract_symbols_from_source(&expanded_source, &entry_rel, &active_features)
        .map_err(|e| format!("Failed to extract expanded symbols: {e}"))?;

    let extracted_impls = rkg_lang_rust::extract_implementations_from_source(
      &expanded_source,
      &entry_rel,
      &active_features,
    )
    .map_err(|e| format!("Failed to extract expanded implementations: {e}"))?;

    let extracted_calls =
      rkg_lang_rust::extract_calls_from_source(&expanded_source, &entry_rel, &active_features)
        .map_err(|e| format!("Failed to extract expanded calls: {e}"))?;

    let extracted_type_refs = rkg_lang_rust::extract_type_references_from_source(
      &expanded_source,
      &entry_rel,
      &active_features,
    )
    .map_err(|e| format!("Failed to extract expanded type references: {e}"))?;

    let extracted_tests =
      rkg_lang_rust::extract_tests_from_source(&expanded_source, &entry_rel, &active_features)
        .map_err(|e| format!("Failed to extract expanded tests: {e}"))?;

    let manifest_dir_str = pkg
      .manifest_path
      .rfind('/')
      .map(|idx| &pkg.manifest_path[..idx])
      .unwrap_or("");

    let get_target_file_id = |qname: &str| -> i64 {
      find_file_id_for_expanded_symbol(
        connection,
        repository_id,
        entry_file.id,
        qname,
        manifest_dir_str,
      )
    };

    let mut symbol_qname_to_id = std::collections::HashMap::new();
    for sym in extracted_symbols {
      let target_file_id = get_target_file_id(&sym.qualified_name);

      let existing =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &sym.qualified_name)
          .map_err(|e| e.to_string())?;

      let sym_id = match existing {
        Some(record) => record.id,
        None => {
          let record = rkg_db::insert_symbol(
            connection,
            &rkg_db::NewSymbolRecord {
              file_id: target_file_id,
              name: sym.name,
              qualified_name: sym.qualified_name.clone(),
              kind: format!("{:?}", sym.kind),
              start_line: sym.location.start_line as i64,
              end_line: sym.location.end_line as i64,
              start_column: sym.location.start_column.map(|c| c as i64),
              end_column: sym.location.end_column.map(|c| c as i64),
            },
          )
          .map_err(|e| e.to_string())?;
          record.id
        }
      };
      symbol_qname_to_id.insert(sym.qualified_name.clone(), sym_id);
    }

    let entry_module_name = module_name_from_path(&entry_rel);
    let mut seen_impl_edges = std::collections::HashSet::new();
    for imp in extracted_impls {
      let Some(trait_name) = imp.trait_name else {
        continue;
      };

      let mut struct_id = None;
      let struct_qname = format!("{entry_module_name}::{}", imp.struct_name);

      let struct_sym =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &struct_qname)
          .map_err(|e| e.to_string())?;

      if let Some(sym) = struct_sym {
        struct_id = Some(sym.id);
      } else if let Some(id) = symbol_qname_to_id.get(&struct_qname) {
        struct_id = Some(*id);
      } else {
        let matches = rkg_db::lookup_symbols_by_name(connection, repository_id, &imp.struct_name)
          .map_err(|e| e.to_string())?;
        if !matches.is_empty() {
          let mut best_match = &matches[0];
          for m in &matches {
            if m.qualified_name.starts_with(&entry_module_name) {
              best_match = m;
              break;
            }
          }
          struct_id = Some(best_match.id);
        }
      }

      let Some(struct_id) = struct_id else {
        continue;
      };

      let trait_qname = if manifest_dir_str.is_empty() {
        trait_name.clone()
      } else {
        format!("{manifest_dir_str}::{trait_name}")
      };

      let mut trait_sym =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &trait_qname)
          .map_err(|e| e.to_string())?;

      if trait_sym.is_none() {
        let matches = rkg_db::lookup_symbols_by_name(connection, repository_id, &trait_name)
          .map_err(|e| e.to_string())?;
        if matches.len() == 1 {
          trait_sym = Some(matches[0].clone());
        }
      }

      let (target_id, unresolved) = match trait_sym {
        Some(sym) => (Some(sym.id), None),
        None => (None, Some(trait_name.clone())),
      };

      if !seen_impl_edges.insert((struct_id, target_id, unresolved.clone())) {
        continue;
      }

      let edge_exists = check_edge_exists(
        connection,
        struct_id,
        target_id,
        unresolved.as_deref(),
        "Implements",
      )?;
      if !edge_exists {
        rkg_db::insert_edge(
          connection,
          &rkg_db::NewEdgeRecord {
            source_symbol_id: struct_id,
            target_symbol_id: target_id,
            unresolved_target: unresolved,
            kind: "Implements".to_string(),
            confidence: Some(1.0),
          },
        )
        .map_err(|e| e.to_string())?;
      }
    }

    for call in extracted_calls {
      let source_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &call.source_symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      let mut target_symbol_id = None;
      let mut unresolved_target = Some(call.target_name.clone());
      let mut confidence = 0.5;

      if call.is_self_call {
        if let Some(colons_idx) = call.source_symbol_qualified_name.find("::") {
          let struct_part = &call.source_symbol_qualified_name[colons_idx + 2..];
          if let Some(inner_colons) = struct_part.find("::") {
            let struct_name = &struct_part[..inner_colons];
            let module_name = &call.source_symbol_qualified_name[..colons_idx];
            let method_name = call.method_name.as_deref().unwrap_or("");
            let local_method_qname = format!("{module_name}::{struct_name}::{method_name}");

            let resolved = rkg_db::lookup_symbol_by_qualified_name(
              connection,
              repository_id,
              &local_method_qname,
            )
            .map_err(|e| e.to_string())?;

            if let Some(sym) = resolved {
              target_symbol_id = Some(sym.id);
              unresolved_target = None;
              confidence = 1.0;
            } else {
              unresolved_target = Some(format!("{struct_name}::{method_name}"));
              confidence = 0.8;
            }
          }
        }
      } else if call.method_name.is_none() {
        let module_name = if let Some(colons_idx) = call.source_symbol_qualified_name.find("::") {
          &call.source_symbol_qualified_name[..colons_idx]
        } else {
          &call.source_symbol_qualified_name
        };
        let local_func_qname = format!("{module_name}::{}", call.target_name);

        let resolved =
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &local_func_qname)
            .map_err(|e| e.to_string())?;

        if let Some(sym) = resolved {
          target_symbol_id = Some(sym.id);
          unresolved_target = None;
          confidence = 1.0;
        }

        if target_symbol_id.is_none() {
          confidence = 0.7;
        }
      }

      let edge_exists = check_edge_exists(
        connection,
        source_symbol.id,
        target_symbol_id,
        unresolved_target.as_deref(),
        "Calls",
      )?;
      if !edge_exists {
        rkg_db::insert_edge(
          connection,
          &rkg_db::NewEdgeRecord {
            source_symbol_id: source_symbol.id,
            target_symbol_id,
            unresolved_target,
            kind: "Calls".to_string(),
            confidence: Some(confidence),
          },
        )
        .map_err(|e| e.to_string())?;
      }
    }

    for type_ref in extracted_type_refs {
      let source_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &type_ref.source_symbol_qualified_name,
      )
      .map_err(|e| e.to_string())?;

      let Some(source_symbol) = source_symbol else {
        continue;
      };

      let mut target_symbol_id = None;
      let mut unresolved_target = Some(type_ref.target_type_name.clone());

      let mut target_symbol = rkg_db::lookup_symbol_by_qualified_name(
        connection,
        repository_id,
        &type_ref.target_type_name,
      )
      .map_err(|e| e.to_string())?;

      if target_symbol.is_none() {
        target_symbol = if let Some(last_colons_idx) = type_ref.target_type_name.rfind("::") {
          let parent = &type_ref.target_type_name[..last_colons_idx];
          let child = &type_ref.target_type_name[last_colons_idx + 2..];
          let qname_colons = format!("{parent}::{child}");
          rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &qname_colons)
            .map_err(|e| e.to_string())?
        } else {
          None
        };
      }

      if let Some(sym) = target_symbol {
        target_symbol_id = Some(sym.id);
        unresolved_target = None;
      }

      let edge_exists = check_edge_exists(
        connection,
        source_symbol.id,
        target_symbol_id,
        unresolved_target.as_deref(),
        "ReferencesType",
      )?;
      if !edge_exists {
        rkg_db::insert_edge(
          connection,
          &rkg_db::NewEdgeRecord {
            source_symbol_id: source_symbol.id,
            target_symbol_id,
            unresolved_target,
            kind: "ReferencesType".to_string(),
            confidence: Some(1.0),
          },
        )
        .map_err(|e| e.to_string())?;
      }
    }

    for t in extracted_tests {
      let target_file_id = get_target_file_id(&t.qualified_name);
      let kind_str = match t.kind {
        rkg_lang_rust::ExtractedTestKind::Class => "Class",
        rkg_lang_rust::ExtractedTestKind::Function => "Function",
        rkg_lang_rust::ExtractedTestKind::Fixture => "Fixture",
      };

      rkg_db::insert_test(
        connection,
        &rkg_db::NewTestRecord {
          file_id: target_file_id,
          name: t.name.clone(),
          qualified_name: t.qualified_name.clone(),
          kind: kind_str.to_string(),
          is_parametrized: t.is_parametrized,
          framework: "cargo".to_string(),
          start_line: Some(t.start_line as i64),
          end_line: Some(t.end_line as i64),
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }

  Ok(())
}

fn index_xml_file(
  connection: &rusqlite::Connection,
  repository_id: i64,
  file_id: i64,
  path: &str,
  content: &str,
) -> Result<(), String> {
  let mut is_layout = false;
  let mut is_navigation = false;
  let mut is_values = false;
  let normalized_path = path.replace('\\', "/");
  let parts: Vec<&str> = normalized_path.split('/').collect();
  for i in 0..parts.len() {
    if parts[i] == "res" && i + 1 < parts.len() {
      let folder = parts[i + 1];
      if folder.starts_with("layout") {
        is_layout = true;
      } else if folder.starts_with("navigation") {
        is_navigation = true;
      } else if folder.starts_with("values") {
        is_values = true;
      }
    }
  }

  if path.ends_with("AndroidManifest.xml") {
    let (components, permissions) = rkg_indexer::android_parser::parse_manifest(content);
    for perm in &permissions {
      rkg_db::insert_android_resource(
        connection,
        &NewAndroidResourceRecord {
          file_id,
          name: perm.clone(),
          resource_type: "permission".to_string(),
          value: None,
          start_line: None,
          end_line: None,
        },
      )
      .map_err(|e| e.to_string())?;
    }
    for comp in components {
      rkg_db::insert_android_component(
        connection,
        &NewAndroidComponentRecord {
          file_id,
          name: comp.name.clone(),
          component_type: comp.component_type.clone(),
          class_name: comp.class_name.clone(),
          permission: comp.permission.clone(),
          intent_actions: comp.intent_actions.clone(),
          intent_categories: comp.intent_categories.clone(),
          start_line: Some(comp.start_line as i64),
          end_line: Some(comp.end_line as i64),
        },
      )
      .map_err(|e| e.to_string())?;

      if !comp.class_name.is_empty() {
        let qname = colon_qualified_name(&comp.class_name);
        let name = comp
          .class_name
          .split('.')
          .next_back()
          .unwrap_or(&comp.class_name)
          .to_string();
        rkg_db::insert_symbol(
          connection,
          &rkg_db::NewSymbolRecord {
            file_id,
            name,
            qualified_name: qname,
            kind: "Class".to_string(),
            start_line: comp.start_line as i64,
            end_line: comp.end_line as i64,
            start_column: None,
            end_column: None,
          },
        )
        .map_err(|e| e.to_string())?;
      }
    }
  } else if is_layout {
    let (resources, links) = rkg_indexer::android_parser::parse_layout(content, path);
    for res in resources {
      rkg_db::insert_android_resource(
        connection,
        &NewAndroidResourceRecord {
          file_id,
          name: res.name.clone(),
          resource_type: res.resource_type.clone(),
          value: res.value.clone(),
          start_line: Some(res.start_line as i64),
          end_line: Some(res.end_line as i64),
        },
      )
      .map_err(|e| e.to_string())?;

      let qname = format!("R.{}.{}", res.resource_type, res.name);
      rkg_db::insert_symbol(
        connection,
        &rkg_db::NewSymbolRecord {
          file_id,
          name: res.name.clone(),
          qualified_name: qname,
          kind: "Struct".to_string(),
          start_line: res.start_line as i64,
          end_line: res.end_line as i64,
          start_column: None,
          end_column: None,
        },
      )
      .map_err(|e| e.to_string())?;
    }

    let mut seen_links = std::collections::HashSet::new();
    for link in links {
      if !seen_links.insert((link.source.clone(), link.target.clone(), link.kind.clone())) {
        continue;
      }
      let source_symbol =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &link.source)
          .map_err(|e| e.to_string())?;
      if let Some(source_sym) = source_symbol {
        let unresolved_target = if link.target.starts_with("R.") {
          link.target
        } else {
          colon_qualified_name(&link.target)
        };

        rkg_db::insert_edge(
          connection,
          &rkg_db::NewEdgeRecord {
            source_symbol_id: source_sym.id,
            target_symbol_id: None,
            unresolved_target: Some(unresolved_target),
            kind: link.kind,
            confidence: Some(1.0),
          },
        )
        .map_err(|e| e.to_string())?;
      }
    }
  } else if is_navigation {
    let (resources, links) = rkg_indexer::android_parser::parse_navigation(content);
    for res in resources {
      rkg_db::insert_android_resource(
        connection,
        &NewAndroidResourceRecord {
          file_id,
          name: res.name.clone(),
          resource_type: res.resource_type.clone(),
          value: res.value.clone(),
          start_line: Some(res.start_line as i64),
          end_line: Some(res.end_line as i64),
        },
      )
      .map_err(|e| e.to_string())?;

      let qname = format!("R.{}.{}", res.resource_type, res.name);
      rkg_db::insert_symbol(
        connection,
        &rkg_db::NewSymbolRecord {
          file_id,
          name: res.name.clone(),
          qualified_name: qname,
          kind: "Struct".to_string(),
          start_line: res.start_line as i64,
          end_line: res.end_line as i64,
          start_column: None,
          end_column: None,
        },
      )
      .map_err(|e| e.to_string())?;
    }

    let mut seen_links = std::collections::HashSet::new();
    for link in links {
      if !seen_links.insert((link.source.clone(), link.target.clone(), link.kind.clone())) {
        continue;
      }
      let source_symbol =
        rkg_db::lookup_symbol_by_qualified_name(connection, repository_id, &link.source)
          .map_err(|e| e.to_string())?;
      if let Some(source_sym) = source_symbol {
        let unresolved_target = if link.target.starts_with("R.") {
          link.target
        } else {
          colon_qualified_name(&link.target)
        };

        rkg_db::insert_edge(
          connection,
          &rkg_db::NewEdgeRecord {
            source_symbol_id: source_sym.id,
            target_symbol_id: None,
            unresolved_target: Some(unresolved_target),
            kind: link.kind,
            confidence: Some(1.0),
          },
        )
        .map_err(|e| e.to_string())?;
      }
    }
  } else if is_values {
    let resources = rkg_indexer::android_parser::parse_values(content);
    for res in resources {
      rkg_db::insert_android_resource(
        connection,
        &NewAndroidResourceRecord {
          file_id,
          name: res.name.clone(),
          resource_type: res.resource_type.clone(),
          value: res.value.clone(),
          start_line: Some(res.start_line as i64),
          end_line: Some(res.end_line as i64),
        },
      )
      .map_err(|e| e.to_string())?;

      let qname = format!("R.{}.{}", res.resource_type, res.name);
      rkg_db::insert_symbol(
        connection,
        &rkg_db::NewSymbolRecord {
          file_id,
          name: res.name.clone(),
          qualified_name: qname,
          kind: "Struct".to_string(),
          start_line: res.start_line as i64,
          end_line: res.end_line as i64,
          start_column: None,
          end_column: None,
        },
      )
      .map_err(|e| e.to_string())?;
    }
  }

  Ok(())
}

fn colon_qualified_name(class_name: &str) -> String {
  if let Some(last_dot) = class_name.rfind('.') {
    let package = &class_name[..last_dot];
    let name = &class_name[last_dot + 1..];
    format!("{}::{}", package, name)
  } else {
    class_name.to_string()
  }
}

fn run_android_components() -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found; run 'rkg index' first".to_string())?;

  let components = rkg_db::lookup_android_components_for_repository(&connection, repository.id)
    .map_err(|e| e.to_string())?;

  if components.is_empty() {
    println!("No Android components found. Run 'rkg index' first.");
    return Ok(());
  }

  println!(
    "{:<15} {:<30} {:<50} {:<30} {:<50}",
    "TYPE", "NAME", "CLASS NAME", "PERMISSION", "INTENT FILTERS"
  );
  println!("{}", "-".repeat(175));

  for c in components {
    let perm = c.permission.as_deref().unwrap_or("-");
    let mut filter_parts = Vec::new();
    if !c.intent_actions.is_empty() {
      filter_parts.push(format!("actions: {}", c.intent_actions.join(",")));
    }
    if !c.intent_categories.is_empty() {
      filter_parts.push(format!("categories: {}", c.intent_categories.join(",")));
    }
    let filter_str = if filter_parts.is_empty() {
      "-".to_string()
    } else {
      filter_parts.join("; ")
    };

    println!(
      "{:<15} {:<30} {:<50} {:<30} {:<50}",
      c.component_type, c.name, c.class_name, perm, filter_str
    );
  }

  Ok(())
}

fn get_resource_references(
  connection: &rusqlite::Connection,
  resource_qname: &str,
) -> rusqlite::Result<Vec<(String, String)>> {
  let mut stmt = connection.prepare(
    "SELECT DISTINCT s_src.qualified_name, f.path
     FROM edges e
     INNER JOIN symbols s_tgt ON e.target_symbol_id = s_tgt.id
     INNER JOIN symbols s_src ON e.source_symbol_id = s_src.id
     INNER JOIN files f ON s_src.file_id = f.id
     WHERE s_tgt.qualified_name = ?1
     ORDER BY s_src.qualified_name ASC",
  )?;
  let rows = stmt.query_map([resource_qname], |row| {
    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
  })?;

  let mut refs = Vec::new();
  for r in rows {
    refs.push(r?);
  }
  Ok(refs)
}

fn run_android_resources() -> Result<(), String> {
  let db_path = default_db_path()?;
  let connection = open_db_connection(&db_path)?;

  let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let repo_root = detect_repo_root(&current_dir).map_err(|e| e.to_string())?;
  let repo_root_text = repo_root.to_string_lossy().to_string();
  let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "repository not found; run 'rkg index' first".to_string())?;

  let resources = rkg_db::lookup_android_resources_for_repository(&connection, repository.id)
    .map_err(|e| e.to_string())?;

  if resources.is_empty() {
    println!("No Android resources found. Run 'rkg index' first.");
    return Ok(());
  }

  println!(
    "{:<15} {:<30} {:<30} {:<60}",
    "TYPE", "NAME", "VALUE", "REFERENCES"
  );
  println!("{}", "-".repeat(135));

  for r in resources {
    let qname = format!("R.{}.{}", r.resource_type, r.name);
    let refs = get_resource_references(&connection, &qname).map_err(|e| e.to_string())?;

    let ref_strs: Vec<String> = refs
      .iter()
      .map(|(src_qname, path)| {
        let file_name = std::path::Path::new(path)
          .file_name()
          .and_then(|n| n.to_str())
          .unwrap_or(path);
        let sym_name = if let Some(idx) = src_qname.find("::") {
          &src_qname[idx + 2..]
        } else {
          src_qname
        };
        format!("{} ({})", file_name, sym_name)
      })
      .collect();
    let refs_display = if ref_strs.is_empty() {
      "-".to_string()
    } else {
      ref_strs.join(", ")
    };

    let val = r.value.as_deref().unwrap_or("-");

    println!(
      "{:<15} {:<30} {:<30} {:<60}",
      r.resource_type, r.name, val, refs_display
    );
  }

  Ok(())
}

#[derive(serde::Deserialize, serde::Serialize)]
struct Task {
  id: String,
  language: String,
  repo: String,
  target_symbol: String,
  expected_files: Vec<String>,
  expected_symbols: Vec<String>,
  description: String,
}

#[derive(serde::Serialize)]
struct TaskResult {
  id: String,
  language: String,
  description: String,
  indexing_latency_ms: u128,
  query_latency_ms: u128,
  baseline_tokens: usize,
  rkg_tokens: usize,
  token_reduction_pct: f64,
  file_precision: f64,
  file_recall: f64,
  file_f1: f64,
  symbol_precision: f64,
  symbol_recall: f64,
  symbol_f1: f64,
  success: bool,
}

#[derive(serde::Serialize)]
struct AggregateSummary {
  total_tasks: usize,
  successful_tasks: usize,
  average_token_reduction_pct: f64,
  average_file_precision: f64,
  average_file_recall: f64,
  average_file_f1: f64,
  average_symbol_precision: f64,
  average_symbol_recall: f64,
  average_symbol_f1: f64,
}

#[derive(serde::Serialize)]
struct BenchOutput {
  timestamp: String,
  summary: AggregateSummary,
  tasks: Vec<TaskResult>,
}

fn run_bench(
  config_path: Option<String>,
  json_output: bool,
  output_file: Option<String>,
) -> Result<(), String> {
  use std::path::{Path, PathBuf};
  use std::time::Instant;

  let workspace_root = find_workspace_root()
    .ok_or_else(|| "Could not locate workspace root containing SPEC.md and fixtures".to_string())?;

  let tasks_file = match config_path {
    Some(path) => PathBuf::from(path),
    None => workspace_root
      .join("fixtures")
      .join("benchmarks")
      .join("tasks.json"),
  };

  let tasks_content = fs::read_to_string(&tasks_file)
    .map_err(|e| format!("Failed to read tasks file at {}: {e}", tasks_file.display()))?;

  let tasks: Vec<Task> =
    serde_json::from_str(&tasks_content).map_err(|e| format!("Failed to parse tasks JSON: {e}"))?;

  struct RestoreCwd {
    original: std::path::PathBuf,
  }

  impl Drop for RestoreCwd {
    fn drop(&mut self) {
      let _ = std::env::set_current_dir(&self.original);
    }
  }

  let original_dir = std::env::current_dir().map_err(|e| e.to_string())?;
  let _cwd_guard = RestoreCwd {
    original: original_dir.clone(),
  };

  let mut task_results = Vec::new();

  for task in &tasks {
    let src_repo_dir = workspace_root
      .join("fixtures")
      .join("benchmarks")
      .join("repos")
      .join(&task.repo);
    if !src_repo_dir.exists() {
      return Err(format!(
        "Source repo directory does not exist: {}",
        src_repo_dir.display()
      ));
    }

    let temp_dir = tempfile::tempdir().map_err(|e| format!("Failed to create temp dir: {e}"))?;
    let temp_path = std::fs::canonicalize(temp_dir.path())
      .map_err(|e| format!("Failed to canonicalize temp path: {e}"))?;

    let _ = std::process::Command::new("git")
      .arg("init")
      .arg("-q")
      .arg(&temp_path)
      .status();

    copy_dir_all(&src_repo_dir, &temp_path)
      .map_err(|e| format!("Failed to copy source files: {e}"))?;

    std::env::set_current_dir(&temp_path)
      .map_err(|e| format!("Failed to change directory to temp path: {e}"))?;

    let db_path = temp_path.join(".rkg").join("rkg.db");
    ensure_db_initialized(&db_path)?;

    let indexing_start = Instant::now();
    run_index(false, false)?;
    let indexing_latency = indexing_start.elapsed().as_millis();

    let connection = open_db_connection(&db_path)?;
    let repo_root_text = temp_path.to_string_lossy().to_string();
    let repository = rkg_db::lookup_repository_by_root_path(&connection, &repo_root_text)
      .map_err(|e| e.to_string())?
      .ok_or_else(|| "Repository not found in DB".to_string())?;

    let query_start = Instant::now();
    let pack = rkg_query::pack_context(
      &connection,
      repository.id,
      &temp_path,
      &task.target_symbol,
      None,
      "json",
    )
    .map_err(|e| format!("Failed to pack context: {e}"))?;
    let query_latency = query_start.elapsed().as_millis();

    let json_pack_str = rkg_query::format_context_pack_json(&pack, &temp_path);
    let parsed_pack: serde_json::Value = serde_json::from_str(&json_pack_str)
      .map_err(|e| format!("Failed to parse query output: {e}"))?;

    let rkg_tokens = parsed_pack["estimated_tokens"].as_u64().unwrap_or(0) as usize;

    let mut retrieved_files = Vec::new();
    if let Some(files_array) = parsed_pack["files"].as_array() {
      for f in files_array {
        if let Some(path) = f["path"].as_str() {
          retrieved_files.push(path.to_string());
        }
      }
    }

    let mut retrieved_symbols = Vec::new();
    if let Some(syms_array) = parsed_pack["symbols"].as_array() {
      for s in syms_array {
        if let Some(qname) = s["qualified_name"].as_str() {
          retrieved_symbols.push(qname.to_string());
        }
      }
    }

    std::env::set_current_dir(&original_dir).map_err(|e| e.to_string())?;

    let (baseline_files, baseline_tokens) = run_grep_baseline(&temp_path, &task.target_symbol)?;
    let _ = baseline_files; // Suppress unused warning

    let file_metrics = compute_set_metrics(&retrieved_files, &task.expected_files);
    let symbol_metrics = compute_set_metrics(&retrieved_symbols, &task.expected_symbols);

    let token_reduction = if baseline_tokens > 0 {
      ((baseline_tokens as f64 - rkg_tokens as f64) / baseline_tokens as f64) * 100.0
    } else {
      0.0
    };

    let expected_files_retrieved = task
      .expected_files
      .iter()
      .all(|ef| retrieved_files.contains(ef));
    let expected_symbols_retrieved = task
      .expected_symbols
      .iter()
      .all(|es| retrieved_symbols.contains(es));
    let success = expected_files_retrieved && expected_symbols_retrieved;

    task_results.push(TaskResult {
      id: task.id.clone(),
      language: task.language.clone(),
      description: task.description.clone(),
      indexing_latency_ms: indexing_latency,
      query_latency_ms: query_latency,
      baseline_tokens,
      rkg_tokens,
      token_reduction_pct: token_reduction,
      file_precision: file_metrics.precision,
      file_recall: file_metrics.recall,
      file_f1: file_metrics.f1,
      symbol_precision: symbol_metrics.precision,
      symbol_recall: symbol_metrics.recall,
      symbol_f1: symbol_metrics.f1,
      success,
    });
  }

  std::env::set_current_dir(&original_dir).map_err(|e| e.to_string())?;

  let total_tasks = task_results.len();
  if total_tasks == 0 {
    return Err("No tasks loaded to benchmark".to_string());
  }

  let successful_tasks = task_results.iter().filter(|r| r.success).count();
  let average_token_reduction = task_results
    .iter()
    .map(|r| r.token_reduction_pct)
    .sum::<f64>()
    / total_tasks as f64;
  let average_file_precision =
    task_results.iter().map(|r| r.file_precision).sum::<f64>() / total_tasks as f64;
  let average_file_recall =
    task_results.iter().map(|r| r.file_recall).sum::<f64>() / total_tasks as f64;
  let average_file_f1 = task_results.iter().map(|r| r.file_f1).sum::<f64>() / total_tasks as f64;
  let average_symbol_precision =
    task_results.iter().map(|r| r.symbol_precision).sum::<f64>() / total_tasks as f64;
  let average_symbol_recall =
    task_results.iter().map(|r| r.symbol_recall).sum::<f64>() / total_tasks as f64;
  let average_symbol_f1 =
    task_results.iter().map(|r| r.symbol_f1).sum::<f64>() / total_tasks as f64;

  let summary = AggregateSummary {
    total_tasks,
    successful_tasks,
    average_token_reduction_pct: average_token_reduction,
    average_file_precision,
    average_file_recall,
    average_file_f1,
    average_symbol_precision,
    average_symbol_recall,
    average_symbol_f1,
  };

  let timestamp = rusqlite::Connection::open_in_memory()
    .and_then(|conn| {
      conn.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%SZ', 'now')", [], |row| {
        row.get(0)
      })
    })
    .unwrap_or_else(|_| "2026-06-08T11:47:23-04:00".to_string());

  let output = BenchOutput {
    timestamp,
    summary,
    tasks: task_results,
  };

  let json_output_str = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;

  let mut md_output = String::new();
  md_output.push_str("# rkg Benchmark Summary\n\n");
  md_output.push_str(&format!(
    "* **Total Tasks**: {}\n",
    output.summary.total_tasks
  ));
  md_output.push_str(&format!(
    "* **Successful Tasks**: {} / {}\n",
    output.summary.successful_tasks, output.summary.total_tasks
  ));
  md_output.push_str(&format!(
    "* **Average Token Reduction**: {:.2}%\n",
    output.summary.average_token_reduction_pct
  ));
  md_output.push_str(&format!(
    "* **Average File Precision**: {:.3}\n",
    output.summary.average_file_precision
  ));
  md_output.push_str(&format!(
    "* **Average File Recall**: {:.3}\n",
    output.summary.average_file_recall
  ));
  md_output.push_str(&format!(
    "* **Average Symbol Precision**: {:.3}\n",
    output.summary.average_symbol_precision
  ));
  md_output.push_str(&format!(
    "* **Average Symbol Recall**: {:.3}\n\n",
    output.summary.average_symbol_recall
  ));

  md_output.push_str("| Language | Task ID | Description | Token Reduction | File Precision | File Recall | Symbol Precision | Symbol Recall | Success |\n");
  md_output.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");
  for t in &output.tasks {
    md_output.push_str(&format!(
      "| {} | {} | {} | {:.1}% | {:.2} | {:.2} | {:.2} | {:.2} | {} |\n",
      t.language,
      t.id,
      t.description,
      t.token_reduction_pct,
      t.file_precision,
      t.file_recall,
      t.symbol_precision,
      t.symbol_recall,
      if t.success { "Yes" } else { "No" }
    ));
  }

  if json_output {
    println!("{json_output_str}");
  } else {
    println!("{md_output}");
  }

  if let Some(out_path) = output_file {
    let p_path = Path::new(&out_path);
    if let Some(parent) = p_path.parent() {
      fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content_to_write = if out_path.ends_with(".json") {
      &json_output_str
    } else {
      &md_output
    };
    fs::write(p_path, content_to_write)
      .map_err(|e| format!("Failed to write output file at {out_path}: {e}"))?;
  }

  Ok(())
}

fn find_workspace_root() -> Option<PathBuf> {
  let mut dir = std::env::current_dir().ok()?;
  loop {
    if dir.join("SPEC.md").exists() && dir.join("fixtures").exists() {
      return Some(dir);
    }
    if !dir.pop() {
      break;
    }
  }
  None
}

fn copy_dir_all(
  src: impl AsRef<std::path::Path>,
  dst: impl AsRef<std::path::Path>,
) -> std::io::Result<()> {
  fs::create_dir_all(&dst)?;
  for entry in fs::read_dir(src)? {
    let entry = entry?;
    let ty = entry.file_type()?;
    if ty.is_dir() {
      copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
    } else {
      fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
    }
  }
  Ok(())
}

fn run_grep_baseline(
  repo_path: &std::path::Path,
  target_symbol: &str,
) -> Result<(Vec<String>, usize), String> {
  let unqualified_name = target_symbol
    .split("::")
    .last()
    .unwrap_or(target_symbol)
    .split('.')
    .next_back()
    .unwrap_or(target_symbol);

  let mut baseline_files = Vec::new();
  let mut total_tokens = 0;

  fn visit_dirs(
    dir: &std::path::Path,
    repo_path: &std::path::Path,
    unqualified_name: &str,
    baseline_files: &mut Vec<String>,
    total_tokens: &mut usize,
  ) -> std::io::Result<()> {
    if dir.is_dir() {
      for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
          let name = path.file_name().unwrap_or_default().to_string_lossy();
          if name == ".git" || name == ".rkg" || name == "target" {
            continue;
          }
          visit_dirs(
            &path,
            repo_path,
            unqualified_name,
            baseline_files,
            total_tokens,
          )?;
        } else {
          if let Ok(content) = fs::read_to_string(&path) {
            let found = is_word_in_text(unqualified_name, &content);
            if found {
              let relative_path = path
                .strip_prefix(repo_path)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
              baseline_files.push(relative_path);
              *total_tokens += rkg_query::estimate_tokens(&content);
            }
          }
        }
      }
    }
    Ok(())
  }

  visit_dirs(
    repo_path,
    repo_path,
    unqualified_name,
    &mut baseline_files,
    &mut total_tokens,
  )
  .map_err(|e| e.to_string())?;

  Ok((baseline_files, total_tokens))
}

fn is_word_in_text(word: &str, text: &str) -> bool {
  if word.is_empty() {
    return false;
  }
  let mut search_start = 0;
  while let Some(idx) = text[search_start..].find(word) {
    let absolute_idx = search_start + idx;
    let before_char = text[..absolute_idx].chars().next_back().unwrap_or(' ');
    let after_idx = absolute_idx + word.len();
    let after_char = text[after_idx..].chars().next().unwrap_or(' ');

    let before_ok = !before_char.is_alphanumeric() && before_char != '_';
    let after_ok = !after_char.is_alphanumeric() && after_char != '_';
    if before_ok && after_ok {
      return true;
    }
    if let Some(next_char) = text[absolute_idx..].chars().next() {
      search_start = absolute_idx + next_char.len_utf8();
    } else {
      break;
    }
  }
  false
}

struct SetMetrics {
  precision: f64,
  recall: f64,
  f1: f64,
}

fn compute_set_metrics(retrieved: &[String], expected: &[String]) -> SetMetrics {
  use std::collections::HashSet;
  let retrieved_set: HashSet<&String> = retrieved.iter().collect();
  let expected_set: HashSet<&String> = expected.iter().collect();

  if expected_set.is_empty() {
    return SetMetrics {
      precision: if retrieved_set.is_empty() { 1.0 } else { 0.0 },
      recall: 1.0,
      f1: if retrieved_set.is_empty() { 1.0 } else { 0.0 },
    };
  }
  if retrieved_set.is_empty() {
    return SetMetrics {
      precision: 0.0,
      recall: 0.0,
      f1: 0.0,
    };
  }

  let intersection_count = retrieved_set
    .iter()
    .filter(|r| expected_set.contains(*r))
    .count() as f64;
  let precision = intersection_count / retrieved_set.len() as f64;
  let recall = intersection_count / expected_set.len() as f64;
  let f1 = if precision + recall > 0.0 {
    2.0 * (precision * recall) / (precision + recall)
  } else {
    0.0
  };

  SetMetrics {
    precision,
    recall,
    f1,
  }
}

#[cfg(test)]
mod db_connection_tests {
  use super::*;

  #[test]
  fn open_db_connection_enables_foreign_keys() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = temp_dir.path().join(".rkg").join("rkg.db");
    let connection = open_db_connection(&db_path).expect("db connection should open");
    let enabled: i64 = connection
      .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
      .expect("pragma query should succeed");
    assert_eq!(enabled, 1);
  }

  #[test]
  fn open_db_connection_creates_parent_directory() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = temp_dir.path().join(".rkg").join("rkg.db");
    assert!(!db_path.parent().expect("parent should exist").exists());
    let _connection = open_db_connection(&db_path).expect("db connection should open");
    assert!(db_path.exists());
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn default_db_path_is_under_dot_rkg() {
    let db_path = default_db_path().expect("default db path should resolve");
    assert!(db_path.ends_with(".rkg/rkg.db"));
  }

  #[test]
  fn test_is_word_in_text() {
    assert!(is_word_in_text("run", "fn run()"));
    assert!(is_word_in_text("run", "let run = 1;"));
    assert!(!is_word_in_text("run", "fn running()"));
    assert!(!is_word_in_text("run", "fn _run()"));
    assert!(!is_word_in_text("run", "let prun = 2;"));
    assert!(is_word_in_text("helper", "utils.helper()"));
    assert!(is_word_in_text("helper", "let helper: String"));
  }

  #[test]
  fn test_compute_set_metrics() {
    let retrieved = vec!["a".to_string(), "b".to_string()];
    let expected = vec!["b".to_string(), "c".to_string()];
    let metrics = compute_set_metrics(&retrieved, &expected);
    assert_eq!(metrics.precision, 0.5);
    assert_eq!(metrics.recall, 0.5);
    assert_eq!(metrics.f1, 0.5);

    let empty_metrics = compute_set_metrics(&[], &[]);
    assert_eq!(empty_metrics.precision, 1.0);
    assert_eq!(empty_metrics.recall, 1.0);
    assert_eq!(empty_metrics.f1, 1.0);

    let only_expected = compute_set_metrics(&[], &["a".to_string()]);
    assert_eq!(only_expected.precision, 0.0);
    assert_eq!(only_expected.recall, 0.0);
    assert_eq!(only_expected.f1, 0.0);
  }
}
