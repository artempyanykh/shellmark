mod add;
mod bookmarks;
mod browse;
mod cli;
mod search;
mod storage;

use anyhow::Result;
use clap::Clap;

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

    match opts.command {
        Some(cli::Command::Add(add_cmd_opts)) => add_cmd(add_cmd_opts).await?,
        Some(cli::Command::Browse(_)) => browse_cmd(opts.out_type).await?,
        None => browse_cmd(opts.out_type).await?,
    }

    Ok(())
}
