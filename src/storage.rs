use std::path::{Path, PathBuf};

use directories::{ProjectDirs, UserDirs};
use once_cell::sync::Lazy;
use tokio::fs;
use tracing::info;

use anyhow::{bail, Context, Result};

static USER_DIRS: Lazy<UserDirs> = Lazy::new(|| {
    UserDirs::new().expect("Couldn't locate HOME. Please, make sure the shell is properly set up")
});
static PROJECT_DIRS: Lazy<ProjectDirs> = Lazy::new(|| {
    ProjectDirs::from("one", "arr", "shellmark")
        .expect("Couldn't locate HOME. Please, make sure the shell is properly set up")
});

pub fn friendly_path(path: &Path) -> String {
    // Strip out the "extended filename" prefix on Windows
    // https://docs.microsoft.com/en-us/windows/win32/fileio/naming-a-file#win32-file-namespaces
    let path = simplify_path(path);

    let home = USER_DIRS.home_dir();
    let home_rel_path = path.strip_prefix(home).unwrap_or(&path);
    let friendly_name = if home_rel_path.is_relative() {
        PathBuf::from("~")
            .join(home_rel_path)
            .to_string_lossy()
            .to_string()
    } else {
        home_rel_path.to_string_lossy().to_string()
    };
    friendly_name
}

#[cfg(target_os = "unix")]
pub fn simplify_path(path: &Path) -> &Path {
    path
}

#[cfg(target_os = "windows")]
pub fn simplify_path(path: &Path) -> &Path {
    dunce::simplified(path)
}

pub async fn get_or_create_data_dir() -> Result<PathBuf> {
    let proj_dirs = &PROJECT_DIRS;
    let data_local_dir = proj_dirs.data_local_dir();

    if let Err(code) = fs::metadata(data_local_dir).await.map_err(|err| err.kind()) {
        match code {
            std::io::ErrorKind::NotFound => {
                info!(
                    "Creating a data folder for shellmark at: {}",
                    friendly_path(data_local_dir)
                );

                fs::create_dir_all(data_local_dir).await.context(
                    "Couldn't create a data folder for shellmark. Please, check the access rights.",
                )?;

                info!("Successfully created the data folder!");
            }
            _ => {
                bail!("Couldn't access app's data dir. Please, check the access rights.");
            }
        }
    }

    Ok(data_local_dir.to_path_buf())
}
