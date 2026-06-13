use assert_cmd::Command;
use rusqlite::Connection;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_kotlin_provenance_and_annotation_resolution() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Write settings.gradle.kts
  let settings_content = r#"
    rootProject.name = "MyKotlinProject"
    include(":app")
  "#;
  write_file(temp_dir.path(), "settings.gradle.kts", settings_content);

  // 2. Write app/build.gradle.kts with custom generated directories
  let app_build_content = r#"
    plugins {
        kotlin("jvm") version "1.9.0"
    }
    
    kotlin {
        sourceSets {
            main {
                kotlin.srcDir("build/generated/ksp/main/kotlin")
            }
        }
    }
  "#;
  write_file(temp_dir.path(), "app/build.gradle.kts", app_build_content);

  // 3. Write source files with annotations
  let source_content = r#"
    package com.example.app

    import kotlinx.serialization.Serializable
    import androidx.room.Dao
    import com.squareup.moshi.JsonClass

    @Serializable
    class User(val id: Int)

    @Dao
    interface UserDao {
        fun get(): User
    }

    @JsonClass(generateAdapter = true)
    class Customer(val id: Int)
  "#;
  write_file(
    temp_dir.path(),
    "app/src/main/kotlin/com/example/app/App.kt",
    source_content,
  );

  // 4. Write mock generated code files under the configured generated source directory
  let generated_serializer = r#"
    package com.example.app
    class UserSerializer
  "#;
  write_file(
    temp_dir.path(),
    "app/build/generated/ksp/main/kotlin/com/example/app/UserSerializer.kt",
    generated_serializer,
  );

  let generated_dao = r#"
    package com.example.app
    class UserDao_Impl : UserDao {
        override fun get(): User = User(1)
    }
  "#;
  write_file(
    temp_dir.path(),
    "app/build/generated/ksp/main/kotlin/com/example/app/UserDao_Impl.kt",
    generated_dao,
  );

  let generated_adapter = r#"
    package com.example.app
    class CustomerJsonAdapter
  "#;
  write_file(
    temp_dir.path(),
    "app/build/generated/ksp/main/kotlin/com/example/app/CustomerJsonAdapter.kt",
    generated_adapter,
  );

  // 5. Initialize database
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 6. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  index_cmd.assert().success();

  // 7. Verify via show command (checks run_show printing)
  let mut show_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  show_cmd
    .current_dir(temp_dir.path())
    .args(["show", "com.example.app::UserSerializer"]);
  let show_output = show_cmd.assert().success().get_output().stdout.clone();
  let show_stdout = String::from_utf8(show_output).expect("stdout should be valid UTF8");
  assert!(show_stdout.contains("Provenance: ksp"));

  // 8. Directly query database to verify annotation linkages (edges) and provenance records
  let db_path = temp_dir.path().join(".rkg/rkg.db");
  let conn = Connection::open(db_path).expect("must be able to open db");

  // Check generated_symbols table has all 3 symbols marked as 'ksp'
  let mut stmt = conn
    .prepare(
      "SELECT s.qualified_name, gs.provenance
     FROM generated_symbols gs
     INNER JOIN symbols s ON s.id = gs.symbol_id",
    )
    .expect("stmt prepare must succeed");
  let gen_rows = stmt
    .query_map([], |row| {
      Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })
    .expect("query map must succeed");

  let mut gen_map = std::collections::HashMap::new();
  for r in gen_rows {
    let (qname, prov) = r.unwrap();
    gen_map.insert(qname, prov);
  }

  assert_eq!(
    gen_map
      .get("com.example.app::UserSerializer")
      .map(|s| s.as_str()),
    Some("ksp")
  );
  assert_eq!(
    gen_map
      .get("com.example.app::UserDao_Impl")
      .map(|s| s.as_str()),
    Some("ksp")
  );
  assert_eq!(
    gen_map
      .get("com.example.app::CustomerJsonAdapter")
      .map(|s| s.as_str()),
    Some("ksp")
  );

  // Check resolved edges back to annotation classes
  // 1. UserSerializer -> User (ConfiguredBy)
  let serializer_linked: bool = conn
    .query_row(
      "SELECT EXISTS(
      SELECT 1 FROM edges e
      INNER JOIN symbols s1 ON e.source_symbol_id = s1.id
      INNER JOIN symbols s2 ON e.target_symbol_id = s2.id
      WHERE s1.qualified_name = 'com.example.app::UserSerializer'
        AND s2.qualified_name = 'com.example.app::User'
        AND e.kind = 'ConfiguredBy'
    )",
      [],
      |row| row.get(0),
    )
    .unwrap();
  assert!(serializer_linked);

  // 2. UserDao_Impl -> UserDao (Implements)
  let dao_linked: bool = conn
    .query_row(
      "SELECT EXISTS(
      SELECT 1 FROM edges e
      INNER JOIN symbols s1 ON e.source_symbol_id = s1.id
      INNER JOIN symbols s2 ON e.target_symbol_id = s2.id
      WHERE s1.qualified_name = 'com.example.app::UserDao_Impl'
        AND s2.qualified_name = 'com.example.app::UserDao'
        AND e.kind = 'Implements'
    )",
      [],
      |row| row.get(0),
    )
    .unwrap();
  assert!(dao_linked);

  // 3. CustomerJsonAdapter -> Customer (ConfiguredBy)
  let adapter_linked: bool = conn
    .query_row(
      "SELECT EXISTS(
      SELECT 1 FROM edges e
      INNER JOIN symbols s1 ON e.source_symbol_id = s1.id
      INNER JOIN symbols s2 ON e.target_symbol_id = s2.id
      WHERE s1.qualified_name = 'com.example.app::CustomerJsonAdapter'
        AND s2.qualified_name = 'com.example.app::Customer'
        AND e.kind = 'ConfiguredBy'
    )",
      [],
      |row| row.get(0),
    )
    .unwrap();
  assert!(adapter_linked);
}

