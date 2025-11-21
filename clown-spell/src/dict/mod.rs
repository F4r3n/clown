use fst::{IntoStreamer, Set};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::{io::Read, path::Path};
pub struct Dictionary {
    words: fst::Set<Vec<u8>>,
}

//TODO: replace anyhow by thiserror
impl Dictionary {
    pub fn try_build_from_path(dict: &str, affix: &str) -> anyhow::Result<Self> {
        let dict_file = File::open(dict)?;
        let dict_reader = BufReader::new(dict_file);

        let affix_file = File::open(affix)?;
        let affix_reader = BufReader::new(affix_file);

        Dictionary::try_build(dict_reader, affix_reader)
    }

    pub fn try_build<T>(reader_dict: T, reader_affix: T) -> anyhow::Result<Self>
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

        let set = fst::Set::from_iter(words)?;

        Ok(Self { words: set })
    }

    pub fn check_word(&self, word: &str) -> bool {
        self.words.contains(word)
    }
}
