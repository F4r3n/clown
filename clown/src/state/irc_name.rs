#[derive(Debug, Clone, Eq, Default)]
pub struct IrcName(String);

//TODO: store

impl IrcName {
    pub fn new(name: &str) -> Self {
        Self(Self::sanitize(name).to_string())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn sanitize(name: &str) -> &str {
        name.strip_prefix('@').unwrap_or(name)
    }
}

impl From<&str> for IrcName {
    fn from(name: &str) -> Self {
        Self::new(name)
    }
}

impl From<String> for IrcName {
    fn from(mut name: String) -> Self {
        let prefix_len = name.len() - Self::sanitize(&name).len();
        name.drain(..prefix_len);
        Self(name)
    }
}

impl std::fmt::Display for IrcName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl PartialEq for IrcName {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl PartialEq<str> for IrcName {
    fn eq(&self, other: &str) -> bool {
        self.0.eq_ignore_ascii_case(other)
    }
}

impl PartialEq<&str> for IrcName {
    fn eq(&self, other: &&str) -> bool {
        self.0.eq_ignore_ascii_case(other)
    }
}

impl std::hash::Hash for IrcName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut buf = [0u8; 64];
        for chunk in self.0.as_bytes().chunks(buf.len()) {
            let buf = &mut buf[..chunk.len()];
            buf.copy_from_slice(chunk);
            buf.make_ascii_lowercase();
            state.write(buf);
        }
        state.write_u8(0xff);
    }
}
