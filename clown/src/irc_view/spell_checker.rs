use crate::model::Model;
use clown_spell::dict;
use color_eyre::eyre::eyre;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use tracing::info;

pub struct SpellChecker {
    words: Option<dict::Dictionary>,
}
use std::{io::Write, path::PathBuf};
impl SpellChecker {
    async fn download_file(url: &str, to: &PathBuf) -> color_eyre::Result<PathBuf> {
        let mut response = reqwest::get(url).await?;
        let mut file = std::fs::File::create(to)?;

        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk)?;
        }
        Ok(to.to_path_buf())
    }

    async fn download_affix(language: &str) -> color_eyre::Result<PathBuf> {
        if let Some(dest) = Model::project_dir()
            .map(|proj_dirs| proj_dirs.data_dir().join(format!("{}.aff", language)))
        {
            info!("affix found here: {}", dest.clone().display());
            if dest.exists() {
                return Ok(dest);
            }
            let url = format!(
                "https://raw.githubusercontent.com/LibreOffice/dictionaries/refs/heads/master/{}_FR/{}.aff",
                language, language
            );
            SpellChecker::download_file(&url, &dest).await
        } else {
            Err(eyre!("Error downloading dict"))
        }
    }

    async fn download_dict(language: &str) -> color_eyre::Result<PathBuf> {
        if let Some(dest) = Model::project_dir()
            .map(|proj_dirs| proj_dirs.data_dir().join(format!("{}.dic", language)))
        {
            info!("dic found here: {}", dest.clone().display());

            if dest.exists() {
                return Ok(dest);
            }
            let url = format!(
                "https://raw.githubusercontent.com/LibreOffice/dictionaries/refs/heads/master/{}_FR/{}.dic",
                language, language
            );
            SpellChecker::download_file(&url, &dest).await
        } else {
            Err(eyre!("Error downloading dict"))
        }
    }

    pub async fn try_build(language: &str) -> color_eyre::Result<Self> {
        let dict = SpellChecker::download_dict(language).await?;
        let affix = SpellChecker::download_affix(language).await?;

        Ok(SpellChecker {
            words: Some(dict::Dictionary::try_build_from_path(&dict, &affix)?),
        })
    }

    pub fn async_build(language: &str) -> JoinHandle<color_eyre::Result<Self>> {
        let handle = Handle::current();
        let language = language.to_string();
        handle.spawn(async move { SpellChecker::try_build(&language).await })
    }
}
