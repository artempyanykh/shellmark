use std::env;

use crate::{
    bookmarks::{read_bookmarks, write_bookmarks, Bookmark},
    cli,
    storage::friendly_path,
};
use anyhow::Result;
use tokio::fs;
use tracing::{info, warn};

pub async fn add_cmd(add_cmd_opts: cli::AddCmd) -> Result<()> {
    let dest = match add_cmd_opts.dest {
        Some(path_str) => fs::canonicalize(&path_str).await?,
        None => env::current_dir()?,
    };
    let name = add_cmd_opts.name.unwrap_or_else(|| {
        // It's possible that the path is a root path (`/` or `C:\`) and file name N/A.
        // In this case just use dest's friendly path
        dest.file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| friendly_path(&dest))
    });
    let mut bookmarks = read_bookmarks().await?;
    let existing = bookmarks.iter().enumerate().find(|(_, bm)| bm.name == name);
    let should_update = match existing {
        None => {
            bookmarks.push(Bookmark::new(name.clone(), dest.clone()).into());
            true
        }
        Some((idx, existing)) => {
            if add_cmd_opts.force {
                bookmarks.remove(idx);
                bookmarks.push(Bookmark::new(name.clone(), dest.clone()).into());
                true
            } else {
                warn!(
                    "A bookmark with name {} already exists pointing at: {}",
                    existing.name,
                    friendly_path(&existing.dest)
                );
                info!("Consider using `--force` to replace the bookmark, or --name to give it a different name");
                false
            }
        }
    };

    if should_update {
        info!(
            "Added a bookmark {} pointing at {}",
            name,
            friendly_path(&dest)
        );
        write_bookmarks(&bookmarks).await?;
    }

    Ok(())
}
