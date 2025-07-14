use crate::parser::{parse_command, parse_parameters, parse_trailing};
use crate::source::{Source, SourceKind, parse_source};
use ouroboros::self_referencing;
/// Note: Server sources (used for server-to-server communications) are not handled.
#[derive(Debug, PartialEq, Eq)]
pub struct IRCMessage<'s> {
    source: Option<Source<'s>>,
    command: Option<&'s [u8]>,
    parameters: Vec<&'s [u8]>,
    trailing: Option<&'s [u8]>,
}

#[self_referencing]
#[derive(Debug, PartialEq, Eq)]
pub struct Message {
    data: Vec<u8>,
    #[borrows(data)]
    #[covariant]
    internal: IRCMessage<'this>,
}

impl Message {
    pub fn get_command_name(&self) -> Option<&str> {
        let irc = self.borrow_internal();
        irc.command.and_then(|value| str::from_utf8(value).ok())
    }

    pub fn get_parameters(&self) -> Vec<&str> {
        let irc = self.borrow_internal();
        irc.parameters
            .iter()
            .map(|value| str::from_utf8(value).unwrap_or_default())
            .collect()
    }

    pub fn get_source(&self) -> Option<&str> {
        use crate::source::Source;
        let irc = self.borrow_internal();
        if let Some(source_kind) = irc.source.as_ref() {
            source_kind.get_source_kind()
        } else {
            None
        }
    }

    pub fn get_trailing(&self) -> Option<&str> {
        let irc = self.borrow_internal();
        irc.trailing.and_then(|value| str::from_utf8(value).ok())
    }
}

fn parse_message(buf: &[u8]) -> anyhow::Result<IRCMessage<'_>> {
    let (buf, source) = parse_source(buf);

    let (buf, command) = match parse_command(buf) {
        Ok((buf, command)) => (buf, command),
        Err(_) => (buf, None),
    };

    let (buf, parameters) = match parse_parameters(buf) {
        Ok((buf, parameters)) => (buf, parameters),
        Err(_) => (buf, vec![]),
    };

    let (_buf, trailing) = match parse_trailing(buf) {
        Ok((buf, trailing)) => (buf, trailing),
        Err(_) => (buf, None),
    };

    Ok(IRCMessage {
        source,
        command,
        parameters,
        trailing,
    })
}

pub fn create_message(buf: &[u8]) -> anyhow::Result<Message> {
    MessageTryBuilder {
        data: buf.to_owned(),
        internal_builder: |data: &Vec<u8>| -> anyhow::Result<IRCMessage<'_>> {
            let slice: &[u8] = data.as_slice();
            parse_message(slice)
        },
    }
    .try_build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_message_full() {
        let input = b":nick!user@host PRIVMSG #chan :hello world\r\n";
        let msg = parse_message(input).unwrap();
        assert_eq!(
            msg.source,
            Some(Source::new(
                Some(SourceKind::Nick(&b"nick"[..])),
                Some(&b"user"[..]),
                Some(&b"host"[..])
            ))
        );
        assert_eq!(msg.command, Some(&b"PRIVMSG"[..]));
        assert_eq!(msg.parameters, vec![&b"#chan"[..]]);
        assert_eq!(msg.trailing, Some(&b"hello world"[..]));
    }

    #[test]
    fn test_parse_message_no_source() {
        let input = b"PING :server\r\n";
        let msg = parse_message(input).unwrap();
        assert_eq!(msg.source, None);
        assert_eq!(msg.command, Some(&b"PING"[..]));
        assert_eq!(msg.trailing, Some(&b"server"[..]));
    }

    #[test]
    fn test_create_message() {
        let input = b":nick!user@host PRIVMSG #chan :hello world\r\n";
        let msg = create_message(input).unwrap();
        let internal = msg.borrow_internal();
        assert_eq!(internal.command, Some(&b"PRIVMSG"[..]));
        assert_eq!(internal.parameters, vec![&b"#chan"[..]]);
        assert_eq!(internal.trailing, Some(&b"hello world"[..]));
    }
}
