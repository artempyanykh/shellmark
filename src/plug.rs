use crate::shell::{self, OutputType};
pub struct PlugCommand {}

impl shell::Output for PlugCommand {
    #[cfg(not(target_os = "windows"))]
    fn to_output(&self, out_type: OutputType) -> String {
        match out_type {
            OutputType::Plain => String::new(),
            OutputType::Fish => include_str!("../integration/s.fish").to_string(),
            OutputType::PowerShell => include_str!("../integration/s.ps1").to_string(),
            OutputType::Posix => include_str!("../integration/s.sh").to_string(),
        }
    }

    #[cfg(target_os = "windows")]
    fn to_output(&self, out_type: OutputType) -> String {
        match out_type {
            OutputType::Plain => String::new(),
            OutputType::Fish => include_str!("..\\integration\\s.fish").to_string(),
            OutputType::PowerShell => include_str!("..\\integration\\s.ps1").to_string(),
            OutputType::Posix => include_str!("..\\integration\\s.sh").to_string(),
        }
    }
}

pub fn plug_cmd() -> PlugCommand {
    PlugCommand {}
}
