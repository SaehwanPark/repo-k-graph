use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_swift_relationships_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create Swift source file
  let swift_content = r#"
    import Foundation
    import class UIKit.UIView

    @objc class PatientManager: NSObject {
      var patient: Patient
      var title: String = ""

      @discardableResult
      func updatePatient(id: String, age: Int, record: MedicalRecord) -> Bool {
        let result = record.verify()
        patient.displayInfo()
        return result
      }
    }

    struct MedicalRecord {
      func verify() -> Bool {
        return true
      }
    }

    struct Patient {
      func displayInfo() {}
    }
  "#;

  write_file(temp_dir.path(), "src/Patient.swift", swift_content);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("files scanned: 1"));

  // 4. Test imports command
  let mut imports_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  imports_cmd
    .current_dir(temp_dir.path())
    .args(["imports", "src/Patient.swift"]);
  let imports_output = imports_cmd.assert().success().get_output().stdout.clone();
  let imports_stdout = String::from_utf8(imports_output).expect("stdout should be valid utf8");
  assert!(imports_stdout.contains("Foundation"));
  assert!(imports_stdout.contains("UIKit.UIView"));

  // 5. Test callers command
  let mut callers_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  callers_cmd
    .current_dir(temp_dir.path())
    .args(["callers", "displayInfo"]);
  let callers_output = callers_cmd.assert().success().get_output().stdout.clone();
  let callers_stdout = String::from_utf8(callers_output).expect("stdout should be valid utf8");
  assert!(callers_stdout.contains("src.Patient::PatientManager.updatePatient"));

  // 6. Test callees command
  let mut callees_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  callees_cmd
    .current_dir(temp_dir.path())
    .args(["callees", "updatePatient"]);
  let callees_output = callees_cmd.assert().success().get_output().stdout.clone();
  let callees_stdout = String::from_utf8(callees_output).expect("stdout should be valid utf8");
  assert!(callees_stdout.contains("verify"));
  assert!(callees_stdout.contains("displayInfo"));

  // 7. Test types command
  let mut types_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  types_cmd
    .current_dir(temp_dir.path())
    .args(["types", "Patient"]);
  let types_output = types_cmd.assert().success().get_output().stdout.clone();
  let types_stdout = String::from_utf8(types_output).expect("stdout should be valid utf8");
  assert!(types_stdout.contains("src.Patient::PatientManager.patient"));
}

