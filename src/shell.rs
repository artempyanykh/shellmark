use std::env;

use clap::{Parser, ValueEnum};

pub trait Output {
    fn to_output(&self, out_type: OutputType) -> Option<String>;
}

impl Output for () {
    fn to_output(&self, _: OutputType) -> Option<String> {
        None
    }
}

impl<A: Output> Output for Option<A> {
    fn to_output(&self, out_type: OutputType) -> Option<String> {
        match self {
            Some(out) => out.to_output(out_type),
            _ => None,
        }
    }
}

#[derive(Parser, Default, Copy, Clone, ValueEnum)]
pub enum OutputType {
    #[default]
    Plain,
    Posix,
    Fish,
    #[clap(name = "powershell")]
    PowerShell,
}

pub(crate) fn is_editor_set() -> bool {
    match env::var("EDITOR") {
        Ok(editor) => !editor.is_empty(),
        _ => false,
    }
}
