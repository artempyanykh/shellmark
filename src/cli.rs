use clap::{crate_version, Clap};

#[derive(Clap)]
#[clap(version = crate_version!())]
/// Cross-platform CLI bookmarks manager.
pub struct Opts {
    #[clap(subcommand)]
    pub command: Option<Command>,
    #[clap(short = 'o', long = "out", possible_values = OUT_TYPES_STR, default_value = OutType::Plain.to_str())]
    /// Output selection as plain data or as evalable command for one of the shells
    pub out_type: OutType,
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

#[derive(Clap, Default)]
#[clap(alias = "b")]
pub struct BrowseCmd {}

const OUT_TYPES_STR: &'static [&'static str] = &["plain", "posix", "powershell"];

#[derive(Clap)]
pub enum OutType {
    Plain,
    Posix,
    PowerShell,
}

impl Default for OutType {
    fn default() -> Self {
        OutType::Plain
    }
}

impl OutType {
    const fn to_str(&self) -> &'static str {
        use OutType::*;

        match self {
            Plain => "plain",
            Posix => "posix",
            PowerShell => "powershell",
        }
    }
}

impl std::str::FromStr for OutType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use OutType::*;
        match s {
            "plain" => Ok(Plain),
            "posix" => Ok(Posix),
            "powershell" => Ok(PowerShell),
            _ => Err(format!(
                "Unexpected out: {}. Possible values are: {}",
                s,
                OUT_TYPES_STR.join(", "),
            )),
        }
    }
}
