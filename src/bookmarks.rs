use anyhow::{Context, Result};
use std::{
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::fs::{self, OpenOptions};

use crate::storage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bookmark {
    pub name: String,
    pub dest: PathBuf,
}

impl Bookmark {
    pub fn new(name: String, dest: PathBuf) -> Bookmark {
        Bookmark { name, dest }
    }
}

async fn get_or_create_bookmarks_file(data_dir: &Path) -> Result<PathBuf> {
    let bookmarks_file = data_dir.join("bookmarks.json");
    if !bookmarks_file.exists() {
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(&bookmarks_file)
            .await
            .context("Couldn't open/create a bookmarks file. Please, check access rights.")?;
    }
    Ok(bookmarks_file)
}

pub async fn read_bookmarks() -> Result<Vec<Arc<Bookmark>>> {
    let project_dir = storage::get_or_create_data_dir().await?;
    let bookmarks_file = get_or_create_bookmarks_file(&project_dir).await?;
    read_bookmarks_intern(&bookmarks_file)
        .await
        .map(|v| v.into_iter().map(Arc::new).collect())
}

pub async fn write_bookmarks(bookmarks: &[Arc<Bookmark>]) -> Result<()> {
    let project_dir = storage::get_or_create_data_dir().await?;
    let bookmarks_file = get_or_create_bookmarks_file(&project_dir).await?;
    write_bookmarks_intern(&bookmarks_file, bookmarks).await
}

async fn read_bookmarks_intern(bookmarks_file: &Path) -> Result<Vec<Bookmark>> {
    let content = fs::read_to_string(bookmarks_file)
        .await
        .with_context(|| format!("Couldn't read bookmarks file: {}", bookmarks_file.display()))?;

    if content.trim().is_empty() {
        Ok(Vec::new())
    } else {
        serde_json::from_str(&content).context("Couldn't parse bookmarks JSON")
    }
}

async fn write_bookmarks_intern(bookmarks_file: &Path, bookmarks: &[Arc<Bookmark>]) -> Result<()> {
    let content =
        serde_json::to_string_pretty(&bookmarks.iter().map(Arc::as_ref).collect::<Vec<_>>())
            .context("Couldn't serialize bookmarks to JSON")?;
    Ok(fs::write(bookmarks_file, content).await?)
}
