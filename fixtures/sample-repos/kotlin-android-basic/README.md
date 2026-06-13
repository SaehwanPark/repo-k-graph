# Kotlin Android Basic Fixture

Small Android project fixture for manifest, resource, navigation, and Kotlin
handler linkage tests. The canonical CI coverage lives in
`crates/rkg-cli/tests/android_linkage.rs`, which mirrors this tree inline.

## Isolated manual smoke test

Because `rkg` discovers the repository root via ancestor `.git` lookup, initialize
a local git repo inside this fixture before indexing it in isolation:

```sh
cd fixtures/sample-repos/kotlin-android-basic
git init
cargo run -p rkg-cli -- init
cargo run -p rkg-cli -- index
cargo run -p rkg-cli -- android components
cargo run -p rkg-cli -- android resources
```

This fixture is indexing-only: it is not intended to build with the Android
Gradle Plugin. Drawable `ic_logo.png` is a placeholder file for path-based
resource discovery.

## Automated test

From the repository root:

```sh
cargo test -p rkg-cli android_linkage
```
