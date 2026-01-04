use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::take_while1,
    character::complete::char,
    combinator::{map, opt, verify},
    sequence::preceded,
};

#[derive(PartialEq, Eq)]
pub enum SourceKind<'a> {
    Nick(&'a [u8]),
    Server(&'a [u8]),
}

#[derive(PartialEq, Eq)]
pub struct Source<'s> {
    source: Option<SourceKind<'s>>,
    user: Option<&'s [u8]>,
    host: Option<&'s [u8]>,
}

fn as_str(bytes: &[u8]) -> std::borrow::Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

impl std::fmt::Debug for SourceKind<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceKind::Nick(nick) => f.debug_tuple("Nick").field(&as_str(nick)).finish(),
            SourceKind::Server(serv) => f.debug_tuple("Server").field(&as_str(serv)).finish(),
        }
    }
}

impl std::fmt::Debug for Source<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Source")
            .field("source", &self.source)
            .field("user", &self.user.as_ref().map(|u| as_str(u)))
            .field("host", &self.host.as_ref().map(|h| as_str(h)))
            .finish()
    }
}

#[cfg(test)]
impl<'a> Source<'a> {
    pub fn new(
        in_source_type: Option<SourceKind<'a>>,
        user: Option<&'a [u8]>,
        host: Option<&'a [u8]>,
    ) -> Self {
        Self {
            source: in_source_type,
            user,
            host,
        }
    }
}

impl Source<'_> {
    pub fn get_source_kind(&self) -> Option<&str> {
        if let Some(source_type) = &self.source {
            match source_type {
                SourceKind::Nick(name) => std::str::from_utf8(name).ok(),
                SourceKind::Server(name) => std::str::from_utf8(name).ok(),
            }
        } else {
            None
        }
    }
}

fn nickname(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let valid_nick_char = |c: u8| !c.is_ascii_whitespace() && c != b'!' && c != b'@' && c != b':';
    verify(take_while1(valid_nick_char), |s: &[u8]| !s.contains(&b'.')).parse(input)
}

#[cfg(test)]
fn nickname_opt(input: &[u8]) -> IResult<&[u8], Option<&[u8]>> {
    opt(nickname).parse(input)
}

fn server(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let not_space = |c: u8| !c.is_ascii_whitespace() && c != b'!';
    verify(take_while1(not_space), |s: &[u8]| s.contains(&b'.')).parse(input)
}

#[cfg(test)]
fn server_opt(input: &[u8]) -> IResult<&[u8], Option<&[u8]>> {
    opt(server).parse(input)
}

fn user(buf: &[u8]) -> IResult<&[u8], &[u8]> {
    let is_valid_user_char = |c: u8| c.is_ascii_alphanumeric();

    let (buf, user) = take_while1(is_valid_user_char)(buf)?;
    Ok((buf, user))
}

fn host(buf: &[u8]) -> IResult<&[u8], &[u8]> {
    let is_valid_host_char = |c: u8| !c.is_ascii_whitespace();

    let (buf, user) = take_while1(is_valid_host_char)(buf)?;
    Ok((buf, user))
}

fn parse_source_inner(buf: &[u8]) -> IResult<&[u8], Source<'_>> {
    let (buf, source) = alt((
        map(nickname, SourceKind::Nick),
        map(server, SourceKind::Server),
    ))
    .parse(buf)?;

    let (buf, user) = opt(preceded(char('!'), user)).parse(buf)?;
    let (buf, host) = opt(preceded(char('@'), host)).parse(buf)?;

    let source = Source {
        source: Some(source),
        user,
        host,
    };
    Ok((buf, source))
}

pub fn parse_source(buf: &[u8]) -> (&[u8], Option<Source<'_>>) {
    let colon = char(':');
    if let Ok((buf, source)) = preceded(colon, parse_source_inner).parse(buf) {
        return (buf, Some(source));
    }
    (buf, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nickname() {
        // Valid nickname
        let input = b"nick rest";
        let (rest, nick) = nickname_opt(input).unwrap();
        assert_eq!(nick, Some(&b"nick"[..]));
        assert_eq!(rest, b" rest");

        // Nickname with dot (should not match)
        let input = b"nick.name rest";
        let (rest, nick) = nickname_opt(input).unwrap();
        assert_eq!(nick, None);
        assert_eq!(rest, b"nick.name rest");

        let input = b"nick!name rest";
        let (rest, nick) = nickname_opt(input).unwrap();
        assert_eq!(nick, Some(&b"nick"[..]));
        assert_eq!(rest, b"!name rest");
    }

    #[test]
    fn test_server() {
        // Valid server name
        let input = b"irc.example.com ";
        let (rest, _server) = server_opt(input).unwrap();
        assert_eq!(_server, Some(&b"irc.example.com"[..]));
        assert_eq!(rest, b" ");

        // Server name without dot (should not match)
        let input = b"ircserver ";
        let (rest, server) = server_opt(input).unwrap();
        assert_eq!(server, None);
        assert_eq!(rest, b"ircserver ");
    }

    #[test]
    fn test_user() {
        let input = b"user123 rest";
        let (rest, user) = user(input).unwrap();
        assert_eq!(user, &b"user123"[..]);
        assert_eq!(rest, b" rest");
    }

    #[test]
    fn test_host() {
        let input = b"host456 remain";
        let (rest, host) = host(input).unwrap();
        assert_eq!(host, &b"host456"[..]);
        assert_eq!(rest, b" remain");
    }

    #[test]
    fn test_parse_source_nick_user_host() {
        let input = b":nick!user@host ";
        let (rest, source) = parse_source(input);
        assert_eq!(
            source,
            Some(Source {
                source: Some(SourceKind::Nick(&b"nick"[..])),
                user: Some(&b"user"[..]),
                host: Some(&b"host"[..]),
            })
        );
        assert_eq!(rest, b" ");
    }

    #[test]
    fn test_parse_source_server() {
        let input = b":irc.example.com ";
        let (rest, source) = parse_source(input);
        assert_eq!(
            source,
            Some(Source {
                source: Some(SourceKind::Server(&b"irc.example.com"[..])),
                user: None,
                host: None,
            })
        );
        assert_eq!(rest, b" ");
    }

    #[test]
    fn test_source_server_kind() {
        let input = b":irc.example.com ";
        let (_rest, source) = parse_source(input);
        if let Some(source) = source {
            assert_eq!(source.get_source_kind(), Some("irc.example.com"));
        } else {
            assert_ne!(source, None)
        }
    }

    #[test]
    fn test_source_nick_kind() {
        let input = b":jo!farine4@inspircd";
        let (_rest, source) = parse_source(input);
        if let Some(source) = source {
            assert_eq!(source.get_source_kind(), Some("jo"));
        } else {
            assert_ne!(source, None)
        }
    }
}