#[test]
fn extracts_and_queries_swift_ui_and_uikit_relationships_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create Swift source file representing SwiftUI / UIKit features
  let swift_content = r#"
    import Foundation

    struct MyView: View {
      @State var count: Int = 0
      @Binding var isActive: Bool
      @ObservedObject var model: MyModel
      @EnvironmentObject var settings: Settings

      var body: some View {
        VStack {
          Text("Count: \(count)")
          NavigationLink(destination: OtherView()) {
            Text("Go")
          }
          Button("Increment") {
            count += 1
          }
        }
      }
    }

    #Preview {
      MyView()
    }

    class MyViewController: UIViewController, UITableViewDelegate {
      @IBOutlet var myButton: UIButton!

      override func viewDidLoad() {
        super.viewDidLoad()
        let storyboard = UIStoryboard(name: "Main", bundle: nil)
        let vc = storyboard.instantiateViewController(identifier: "DetailVC")
        let nib = UINib(nibName: "CustomCell", bundle: nil)

        let nsStoryboard = NSStoryboard(name: "MacMain", bundle: nil)
        let nsVC = nsStoryboard.instantiateController(withIdentifier: "MacVC")
        let nsNib = NSNib(nibNamed: "MacCell", bundle: nil)
      }

      @IBAction func tapped(_ sender: Any) {}
    }
  "#;

  write_file(temp_dir.path(), "src/MyView.swift", swift_content);

  // 3. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  assert!(stdout.contains("files scanned: 1"));

  // 4. Test decorators query for count property
  let mut decs_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  decs_cmd
    .current_dir(temp_dir.path())
    .args(["decorators", "count"]);
  let decs_output = decs_cmd.assert().success().get_output().stdout.clone();
  let decs_stdout = String::from_utf8(decs_output).expect("stdout should be valid utf8");
  assert!(decs_stdout.contains("State"));

  // 5. Test decorators query for isActive property
  let mut decs_cmd_b = Command::cargo_bin("rkg").expect("rkg binary should compile");
  decs_cmd_b
    .current_dir(temp_dir.path())
    .args(["decorators", "isActive"]);
  let decs_output_b = decs_cmd_b.assert().success().get_output().stdout.clone();
  let decs_stdout_b = String::from_utf8(decs_output_b).expect("stdout should be valid utf8");
  assert!(decs_stdout_b.contains("Binding"));

  // 6. Test decorators query for myButton property
  let mut decs_cmd2 = Command::cargo_bin("rkg").expect("rkg binary should compile");
  decs_cmd2
    .current_dir(temp_dir.path())
    .args(["decorators", "myButton"]);
  let decs_output2 = decs_cmd2.assert().success().get_output().stdout.clone();
  let decs_stdout2 = String::from_utf8(decs_output2).expect("stdout should be valid utf8");
  assert!(decs_stdout2.contains("IBOutlet"));

  // 7. Test decorators query for tapped method
  let mut decs_cmd3 = Command::cargo_bin("rkg").expect("rkg binary should compile");
  decs_cmd3
    .current_dir(temp_dir.path())
    .args(["decorators", "tapped"]);
  let decs_output3 = decs_cmd3.assert().success().get_output().stdout.clone();
  let decs_stdout3 = String::from_utf8(decs_output3).expect("stdout should be valid utf8");
  assert!(decs_stdout3.contains("IBAction"));

  // 8. Test callers command for nested view
  let mut callers_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  callers_cmd
    .current_dir(temp_dir.path())
    .args(["callers", "VStack"]);
  let callers_output = callers_cmd.assert().success().get_output().stdout.clone();
  let callers_stdout = String::from_utf8(callers_output).expect("stdout should be valid utf8");
  assert!(callers_stdout.contains("src.MyView::MyView.body"));

  // 9. Test callers command for NavigationLink
  let mut callers_cmd_nav = Command::cargo_bin("rkg").expect("rkg binary should compile");
  callers_cmd_nav
    .current_dir(temp_dir.path())
    .args(["callers", "NavigationLink"]);
  let callers_output_nav = callers_cmd_nav
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let callers_stdout_nav =
    String::from_utf8(callers_output_nav).expect("stdout should be valid utf8");
  assert!(callers_stdout_nav.contains("src.MyView::MyView.body"));

  // 10. Test callers command for storyboard::Main
  let mut callers_cmd2 = Command::cargo_bin("rkg").expect("rkg binary should compile");
  callers_cmd2
    .current_dir(temp_dir.path())
    .args(["callers", "storyboard::Main"]);
  let callers_output2 = callers_cmd2.assert().success().get_output().stdout.clone();
  let callers_stdout2 = String::from_utf8(callers_output2).expect("stdout should be valid utf8");
  assert!(callers_stdout2.contains("src.MyView::MyViewController.viewDidLoad"));

  // 11. Test callers command for storyboard::MacMain (AppKit)
  let mut callers_cmd_mac = Command::cargo_bin("rkg").expect("rkg binary should compile");
  callers_cmd_mac
    .current_dir(temp_dir.path())
    .args(["callers", "storyboard::MacMain"]);
  let callers_output_mac = callers_cmd_mac
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let callers_stdout_mac =
    String::from_utf8(callers_output_mac).expect("stdout should be valid utf8");
  assert!(callers_stdout_mac.contains("src.MyView::MyViewController.viewDidLoad"));

  // 12. Test callees command for viewDidLoad (should list all UIKit and AppKit assets)
  let mut callees_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  callees_cmd
    .current_dir(temp_dir.path())
    .args(["callees", "viewDidLoad"]);
  let callees_output = callees_cmd.assert().success().get_output().stdout.clone();
  let callees_stdout = String::from_utf8(callees_output).expect("stdout should be valid utf8");
  assert!(callees_stdout.contains("storyboard::Main"));
  assert!(callees_stdout.contains("viewcontroller::DetailVC"));
  assert!(callees_stdout.contains("nib::CustomCell"));
  assert!(callees_stdout.contains("storyboard::MacMain"));
  assert!(callees_stdout.contains("viewcontroller::MacVC"));
  assert!(callees_stdout.contains("nib::MacCell"));

  // 13. Test types command for UITableViewDelegate (protocol conformance)
  let mut types_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  types_cmd
    .current_dir(temp_dir.path())
    .args(["types", "UITableViewDelegate"]);
  let types_output = types_cmd.assert().success().get_output().stdout.clone();
  let types_stdout = String::from_utf8(types_output).expect("stdout should be valid utf8");
  assert!(types_stdout.contains("src.MyView::MyViewController"));
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
