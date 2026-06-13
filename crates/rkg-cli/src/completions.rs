//! Standalone binary that generates shell completions for the `rkg` CLI.
//!
//! Usage:
//! ```text
//! rkg-completions <bash|zsh|fish|elvish|powershell>
//! ```

use clap::CommandFactory;
use clap_complete::{Shell, generate};
use rkg_cli::Cli;
use std::io;
use std::str::FromStr;

fn main() {
  let shell_name = std::env::args().nth(1).unwrap_or_else(|| {
    eprintln!("Usage: rkg-completions <bash|zsh|fish|elvish|powershell>");
    std::process::exit(1);
  });

  let shell = Shell::from_str(&shell_name).unwrap_or_else(|_| {
    eprintln!("Unknown shell: '{shell_name}'. Supported: bash, zsh, fish, elvish, powershell");
    std::process::exit(1);
  });

  let mut cmd = Cli::command();
  generate(shell, &mut cmd, "rkg", &mut io::stdout());
}
