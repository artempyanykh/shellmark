use crate::bookmarks;
use crate::shell::{Output, OutputType};
use crate::storage;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

pub struct Diag {
    pub data_dir: PathBuf,
    pub bookmark_count: usize,
}

impl Display for Diag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Data directory: {}", self.data_dir.display())?;
        writeln!(f, "Bookmark count: {}", self.bookmark_count)
    }
}

impl Output for Diag {
    fn to_output(&self, _out_type: OutputType) -> Option<String> {
        Some(format!("{self}"))
    }
}

pub async fn diag_cmd() -> Result<Diag, Box<dyn Error>> {
    let data_dir = storage::get_or_create_data_dir().await?;
    let bookmarks = bookmarks::read_bookmarks().await?;
    let bookmark_count = bookmarks.len();
    Ok(Diag {
        data_dir,
        bookmark_count,
    })
}
