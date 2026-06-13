//! `rkg-cli` library surface — re-exports the Clap CLI definitions so the
//! `rkg-completions` binary can share them without duplicating struct definitions.

mod cli_def;

pub use cli_def::{AndroidCommands, Cli, Commands, DbCommands, McpCommands};
