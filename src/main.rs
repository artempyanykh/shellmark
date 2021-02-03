mod add;
mod bookmarks;
mod browse;
mod cli;
mod search;
mod storage;

use anyhow::Result;
use clap::Clap;
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    execute, terminal,
};
use futures::stream::{self, TryStreamExt};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::{
    env,
    error::Error,
    io::{self, Stdout},
    iter::FromIterator,
    ops::Range,
    process::exit,
    sync::Arc,
    time::Duration,
    unreachable, usize,
};
use storage::friendly_path;
use terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use tokio::{
    fs::{self, File},
    time::Instant,
};
use tokio_stream::StreamExt;
use tracing::{info, warn, Level};
use tracing_subscriber::EnvFilter;
use tui::{backend::CrosstermBackend, Terminal};

use crate::add::add_cmd;
use crate::browse::browse_cmd;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let filter = EnvFilter::default().add_directive(Level::INFO.into());
    tracing_subscriber::fmt().with_env_filter(filter).init();
    let opts = cli::Opts::parse();

    match opts.command {
        Some(cli::Command::Add(add_cmd_opts)) => add_cmd(add_cmd_opts).await?,
        Some(cli::Command::Browse(_)) | None => browse_cmd().await?,
    }

    Ok(())
}
