use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_rust_routes_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create Rust code representing an Axum and Actix-web app
  let api_code = r#"
    use axum::{extract::{Path, State}, Json};
    use actix_web::{get, post, web, Responder};

    #[derive(Clone)]
    pub struct AppState {
      pub db: String,
    }

    pub struct UserPayload {
      pub username: String,
    }

    pub struct UserResponse {
      pub id: u64,
      pub username: String,
    }

    pub struct ItemPath {
      pub id: u32,
    }

    #[get("/items/{id}")]
    pub async fn get_item(
      state: web::Data<AppState>,
      path: web::Path<ItemPath>,
      raw_id: Path<u32>,
      tuple_path: web::Path<(String, u32)>,
    ) -> impl Responder {
      "item"
    }

    pub async fn create_user(
      State(state): State<AppState>,
      Json(payload): Json<UserPayload>,
    ) -> Json<UserResponse> {
      Json(UserResponse { id: 1, username: payload.username })
    }

    pub async fn list_users(
      State(state): State<AppState>,
    ) -> Json<Vec<UserResponse>> {
      Json(vec![])
    }

    pub fn app_router() {
      let app = Router::new()
        .route("/users", post(create_user).get(list_users))
        .route("/legacy", web::get().to(get_item));
    }
  "#;
  write_file(temp_dir.path(), "src/lib.rs", api_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("indexed repository:"));

  // 4. Test routes query command: rkg routes
  let mut routes_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  routes_cmd.current_dir(temp_dir.path()).arg("routes");
  let routes_output = routes_cmd.assert().success().get_output().stdout.clone();
  let routes_stdout = String::from_utf8(routes_output).expect("stdout should be valid utf8");

  // Verify headers are present
  assert!(routes_stdout.contains("METHOD"));
  assert!(routes_stdout.contains("PATH"));
  assert!(routes_stdout.contains("HANDLER"));
  assert!(routes_stdout.contains("RESPONSE MODEL"));
  assert!(routes_stdout.contains("DEPENDENCIES"));

  // Verify Actix route details
  assert!(routes_stdout.contains("GET"));
  assert!(routes_stdout.contains("/items/{id}"));
  assert!(routes_stdout.contains("src::lib::get_item"));
  assert!(routes_stdout.contains("AppState"));
  assert!(routes_stdout.contains("ItemPath"));
  assert!(!routes_stdout.contains("u32"));
  assert!(!routes_stdout.contains("(String, u32)"));

  // Verify Axum route details
  assert!(routes_stdout.contains("POST"));
  assert!(routes_stdout.contains("/users"));
  assert!(routes_stdout.contains("src::lib::create_user"));
  assert!(routes_stdout.contains("UserResponse"));
  assert!(routes_stdout.contains("UserPayload"));
}

fn setup_repo(root: &std::path::Path) {
  fs::create_dir(root.join(".git")).expect(".git directory should be created");
}

fn write_file(root: &std::path::Path, relative_path: &str, content: &str) {
  let path = root.join(relative_path);
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).expect("parent directories should be created");
  }
  fs::write(path, content).expect("file should be written");
}
