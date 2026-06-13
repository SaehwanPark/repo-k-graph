use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn indexes_and_queries_jupyter_notebooks_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create jupyter notebook file with code cells
  let notebook_json = r##"{
  "cells": [
    {
      "cell_type": "code",
      "source": [
        "import math\n",
        "\n",
        "class NotebookModel:\n",
        "    def __init__(self, val):\n",
        "        self.val = val\n",
        "\n",
        "def compute_square_root(x):\n",
        "    return math.sqrt(x)\n",
        "\n",
        "res = compute_square_root(16)\n"
      ]
    },
    {
      "cell_type": "markdown",
      "source": [
        "# Some markdown that should be ignored\n"
      ]
    },
    {
      "cell_type": "code",
      "source": [
        "def test_compute_square_root():\n",
        "    assert compute_square_root(9) == 3.0\n"
      ]
    }
  ],
  "metadata": {},
  "nbformat": 4,
  "nbformat_minor": 2
}"##;

  write_file(temp_dir.path(), "src/notebook.ipynb", notebook_json);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("files scanned: 1"));
  assert!(stdout.contains("files changed: 1"));

  // 4. Test rkg symbols
  let mut symbols_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  symbols_cmd.current_dir(temp_dir.path()).arg("symbols");
  let sym_output = symbols_cmd.assert().success().get_output().stdout.clone();
  let sym_stdout = String::from_utf8(sym_output).expect("stdout should be valid utf8");
  println!("SYM STDOUT:\n{}", sym_stdout);
  assert!(sym_stdout.contains("src.notebook.ipynb#cell_0 [Module]"));
  assert!(sym_stdout.contains("src.notebook.ipynb#cell_0::NotebookModel [Class]"));
  assert!(sym_stdout.contains("src.notebook.ipynb#cell_0::NotebookModel.__init__ [Method]"));
  assert!(sym_stdout.contains("src.notebook.ipynb#cell_0::compute_square_root [Function]"));

  // 5. Test rkg find compute_square_root
  let mut find_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  find_cmd
    .current_dir(temp_dir.path())
    .args(["find", "compute_square_root"]);
  let find_output = find_cmd.assert().success().get_output().stdout.clone();
  let find_stdout = String::from_utf8(find_output).expect("stdout should be valid utf8");
  println!("FIND STDOUT:\n{}", find_stdout);
  assert!(find_stdout.contains(
    "src.notebook.ipynb#cell_0::compute_square_root [Function] (src/notebook.ipynb:7-8)"
  ));

  // 6. Test rkg show src.notebook.ipynb#cell_0::compute_square_root
  let mut show_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  show_cmd
    .current_dir(temp_dir.path())
    .args(["show", "src.notebook.ipynb#cell_0::compute_square_root"]);
  let show_output = show_cmd.assert().success().get_output().stdout.clone();
  let show_stdout = String::from_utf8(show_output).expect("stdout should be valid utf8");
  println!("SHOW STDOUT:\n{}", show_stdout);
  assert!(
    show_stdout.contains("Symbol: src.notebook.ipynb#cell_0::compute_square_root [Function]")
  );
  assert!(show_stdout.contains("File: src/notebook.ipynb (lines 7-8)"));
  assert!(show_stdout.contains("def compute_square_root(x):"));
  assert!(show_stdout.contains("    return math.sqrt(x)"));
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
