use crate::{
    cli,
    shell::{self, OutputType},
};
pub struct PlugCommand {
    name: String,
}

impl shell::Output for PlugCommand {
    fn to_output(&self, out_type: OutputType) -> Option<String> {
        let content = inner_content(out_type);
        content.map(|x| x.replace("{name}", &self.name))
    }
}

pub fn plug_cmd(opts: cli::PlugCmd) -> PlugCommand {
    PlugCommand { name: opts.name }
}

#[cfg(not(target_os = "windows"))]
fn inner_content(out_type: OutputType) -> Option<&'static str> {
    match out_type {
        OutputType::Plain => None,
        OutputType::Fish => Some(include_str!("../integration/s.fish")),
        OutputType::PowerShell => Some(include_str!("../integration/s.ps1")),
        OutputType::Posix => Some(include_str!("../integration/s.sh")),
    }
}

#[cfg(target_os = "windows")]
fn inner_content(out_type: OutputType) -> Option<&'static str> {
    match out_type {
        OutputType::Plain => None,
        OutputType::Fish => Some(include_str!("..\\integration\\s.fish")),
        OutputType::PowerShell => Some(include_str!("..\\integration\\s.ps1")),
        OutputType::Posix => Some(include_str!("..\\integration\\s.sh")),
    }
}
