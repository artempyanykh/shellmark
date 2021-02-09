use clap::Clap;

pub trait Output {
    fn to_output(&self, out_type: OutputType) -> String;
}

impl Output for () {
    fn to_output(&self, _: OutputType) -> String {
        String::new()
    }
}

impl<A: Output> Output for Option<A> {
    fn to_output(&self, out_type: OutputType) -> String {
        match self {
            Some(out) => out.to_output(out_type),
            None => String::new(),
        }
    }
}

pub const OUTPUT_TYPES_STR: &[&str] = &["plain", "posix", "fish", "powershell"];

#[derive(Clap)]
pub enum OutputType {
    Plain,
    Posix,
    Fish,
    PowerShell,
}

impl Default for OutputType {
    fn default() -> Self {
        OutputType::Plain
    }
}

impl OutputType {
    pub const fn to_str(&self) -> &'static str {
        use OutputType::*;

        match self {
            Plain => "plain",
            Posix => "posix",
            Fish => "fish",
            PowerShell => "powershell",
        }
    }
}

impl std::str::FromStr for OutputType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use OutputType::*;
        match s {
            "plain" => Ok(Plain),
            "posix" => Ok(Posix),
            "fish" => Ok(Fish),
            "powershell" => Ok(PowerShell),
            _ => Err(format!(
                "Unexpected out: {}. Possible values are: {}",
                s,
                OUTPUT_TYPES_STR.join(", "),
            )),
        }
    }
}
