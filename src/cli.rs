use clap::{crate_version, Clap};

#[derive(Clap)]
#[clap(version = crate_version!())]
/// Cross-platform CLI bookmarks manager.
pub struct Opts {
    #[clap(subcommand)]
    pub command: Option<Command>,
}

#[derive(Clap)]
pub enum Command {
    /// (alias: a) Add bookmarks
    Add(AddCmd),
    /// (default, alias: b) Interactively find and select bookmarks
    Browse(BrowseCmd),
}

#[derive(Clap)]
#[clap(alias = "a")]
pub struct AddCmd {
    #[clap(short, long)]
    /// Replace the bookmark's destination when similarly named bookmark exists
    pub force: bool,
    /// Path to the destination file or directory (default: current directory)
    pub dest: Option<String>,
    /// Name of the bookmark (default: the name of the destination)
    #[clap(short, long)]
    pub name: Option<String>,
}

#[derive(Clap)]
#[clap(alias = "b")]
pub struct BrowseCmd {}
