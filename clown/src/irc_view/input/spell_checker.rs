use crate::project_path::ProjectPath;
use clown_spell::dict;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use tracing::info;

pub struct SpellChecker {
    dict: Option<dict::Dictionary>,
}
use std::{
    io::{Read, Write},
    path::PathBuf,
};

impl SpellChecker {
    async fn download_file(url: &str, to: &PathBuf) -> anyhow::Result<PathBuf> {
        let mut reader = ureq::get(url).call()?.into_body().into_reader();
        to.parent().map(std::fs::create_dir_all);
        let mut file = std::fs::File::create(to)?;
        let mut buf = [0u8; 4096];

        while let Ok(size) = reader.read(&mut buf[..]) {
            file.write_all(&buf)?;
        }
        Ok(to.to_path_buf())
    }

    async fn download_affix(language: &str) -> anyhow::Result<PathBuf> {
        if let Some(dest) = ProjectPath::project_dir()
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
            Err(anyhow::anyhow!("Error downloading dict"))
        }
    }

    async fn download_dict(language: &str) -> anyhow::Result<PathBuf> {
        if let Some(dest) = ProjectPath::project_dir()
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
            Err(anyhow::anyhow!("Error downloading dict"))
        }
    }

    pub async fn try_build(language: &str) -> anyhow::Result<Self> {
        let dict = SpellChecker::download_dict(language).await?;
        let affix = SpellChecker::download_affix(language).await?;

        Ok(SpellChecker {
            dict: Some(dict::Dictionary::try_build_from_path(&dict, &affix)?),
        })
    }

    pub fn async_build(language: &str) -> JoinHandle<anyhow::Result<Self>> {
        let handle = Handle::current();
        let language = language.to_string();
        handle.spawn(async move { SpellChecker::try_build(&language).await })
    }

    pub fn check_word(&self, word: &str) -> bool {
        self.dict.as_ref().is_some_and(|v| v.check_word(word))
    }
}
