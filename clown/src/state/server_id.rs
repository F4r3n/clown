use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct ServerID(usize);

impl ServerID {
    pub fn as_usize(&self) -> usize {
        self.0
    }
    pub const fn new(val: usize) -> Self {
        Self(val)
    }
}

impl Display for ServerID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
