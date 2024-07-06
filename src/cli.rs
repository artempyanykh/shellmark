use crate::shell::OutputType;
use clap::{crate_version, Parser};

#[derive(Parser)]
#[command(version = crate_version!())]
/// Cross-platform CLI bookmarks manager.
pub struct Opts {
    #[command(subcommand)]
    pub command: Option<Command>,
    #[arg(short = 'o', long = "out", value_enum, default_value_t = OutputType::Plain)]
    /// Output result as plain text or as eval-able command for one of the shells
    pub out_type: OutputType,
}

#[derive(Parser)]
pub enum Command {
    /// (alias: a) Add bookmarks
    Add(AddCmd),
    /// (default, alias: b) Interactively find and select bookmarks
    Browse(BrowseCmd),
    /// Output a command string to integrate shellmark into the shell
    Plug(PlugCmd),
    /// Print storage location and other diagnostics
    Diag(DiagCmd),
}

#[derive(Parser)]
#[command(alias = "a")]
pub struct AddCmd {
    #[arg(short, long)]
    /// Replace the bookmark's destination when similarly named bookmark exists
    pub force: bool,
    /// Path to the destination file or directory (default: current directory)
    pub dest: Option<String>,
    /// Name of the bookmark (default: the name of the destination)
    #[arg(short, long)]
    pub name: Option<String>,
}

#[derive(Parser, Default)]
#[command(alias = "b")]
pub struct BrowseCmd {}

#[derive(Parser)]
pub struct PlugCmd {
    #[arg(short, long, default_value = "s")]
    /// Name of the shell alias
    pub name: String,
}

#[derive(Parser)]
pub struct DiagCmd {}
