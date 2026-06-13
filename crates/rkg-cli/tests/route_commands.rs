use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_routes_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create python files representing a FastAPI app
  let api_code = r#"
class ItemResponse:
  pass

def get_db():
  pass

@app.get("/items/{item_id}", response_model=ItemResponse)
def read_item(item_id: int, db: Session = Depends(get_db)):
  pass

@router.post("/items/")
def create_item(db: Session = Depends(get_db), current_user: User = Depends()):
  pass

@app.route("/legacy", methods=["GET", "POST"])
def legacy_endpoint():
  pass
"#;
  write_file(temp_dir.path(), "src/api.py", api_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("python files parsed: 1"));

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

  // Verify route 1 details
  assert!(routes_stdout.contains("GET"));
  assert!(routes_stdout.contains("/items/{item_id}"));
  assert!(routes_stdout.contains("src.api::read_item"));
  assert!(routes_stdout.contains("ItemResponse"));
  assert!(routes_stdout.contains("get_db"));

  // Verify route 2 details
  assert!(routes_stdout.contains("POST"));
  assert!(routes_stdout.contains("/items/"));
  assert!(routes_stdout.contains("src.api::create_item"));
  assert!(routes_stdout.contains("User"));

  // Verify legacy endpoint details (both GET and POST exist for /legacy)
  assert!(routes_stdout.contains("/legacy"));
  assert!(routes_stdout.contains("src.api::legacy_endpoint"));

  // Check that both lines are output:
  // "GET      /legacy                        src.api::legacy_endpoint"
  // "POST     /legacy                        src.api::legacy_endpoint"
  let lines: Vec<&str> = routes_stdout.lines().collect();
  let get_legacy_count = lines
    .iter()
    .filter(|l| {
      l.contains("GET") && l.contains("/legacy") && l.contains("src.api::legacy_endpoint")
    })
    .count();
  let post_legacy_count = lines
    .iter()
    .filter(|l| {
      l.contains("POST") && l.contains("/legacy") && l.contains("src.api::legacy_endpoint")
    })
    .count();
  assert_eq!(get_legacy_count, 1, "Should find GET /legacy");
  assert_eq!(post_legacy_count, 1, "Should find POST /legacy");
}

#[test]
fn extracts_and_queries_flask_routes_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create python files representing a Flask app
  let api_code = r#"
from flask import Blueprint, Flask

app = Flask(__name__)
auth_bp = Blueprint('auth', __name__, url_prefix='/auth')
admin_bp = Blueprint('admin', __name__, url_prefix='/admin/')

@app.route('/home', methods=['GET'])
def home():
  pass

@auth_bp.route('/login', methods=['GET', 'POST'])
def login():
  pass

@auth_bp.get('/logout')
def logout():
  pass

@admin_bp.route('/dashboard')
def dashboard():
  pass
"#;
  write_file(temp_dir.path(), "src/api.py", api_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  index_cmd.assert().success();

  // 4. Test routes query command: rkg routes
  let mut routes_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  routes_cmd.current_dir(temp_dir.path()).arg("routes");
  let routes_output = routes_cmd.assert().success().get_output().stdout.clone();
  let routes_stdout = String::from_utf8(routes_output).expect("stdout should be valid utf8");

  // Verify routes are present
  assert!(routes_stdout.contains("GET"));
  assert!(routes_stdout.contains("POST"));
  assert!(routes_stdout.contains("/home"));
  assert!(routes_stdout.contains("/auth/login"));
  assert!(routes_stdout.contains("/auth/logout"));
  assert!(routes_stdout.contains("/admin/dashboard"));

  assert!(routes_stdout.contains("src.api::home"));
  assert!(routes_stdout.contains("src.api::login"));
  assert!(routes_stdout.contains("src.api::logout"));
  assert!(routes_stdout.contains("src.api::dashboard"));
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
