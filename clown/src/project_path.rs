use std::path::PathBuf;

use directories::ProjectDirs;

pub struct ProjectPath;

impl ProjectPath {
    pub fn project_dir() -> Option<ProjectDirs> {
        ProjectDirs::from("com", "share", "clown")
    }

    pub fn cache_dir() -> Option<PathBuf> {
        Self::project_dir().map(|v| v.cache_dir().to_path_buf())
    }

    pub fn log_name() -> &'static str {
        "app.log"
    }
}
