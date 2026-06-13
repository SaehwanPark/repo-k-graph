fn main() {
  let src_dir = std::path::Path::new("src");
  println!("cargo:rerun-if-changed=src/parser.c");
  println!("cargo:rerun-if-changed=src/scanner.c");
  cc::Build::new()
    .include(src_dir)
    .file(src_dir.join("parser.c"))
    .file(src_dir.join("scanner.c"))
    .warnings(false)
    .compile("tree-sitter-mojo");
}
