use thiserror::Error;

#[derive(Error, Debug)]
pub enum DictionaryBuildError {
    #[error("Cannot open file")]
    OpeningFile(#[from] std::io::Error),
    #[error("Cannot open file")]
    InvalidFormatNoCount(#[from] std::num::ParseIntError),
    #[error("cannot iterate over words")]
    WordsIterator(#[from] fst::Error),
}
