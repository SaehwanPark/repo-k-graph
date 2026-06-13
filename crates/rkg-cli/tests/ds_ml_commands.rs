use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_ds_ml_elements_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create python files representing PyTorch module and Pandas pipeline
  let model_code = r#"
import torch.nn as nn
import torch

class ConvBlock(nn.Module):
  pass

class MyNet(nn.Module):
  def __init__(self):
    super(MyNet, self).__init__()
    self.conv = ConvBlock()
    self.fc = nn.Linear(10, 2)

  def forward(self, x: torch.Tensor[batch, 3, 224, 224]):
    # shape: (batch, channels, height, width)
    y = self.conv(x)
    y = self.fc(y)
    batch, channels, height, width = y.shape
    return y
"#;
  write_file(temp_dir.path(), "src/model.py", model_code);

  let pipeline_code = r#"
import polars as pl

def clean_data(df):
  df = df.with_columns(total_revenue = pl.col("revenue") * 1.1)
  df = df.select(pl.col("revenue").alias("rev"))
  df = df.rename({"revenue": "rev"})
  df["rev"] = df["revenue"]
  return df.select(
    pl.col("user_id"),
    "clicks",
    'revenue'
  ).filter(pl.col("user_id") > 0)
"#;
  write_file(temp_dir.path(), "src/pipeline.py", pipeline_code);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("python files parsed: 2"));

  // 4. Verify PyTorch submodules are indexed as symbols
  // Query for the fc submodule symbol: rkg find fc
  let mut find_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  find_cmd.current_dir(temp_dir.path()).args(["find", "fc"]);
  let find_output = find_cmd.assert().success().get_output().stdout.clone();
  let find_stdout = String::from_utf8(find_output).expect("stdout should be valid utf8");
  assert!(find_stdout.contains("src.model::MyNet.fc"));

  // 5. Verify forward call graph: forward method calls conv and fc
  // Query callers of fc: rkg callers fc
  let mut callers_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  callers_cmd
    .current_dir(temp_dir.path())
    .args(["callers", "fc"]);
  let callers_output = callers_cmd.assert().success().get_output().stdout.clone();
  let callers_stdout = String::from_utf8(callers_output).expect("stdout should be valid utf8");
  assert!(callers_stdout.contains("src.model::MyNet.forward"));

  // 6. Verify PyTorch submodule layer type references: MyNet.fc references nn.Linear
  // Query type references of fc: rkg types fc
  let mut types_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  types_cmd.current_dir(temp_dir.path()).args(["types", "fc"]);
  let types_output = types_cmd.assert().success().get_output().stdout.clone();
  let types_stdout = String::from_utf8(types_output).expect("stdout should be valid utf8");
  assert!(types_stdout.contains("nn.Linear [Unresolved]"));

  // Query type references of conv: MyNet.conv references ConvBlock (resolved!)
  let mut conv_types_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  conv_types_cmd
    .current_dir(temp_dir.path())
    .args(["types", "conv"]);
  let conv_output = conv_types_cmd
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let conv_stdout = String::from_utf8(conv_output).expect("stdout should be valid utf8");
  assert!(conv_stdout.contains("src.model::ConvBlock [Resolved] [Class]"));

  // 7. Verify Pandas/Polars column lineages
  // Query what process or function references the user_id column: rkg types user_id
  let mut col_types_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  col_types_cmd
    .current_dir(temp_dir.path())
    .args(["types", "user_id"]);
  let col_output = col_types_cmd.assert().success().get_output().stdout.clone();
  let col_stdout = String::from_utf8(col_output).expect("stdout should be valid utf8");
  assert!(col_stdout.contains("Symbols referencing type user_id:"));
  assert!(col_stdout.contains("src.pipeline::clean_data"));

  // Query what process or function references the revenue column: rkg types col::revenue
  let mut col_rev_types_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  col_rev_types_cmd
    .current_dir(temp_dir.path())
    .args(["types", "col::revenue"]);
  let col_rev_output = col_rev_types_cmd
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let col_rev_stdout = String::from_utf8(col_rev_output).expect("stdout should be valid utf8");
  assert!(col_rev_stdout.contains("Symbols referencing type col::revenue:"));
  assert!(col_rev_stdout.contains("src.pipeline::clean_data"));

  // 8. Verify shape hints query via CLI
  // Forward query on MyNet.forward
  let mut forward_types_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  forward_types_cmd
    .current_dir(temp_dir.path())
    .args(["types", "src.model::MyNet.forward"]);
  let forward_types_output = forward_types_cmd
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let forward_types_stdout =
    String::from_utf8(forward_types_output).expect("stdout should be valid utf8");
  assert!(forward_types_stdout.contains("shape::x::[batch, 3, 224, 224]"));
  assert!(forward_types_stdout.contains("shape::y::[batch, channels, height, width]"));

  // Backward query on shape::x
  let mut shape_x_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  shape_x_cmd
    .current_dir(temp_dir.path())
    .args(["types", "shape::x"]);
  let shape_x_output = shape_x_cmd.assert().success().get_output().stdout.clone();
  let shape_x_stdout = String::from_utf8(shape_x_output).expect("stdout should be valid utf8");
  assert!(shape_x_stdout.contains("src.model::MyNet.forward"));

  // 9. Verify column lineage query via CLI
  // Forward query on clean_data
  let mut pipeline_types_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  pipeline_types_cmd
    .current_dir(temp_dir.path())
    .args(["types", "src.pipeline::clean_data"]);
  let pipeline_types_output = pipeline_types_cmd
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let pipeline_types_stdout =
    String::from_utf8(pipeline_types_output).expect("stdout should be valid utf8");
  assert!(pipeline_types_stdout.contains("col::total_revenue <- with_columns(col::revenue)"));
  assert!(pipeline_types_stdout.contains("col::rev <- select(col::revenue)"));

  // Backward query on col::total_revenue
  let mut col_lineage_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  col_lineage_cmd
    .current_dir(temp_dir.path())
    .args(["types", "col::total_revenue"]);
  let col_lineage_output = col_lineage_cmd
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let col_lineage_stdout =
    String::from_utf8(col_lineage_output).expect("stdout should be valid utf8");
  assert!(col_lineage_stdout.contains("src.pipeline::clean_data"));
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
