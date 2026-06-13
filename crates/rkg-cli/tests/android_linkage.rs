use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn extracts_and_queries_android_linkage_via_cli() {
  let temp_dir = TempDir::new().expect("temp dir must be created");
  setup_repo(temp_dir.path());

  // 1. Initialize DB
  let mut init_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  init_cmd.current_dir(temp_dir.path()).arg("init");
  init_cmd.assert().success();

  // 2. Create Android Manifest with comments and permissions
  let manifest = r#"
    <manifest xmlns:android="http://schemas.android.com/apk/res/android"
        package="com.example.app">
        <uses-permission android:name="android.permission.INTERNET" />
        <!-- <uses-permission android:name="android.permission.SEND_SMS" /> -->
        <application android:name=".MyApplication">
            <activity
                android:name=".MainActivity"
                android:exported="true">
                <intent-filter>
                    <action android:name="android.intent.action.MAIN" />
                    <category android:name="android.intent.category.LAUNCHER" />
                </intent-filter>
            </activity>
            <service android:name="com.example.app.MyService" android:permission="android.permission.BIND_JOB_SERVICE" />
            <!--
            <service android:name="com.example.app.IgnoredService" />
            -->
        </application>
    </manifest>
  "#;
  write_file(
    temp_dir.path(),
    "app/src/main/AndroidManifest.xml",
    manifest,
  );

  // 3. Create Android Layout
  let layout = r#"
    <LinearLayout xmlns:android="http://schemas.android.com/apk/res/android"
        android:layout_width="match_parent"
        android:layout_height="match_parent">
        <!-- <TextView android:id="@+id/ignored_text" /> -->
        <TextView
            android:id="@+id/text_title"
            android:layout_width="wrap_content"
            android:layout_height="wrap_content"
            android:text="@string/welcome_message" />
        <Button
            android:id="@id/btn_submit"
            android:layout_width="wrap_content"
            android:layout_height="wrap_content" />
    </LinearLayout>
  "#;
  write_file(
    temp_dir.path(),
    "app/src/main/res/layout/activity_main.xml",
    layout,
  );

  // Qualified layout directory to test B2 / C2
  let layout_land = r#"
    <LinearLayout xmlns:android="http://schemas.android.com/apk/res/android">
        <Button android:id="@+id/btn_land" />
    </LinearLayout>
  "#;
  write_file(
    temp_dir.path(),
    "app/src/main/res/layout-land/activity_main.xml",
    layout_land,
  );

  // XML Vector Drawable
  let xml_drawable = r#"
    <vector xmlns:android="http://schemas.android.com/apk/res/android"
        android:width="24dp"
        android:height="24dp" />
  "#;
  write_file(
    temp_dir.path(),
    "app/src/main/res/drawable/ic_launcher.xml",
    xml_drawable,
  );

  // PNG Drawable
  write_file(
    temp_dir.path(),
    "app/src/main/res/drawable-night/ic_logo.png",
    "mock png content",
  );

  // 4. Create Android Navigation Graph
  let nav = r#"
    <navigation xmlns:android="http://schemas.android.com/apk/res/android"
        xmlns:app="http://schemas.android.com/apk/res-auto"
        xmlns:tools="http://schemas.github.com/tools"
        android:id="@+id/nav_graph">
        <fragment
            android:id="@+id/navigation_home"
            android:name="com.example.app.ui.home.HomeFragment"
            tools:layout="@layout/fragment_home" />
    </navigation>
  "#;
  write_file(
    temp_dir.path(),
    "app/src/main/res/navigation/nav_graph.xml",
    nav,
  );

  // 5. Create Android Values
  let values = r#"
    <resources>
        <string name="app_name">My App</string>
        <string name="welcome_message">Welcome!</string>
    </resources>
  "#;
  write_file(
    temp_dir.path(),
    "app/src/main/res/values/strings.xml",
    values,
  );

  // 6. Create Kotlin Source Code referencing layout and drawables
  let kotlin_code = r#"
    package com.example.app

    import android.os.Bundle
    import androidx.appcompat.app.AppCompatActivity

    class MainActivity : AppCompatActivity() {
        override fun onCreate(savedInstanceState: Bundle?) {
            super.onCreate(savedInstanceState)
            setContentView(R.layout.activity_main)
            val button = findViewById<Button>(R.id.btn_submit)
            val launchIcon = R.drawable.ic_launcher
            val logoIcon = R.drawable.ic_logo
        }
    }
  "#;
  write_file(
    temp_dir.path(),
    "app/src/main/java/com/example/app/MainActivity.kt",
    kotlin_code,
  );

  // 7. Index repository
  let mut index_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  index_cmd.current_dir(temp_dir.path()).arg("index");
  let output = index_cmd.assert().success().get_output().stdout.clone();
  let stdout = String::from_utf8(output).expect("stdout should be valid utf8");
  println!("=== index stdout ===\n{}", stdout);

  // Verify files are parsed (AndroidManifest, activity_main, activity_main-land, ic_launcher, ic_logo, nav_graph, strings, MainActivity)
  assert!(stdout.contains("files scanned: 8"));

  // 8. Test Android components query command: rkg android components
  let mut comps_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  comps_cmd
    .current_dir(temp_dir.path())
    .args(["android", "components"]);
  let comps_output = comps_cmd.assert().success().get_output().stdout.clone();
  let comps_stdout = String::from_utf8(comps_output).expect("stdout should be valid utf8");
  println!("=== comps stdout ===\n{}", comps_stdout);

  // Verify application intent filters are NOT polluted by child activities (Major finding B8 / C3)
  let app_lines: Vec<_> = comps_stdout
    .lines()
    .filter(|l| l.contains("application") && l.contains("MyApplication"))
    .collect();
  assert!(!app_lines.is_empty());
  assert!(!app_lines[0].contains("android.intent.action.MAIN"));

  // Verify commented out components are NOT indexed
  assert!(!comps_stdout.contains("IgnoredService"));

  // Verify headers and component entries
  assert!(comps_stdout.contains("TYPE"));
  assert!(comps_stdout.contains("NAME"));
  assert!(comps_stdout.contains("CLASS NAME"));
  assert!(comps_stdout.contains("INTENT FILTERS"));

  assert!(comps_stdout.contains("application"));
  assert!(comps_stdout.contains("MyApplication"));
  assert!(comps_stdout.contains("com.example.app.MyApplication"));

  assert!(comps_stdout.contains("activity"));
  assert!(comps_stdout.contains("MainActivity"));
  assert!(comps_stdout.contains("com.example.app.MainActivity"));
  assert!(
    comps_stdout.contains(
      "actions: android.intent.action.MAIN; categories: android.intent.category.LAUNCHER"
    )
  );

  assert!(comps_stdout.contains("service"));
  assert!(comps_stdout.contains("MyService"));
  assert!(comps_stdout.contains("com.example.app.MyService"));
  assert!(comps_stdout.contains("android.permission.BIND_JOB_SERVICE"));

  // 9. Test Android resources query command: rkg android resources
  let mut res_cmd = Command::cargo_bin("rkg").expect("rkg binary should compile");
  res_cmd
    .current_dir(temp_dir.path())
    .args(["android", "resources"]);
  let res_output = res_cmd.assert().success().get_output().stdout.clone();
  let res_stdout = String::from_utf8(res_output).expect("stdout should be valid utf8");
  println!("=== res stdout ===\n{}", res_stdout);

  // Verify headers and resource entries
  assert!(res_stdout.contains("TYPE"));
  assert!(res_stdout.contains("NAME"));
  assert!(res_stdout.contains("REFERENCES"));

  // activity_main layout should be referenced by MainActivity.kt's onCreate
  assert!(res_stdout.contains("layout"));
  assert!(res_stdout.contains("activity_main"));
  assert!(res_stdout.contains("MainActivity.kt (MainActivity.onCreate)"));

  // btn_submit id should be referenced by MainActivity.kt's onCreate
  assert!(res_stdout.contains("id"));
  assert!(res_stdout.contains("btn_submit"));

  // btn_land in qualified land directory layout should be indexed (B2 / C2)
  assert!(res_stdout.contains("btn_land"));

  // welcome_message string should be referenced by activity_main layout
  assert!(res_stdout.contains("string"));
  assert!(res_stdout.contains("welcome_message"));
  assert!(res_stdout.contains("activity_main.xml (R.layout.activity_main)"));

  // app_name string is not referenced
  assert!(res_stdout.contains("app_name"));

  // permissions should be registered as resources (B6)
  assert!(res_stdout.contains("permission"));
  assert!(res_stdout.contains("android.permission.INTERNET"));
  // Commented out permission should NOT be indexed
  assert!(!res_stdout.contains("android.permission.SEND_SMS"));

  // Drawables should be indexed and resolve references (B7)
  assert!(res_stdout.contains("drawable"));
  assert!(res_stdout.contains("ic_launcher"));
  assert!(res_stdout.contains("ic_logo"));
  let occurrences = res_stdout
    .matches("MainActivity.kt (MainActivity.onCreate)")
    .count();
  // ic_launcher, ic_logo, and btn_submit all resolve to MainActivity.onCreate
  assert!(occurrences >= 2);
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