#[test]
fn test_kotlin_multi_source_flow_and_collection() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // Write build.gradle.kts
  write_file(
    temp_dir.path(),
    "build.gradle.kts",
    r#"
    plugins {
        kotlin("jvm") version "1.9.0"
    }
  "#,
  );

  // Write multi-source flows and collection code
  let flow_code = r#"
    package com.example.flow

    import kotlinx.coroutines.flow.*

    fun left(): Flow<Int> = flowOf(1)
    fun right(): Flow<Int> = flowOf(2)
    fun combined(): Flow<Int> = left().combine(right()) { a, b -> a + b }

    suspend fun runCollect() {
        combined().collectLatest { println(it) }
    }
  "#;
  write_file(
    temp_dir.path(),
    "src/main/kotlin/com/example/flow/App.kt",
    flow_code,
  );

  // Initialize and index
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  index_cmd.assert().success();

  // Directly check DB for SendsTo edges representing multi-source and collection dataflow
  let db_path = temp_dir.path().join(".rkg/rkg.db");
  let conn = Connection::open(db_path).expect("must be able to open db");

  // Check left -> combined SendsTo
  let left_to_combined: bool = conn
    .query_row(
      "SELECT EXISTS(
      SELECT 1 FROM edges e
      INNER JOIN symbols s1 ON e.source_symbol_id = s1.id
      INNER JOIN symbols s2 ON e.target_symbol_id = s2.id
      WHERE s1.qualified_name = 'com.example.flow::left'
        AND s2.qualified_name = 'com.example.flow::combined'
        AND e.kind = 'SendsTo'
    )",
      [],
      |row| row.get(0),
    )
    .unwrap();
  assert!(left_to_combined);

  // Check right -> combined SendsTo
  let right_to_combined: bool = conn
    .query_row(
      "SELECT EXISTS(
      SELECT 1 FROM edges e
      INNER JOIN symbols s1 ON e.source_symbol_id = s1.id
      INNER JOIN symbols s2 ON e.target_symbol_id = s2.id
      WHERE s1.qualified_name = 'com.example.flow::right'
        AND s2.qualified_name = 'com.example.flow::combined'
        AND e.kind = 'SendsTo'
    )",
      [],
      |row| row.get(0),
    )
    .unwrap();
  assert!(right_to_combined);

  // Check combined -> runCollect SendsTo (via collectLatest)
  let combined_to_run: bool = conn
    .query_row(
      "SELECT EXISTS(
      SELECT 1 FROM edges e
      INNER JOIN symbols s1 ON e.source_symbol_id = s1.id
      INNER JOIN symbols s2 ON e.target_symbol_id = s2.id
      WHERE s1.qualified_name = 'com.example.flow::combined'
        AND s2.qualified_name = 'com.example.flow::runCollect'
        AND e.kind = 'SendsTo'
    )",
      [],
      |row| row.get(0),
    )
    .unwrap();
  assert!(combined_to_run);
}

fn setup_repo(root: &std::path::Path) {
  fs::create_dir_all(root.join(".git")).expect(".git directory should be created");
}

fn write_file(root: &std::path::Path, relative_path: &str, content: &str) {
  let path = root.join(relative_path);
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).expect("parent directories should be created");
  }
  fs::write(path, content).expect("file should be written");
}
