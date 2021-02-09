mod add;
mod bookmarks;
mod browse;
mod cli;
mod plug;
mod search;
mod shell;
mod storage;

use anyhow::Result;
use clap::Clap;
use plug::plug_cmd;
use shell::Output;

use std::default::Default;
use std::error::Error;

use tracing::Level;
use tracing_subscriber::EnvFilter;

use crate::add::add_cmd;
use crate::browse::browse_cmd;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let filter = EnvFilter::default().add_directive(Level::INFO.into());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
    let opts = cli::Opts::parse();

    let output = match opts.command {
        Some(cli::Command::Add(add_cmd_opts)) => {
            add_cmd(add_cmd_opts).await?.to_output(opts.out_type)
        }
        Some(cli::Command::Browse(_)) => browse_cmd().await?.to_output(opts.out_type),
        Some(cli::Command::Plug) => plug_cmd().to_output(opts.out_type),
        None => browse_cmd().await?.to_output(opts.out_type),
    };

    if !output.is_empty() {
        print!("{}", output);
    }

    Ok(())
}
