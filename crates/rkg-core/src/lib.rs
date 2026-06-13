#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
  pub path: String,
  pub language: Option<String>,
  pub hash: Option<String>,
  pub line_count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
  pub name: String,
  pub qualified_name: String,
  pub kind: SymbolKind,
  pub location: Location,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
  Function,
  Method,
  Class,
  Trait,
  Struct,
  Enum,
  Module,
  Interface,
  TypeAlias,
  Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
  pub file_path: String,
  pub start_line: usize,
  pub end_line: usize,
  pub start_column: Option<usize>,
  pub end_column: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
  pub source: String,
  pub target: String,
  pub kind: EdgeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeKind {
  Imports,
  Calls,
  Defines,
  Implements,
  Extends,
  ReferencesType,
  TestedBy,
  DocumentedBy,
  ConfiguredBy,
  ModifiedWith,
  Spawns,
  SendsTo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocBlock {
  pub title: Option<String>,
  pub text: String,
  pub location: Location,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCase {
  pub name: String,
  pub location: Location,
  pub target_symbols: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextPack {
  pub target: Option<String>,
  pub files: Vec<File>,
  pub symbols: Vec<Symbol>,
  pub edges: Vec<Edge>,
  pub docs: Vec<DocBlock>,
  pub tests: Vec<TestCase>,
  pub token_budget: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCommitInfo {
  pub hash: String,
  pub author_name: String,
  pub author_email: String,
  pub date: String,
  pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitFileMetadata {
  pub path: String,
  pub churn: usize,
  pub last_commit: Option<GitCommitInfo>,
  pub author_frequency: Vec<(String, usize)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CochangeRecord {
  pub name: String,
  pub count: usize,
  pub rate: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CochangeAnalysis {
  pub target: String,
  pub churn: usize,
  pub symbol_cochanges: Vec<CochangeRecord>,
  pub file_cochanges: Vec<CochangeRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
  pub handler_name: String,
  pub qualified_name: String,
  pub method: String,
  pub path: String,
  pub response_model: Option<String>,
  pub location: Location,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PydanticField {
  pub name: String,
  pub type_annotation: String,
  pub is_required: bool,
  pub default_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PydanticValidator {
  pub name: String,
  pub validator_type: String, // "field" or "model"
  pub target_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PydanticModel {
  pub name: String,
  pub qualified_name: String,
  pub fields: Vec<PydanticField>,
  pub validators: Vec<PydanticValidator>,
  pub location: Location,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoPackage {
  pub id: i64,
  pub repository_id: i64,
  pub name: String,
  pub manifest_path: String,
  pub version: String,
  pub is_workspace_member: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoDependency {
  pub id: i64,
  pub package_id: i64,
  pub name: String,
  pub version_requirement: Option<String>,
  pub is_workspace_dependency: bool,
  pub features: Vec<String>,
  pub is_dev: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FSharpProject {
  pub id: i64,
  pub repository_id: i64,
  pub name: String,
  pub project_path: String,
  pub target_framework: Option<String>,
  pub is_solution_member: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FSharpDependency {
  pub id: i64,
  pub project_id: i64,
  pub name: String,
  pub dependency_type: String, // "package" (NuGet/Paket) or "project" (ProjectReference)
  pub version_requirement: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KotlinProject {
  pub id: i64,
  pub repository_id: i64,
  pub name: String,
  pub project_path: String,
  pub target_framework: Option<String>,
  pub is_solution_member: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KotlinDependency {
  pub id: i64,
  pub project_id: i64,
  pub name: String,
  pub dependency_type: String, // "package" (Maven/Gradle dependency) or "project" (subproject reference)
  pub version_requirement: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwiftProject {
  pub id: i64,
  pub repository_id: i64,
  pub name: String,
  pub project_path: String,
  pub target_framework: Option<String>,
  pub is_solution_member: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwiftDependency {
  pub id: i64,
  pub project_id: i64,
  pub name: String,
  pub dependency_type: String, // "package" (SPM external package) or "project" (local path dependency)
  pub version_requirement: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcurrencySpawn {
  pub source_symbol_qualified_name: String,
  pub spawn_kind: String,
  pub target_name: Option<String>,
  pub location: Location,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcurrencyChannel {
  pub source_symbol_qualified_name: String,
  pub channel_kind: String,
  pub tx_name: String,
  pub rx_name: String,
  pub location: Location,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcurrencySelect {
  pub source_symbol_qualified_name: String,
  pub location: Location,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustUnsafeBlock {
  pub source_symbol_qualified_name: String,
  pub location: Location,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustUnsafeFunction {
  pub qualified_name: String,
  pub location: Location,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustFFIBinding {
  pub source_symbol_qualified_name: String,
  pub foreign_item_name: String,
  pub abi: String,
  pub location: Location,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SafetyProfile {
  pub target_name: String,
  pub unsafe_blocks: Vec<RustUnsafeBlock>,
  pub unsafe_functions: Vec<RustUnsafeFunction>,
  pub ffi_bindings: Vec<RustFFIBinding>,
  pub safety_score: u32,
  pub risk_level: String,
  pub safe_wrapper_percentage: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestSuiteCoverage {
  pub test_suite: Option<String>,
  pub report_path: String,
  pub lines_valid: usize,
  pub lines_covered: usize,
  pub branches_valid: usize,
  pub branches_covered: usize,
  pub uncovered_lines: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoverageProfile {
  pub target_name: String,
  pub is_file: bool,
  pub lines_valid: usize,
  pub lines_covered: usize,
  pub branches_valid: usize,
  pub branches_covered: usize,
  pub uncovered_lines: Vec<usize>,
  pub test_suites: Vec<TestSuiteCoverage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndroidComponent {
  pub id: i64,
  pub file_id: i64,
  pub name: String,
  pub component_type: String, // "activity", "service", "receiver", "provider", "application"
  pub class_name: String,
  pub permission: Option<String>,
  pub intent_actions: Vec<String>,
  pub intent_categories: Vec<String>,
  pub location: Location,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndroidResource {
  pub id: i64,
  pub file_id: i64,
  pub name: String,
  pub resource_type: String, // "layout", "string", "id", "navigation", "dimen", "color", "drawable"
  pub value: Option<String>,
  pub location: Location,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn android_component_and_resource_records() {
    let component = AndroidComponent {
      id: 1,
      file_id: 2,
      name: "MainActivity".to_string(),
      component_type: "activity".to_string(),
      class_name: "com.example.app.MainActivity".to_string(),
      permission: Some("android.permission.BIND_REMOTEVIEWS".to_string()),
      intent_actions: vec!["android.intent.action.MAIN".to_string()],
      intent_categories: vec!["android.intent.category.LAUNCHER".to_string()],
      location: sample_location(),
    };

    let resource = AndroidResource {
      id: 3,
      file_id: 2,
      name: "activity_main".to_string(),
      resource_type: "layout".to_string(),
      value: None,
      location: sample_location(),
    };

    assert_eq!(component.name, "MainActivity");
    assert_eq!(component.component_type, "activity");
    assert_eq!(component.intent_actions[0], "android.intent.action.MAIN");
    assert_eq!(resource.name, "activity_main");
    assert_eq!(resource.resource_type, "layout");
  }

  fn sample_location() -> Location {
    Location {
      file_path: "src/example.py".to_string(),
      start_line: 1,
      end_line: 3,
      start_column: Some(0),
      end_column: Some(12),
    }
  }

  #[test]
  fn symbol_records_name_kind_and_location() {
    let symbol = Symbol {
      name: "validate_patient".to_string(),
      qualified_name: "src.example::validate_patient".to_string(),
      kind: SymbolKind::Function,
      location: sample_location(),
    };

    assert_eq!(symbol.name, "validate_patient");
    assert_eq!(symbol.kind, SymbolKind::Function);
    assert_eq!(symbol.location.file_path, "src/example.py");
  }

  #[test]
  fn edge_records_deterministic_relationship() {
    let edge = Edge {
      source: "tests/test_example.py::test_validate_patient".to_string(),
      target: "src.example::validate_patient".to_string(),
      kind: EdgeKind::TestedBy,
    };

    assert_eq!(edge.kind, EdgeKind::TestedBy);
    assert_eq!(edge.target, "src.example::validate_patient");
  }

  #[test]
  fn edge_records_concurrency_relationships() {
    let spawn_edge = Edge {
      source: "src.main::run_server".to_string(),
      target: "src.worker::worker_loop".to_string(),
      kind: EdgeKind::Spawns,
    };
    let send_edge = Edge {
      source: "src.worker::worker_loop".to_string(),
      target: "src.main::run_server".to_string(),
      kind: EdgeKind::SendsTo,
    };

    assert_eq!(spawn_edge.kind, EdgeKind::Spawns);
    assert_eq!(send_edge.kind, EdgeKind::SendsTo);
  }

  #[test]
  fn context_pack_groups_related_repository_facts() {
    let file = File {
      path: "src/example.py".to_string(),
      language: Some("python".to_string()),
      hash: None,
      line_count: Some(3),
    };
    let symbol = Symbol {
      name: "validate_patient".to_string(),
      qualified_name: "src.example::validate_patient".to_string(),
      kind: SymbolKind::Function,
      location: sample_location(),
    };
    let doc = DocBlock {
      title: Some("Validation".to_string()),
      text: "Validate patient records before export.".to_string(),
      location: sample_location(),
    };
    let test = TestCase {
      name: "test_validate_patient".to_string(),
      location: sample_location(),
      target_symbols: vec!["src.example::validate_patient".to_string()],
    };

    let pack = ContextPack {
      target: Some("src.example::validate_patient".to_string()),
      files: vec![file],
      symbols: vec![symbol],
      edges: Vec::new(),
      docs: vec![doc],
      tests: vec![test],
      token_budget: Some(2_000),
    };

    assert_eq!(pack.files.len(), 1);
    assert_eq!(pack.symbols.len(), 1);
    assert_eq!(pack.docs.len(), 1);
    assert_eq!(pack.tests.len(), 1);
    assert_eq!(pack.token_budget, Some(2_000));
  }

  #[test]
  fn route_records_properties() {
    let route = Route {
      handler_name: "read_item".to_string(),
      qualified_name: "src.api::read_item".to_string(),
      method: "GET".to_string(),
      path: "/items/{item_id}".to_string(),
      response_model: Some("ItemResponse".to_string()),
      location: sample_location(),
    };

    assert_eq!(route.handler_name, "read_item");
    assert_eq!(route.method, "GET");
    assert_eq!(route.path, "/items/{item_id}");
    assert_eq!(route.response_model, Some("ItemResponse".to_string()));
  }

  #[test]
  fn pydantic_records_properties() {
    let field = PydanticField {
      name: "age".to_string(),
      type_annotation: "int".to_string(),
      is_required: false,
      default_value: Some("30".to_string()),
    };

    let validator = PydanticValidator {
      name: "check_age".to_string(),
      validator_type: "field".to_string(),
      target_fields: vec!["age".to_string()],
    };

    let model = PydanticModel {
      name: "Patient".to_string(),
      qualified_name: "src.models::Patient".to_string(),
      fields: vec![field],
      validators: vec![validator],
      location: sample_location(),
    };

    assert_eq!(model.name, "Patient");
    assert_eq!(model.fields.len(), 1);
    assert_eq!(model.fields[0].name, "age");
    assert_eq!(model.validators.len(), 1);
    assert_eq!(model.validators[0].name, "check_age");
  }

  #[test]
  fn cargo_package_and_dependency_records() {
    let pkg = CargoPackage {
      id: 1,
      repository_id: 10,
      name: "rkg-core".to_string(),
      manifest_path: "crates/rkg-core/Cargo.toml".to_string(),
      version: "0.1.0".to_string(),
      is_workspace_member: true,
    };

    let dep = CargoDependency {
      id: 2,
      package_id: 1,
      name: "tokio".to_string(),
      version_requirement: Some("1.0".to_string()),
      is_workspace_dependency: false,
      features: vec!["full".to_string(), "sync".to_string()],
      is_dev: false,
    };

    assert_eq!(pkg.name, "rkg-core");
    assert!(pkg.is_workspace_member);
    assert_eq!(dep.name, "tokio");
    assert_eq!(dep.features.len(), 2);
    assert!(!dep.is_workspace_dependency);
  }

  #[test]
  fn fsharp_project_and_dependency_records() {
    let proj = FSharpProject {
      id: 1,
      repository_id: 10,
      name: "MyLib".to_string(),
      project_path: "src/MyLib/MyLib.fsproj".to_string(),
      target_framework: Some("net8.0".to_string()),
      is_solution_member: true,
    };

    let dep = FSharpDependency {
      id: 2,
      project_id: 1,
      name: "Newtonsoft.Json".to_string(),
      dependency_type: "package".to_string(),
      version_requirement: Some("13.0.3".to_string()),
    };

    assert_eq!(proj.name, "MyLib");
    assert!(proj.is_solution_member);
    assert_eq!(proj.target_framework.as_deref(), Some("net8.0"));
    assert_eq!(dep.name, "Newtonsoft.Json");
    assert_eq!(dep.dependency_type, "package");
  }

  #[test]
  fn swift_project_and_dependency_records() {
    let proj = SwiftProject {
      id: 1,
      repository_id: 10,
      name: "MySwiftPackage".to_string(),
      project_path: "Package.swift".to_string(),
      target_framework: Some("5.9".to_string()),
      is_solution_member: true,
    };

    let dep = SwiftDependency {
      id: 2,
      project_id: 1,
      name: "swift-algorithms".to_string(),
      dependency_type: "package".to_string(),
      version_requirement: Some("1.2.0".to_string()),
    };

    assert_eq!(proj.name, "MySwiftPackage");
    assert!(proj.is_solution_member);
    assert_eq!(proj.target_framework.as_deref(), Some("5.9"));
    assert_eq!(dep.name, "swift-algorithms");
    assert_eq!(dep.dependency_type, "package");
  }

  #[test]
  fn test_concurrency_records() {
    let spawn = ConcurrencySpawn {
      source_symbol_qualified_name: "src::main::start".to_string(),
      spawn_kind: "tokio::spawn".to_string(),
      target_name: Some("worker".to_string()),
      location: sample_location(),
    };

    let channel = ConcurrencyChannel {
      source_symbol_qualified_name: "src::main::start".to_string(),
      channel_kind: "mpsc".to_string(),
      tx_name: "tx".to_string(),
      rx_name: "rx".to_string(),
      location: sample_location(),
    };

    let select = ConcurrencySelect {
      source_symbol_qualified_name: "src::main::start".to_string(),
      location: sample_location(),
    };

    assert_eq!(spawn.spawn_kind, "tokio::spawn");
    assert_eq!(channel.channel_kind, "mpsc");
    assert_eq!(select.source_symbol_qualified_name, "src::main::start");
  }

  #[test]
  fn test_safety_records() {
    let block = RustUnsafeBlock {
      source_symbol_qualified_name: "src::lib::my_func".to_string(),
      location: sample_location(),
    };

    let func = RustUnsafeFunction {
      qualified_name: "src::lib::my_unsafe_func".to_string(),
      location: sample_location(),
    };

    let ffi = RustFFIBinding {
      source_symbol_qualified_name: "src::lib::ffi".to_string(),
      foreign_item_name: "my_c_func".to_string(),
      abi: "C".to_string(),
      location: sample_location(),
    };

    let profile = SafetyProfile {
      target_name: "src::lib::my_func".to_string(),
      unsafe_blocks: vec![block],
      unsafe_functions: vec![func],
      ffi_bindings: vec![ffi],
      safety_score: 80,
      risk_level: "Medium".to_string(),
      safe_wrapper_percentage: 100.0,
    };

    assert_eq!(profile.unsafe_blocks.len(), 1);
    assert_eq!(profile.unsafe_functions.len(), 1);
    assert_eq!(profile.ffi_bindings.len(), 1);
    assert_eq!(profile.safety_score, 80);
    assert_eq!(profile.risk_level, "Medium");
    assert_eq!(profile.safe_wrapper_percentage, 100.0);
  }

  #[test]
  fn test_coverage_records() {
    let suite = TestSuiteCoverage {
      test_suite: Some("unit".to_string()),
      report_path: "coverage.xml".to_string(),
      lines_valid: 10,
      lines_covered: 8,
      branches_valid: 4,
      branches_covered: 3,
      uncovered_lines: vec![12, 13],
    };

    let profile = CoverageProfile {
      target_name: "src/lib.rs".to_string(),
      is_file: true,
      lines_valid: 10,
      lines_covered: 8,
      branches_valid: 4,
      branches_covered: 3,
      uncovered_lines: vec![12, 13],
      test_suites: vec![suite],
    };

    assert_eq!(profile.lines_covered, 8);
    assert_eq!(profile.test_suites.len(), 1);
    assert_eq!(profile.test_suites[0].test_suite, Some("unit".to_string()));
  }
}
