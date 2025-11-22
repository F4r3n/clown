use fst::{IntoStreamer, Set};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::{io::Read, path::Path};

use thiserror::Error;
#[derive(Error, Debug)]
pub enum DictionaryBuildError {
    #[error("Cannot open file")]
    OpeningFile(#[from] std::io::Error),
    #[error("cannot iterate over words")]
    WordsIterator(#[from] fst::Error),
}

pub struct Dictionary {
    words: fst::Set<Vec<u8>>,
}

//TODO: replace anyhow by thiserror
impl Dictionary {
    pub fn try_build_from_path(
        dict: &PathBuf,
        affix: &PathBuf,
    ) -> Result<Self, DictionaryBuildError> {
        let dict_file = File::open(dict).map_err(|err| DictionaryBuildError::OpeningFile(err))?;
        let dict_reader = BufReader::new(dict_file);

        let affix_file = File::open(affix).map_err(|err| DictionaryBuildError::OpeningFile(err))?;
        let affix_reader = BufReader::new(affix_file);

        Dictionary::try_build(dict_reader, affix_reader)
    }

    pub fn try_build<T>(reader_dict: T, reader_affix: T) -> Result<Self, DictionaryBuildError>
    where
        T: BufRead,
    {
        //The words should be already sorted by the dictionary
        let words = reader_dict
            .lines()
            .skip(1) // skip first line = count
            .filter_map(|line| line.ok())
            .filter(|line| !line.is_empty())
            .map(|line| {
                line.split('/')
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .as_bytes()
                    .to_vec()
            });

        let set = fst::Set::from_iter(words).map_err(DictionaryBuildError::WordsIterator)?;

        Ok(Self { words: set })
    }

    pub fn check_word(&self, word: &str) -> bool {
        self.words.contains(word)
    }
}
