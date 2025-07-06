use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{take_till, take_while_m_n, take_while1},
    character::complete::{alpha1, space0, space1},
    combinator::opt,
    sequence::{delimited, preceded},
};
mod source;
use ouroboros::self_referencing;
/// Note: Server sources (used for server-to-server communications) are not handled.
#[derive(Debug, PartialEq, Eq)]
pub struct IRCMessage<'s> {
    source: Option<source::Source<'s>>,
    command: Option<&'s [u8]>,
    parameters: Vec<&'s [u8]>,
}

#[self_referencing]
#[derive(Debug, PartialEq, Eq)]
pub struct Message {
    data: Vec<u8>,
    #[borrows(data)]
    #[covariant]
    internal: IRCMessage<'this>,
}

fn parse_command(buf: &[u8]) -> IResult<&[u8], Option<&[u8]>> {
    opt(delimited(
        space0,                                                          // optional leading spaces
        alt((alpha1, take_while_m_n(3, 3, |c: u8| c.is_ascii_digit()))), // letter* / 3digit
        space0,                                                          // optional trailing spaces
    ))
    .parse(buf)
}

/// nospcrlfcl: any char except NUL, CR, LF, colon (:) and SPACE
fn is_nospcrlfcl(c: u8) -> bool {
    c != 0 && c != b'\r' && c != b'\n' && c != b':' && c != b' '
}

/// Parse a "middle" parameter (see IRC spec)
fn parse_middle(input: &[u8]) -> IResult<&[u8], &[u8]> {
    take_while1(is_nospcrlfcl)(input)
}

/// Parse a "trailing" parameter (after a colon, can contain anything except CR/LF/NUL)
fn parse_trailing(input: &[u8]) -> IResult<&[u8], &[u8]> {
    // Skip the leading ':'
    let (input, _) = nom::bytes::complete::tag(":")(input)?;
    // Take until CR/LF/NUL or end of input
    let (input, trailing) = take_till(|c| c == b'\r' || c == b'\n' || c == 0)(input)?;
    Ok((input, trailing))
}

/// Parse IRC parameters into a Vec<&[u8]>
pub fn parse_parameters(input: &[u8]) -> IResult<&[u8], Vec<&[u8]>> {
    let mut params = Vec::new();
    let mut input = input;
    let mut has_trailing = false;
    // Parse up to 14 middle parameters (spec says max 15, but trailing counts as one)
    for _ in 0..14 {
        // Skip leading spaces
        let (rest, _) = space0(input)?;
        // If next is ':', break to parse trailing
        if rest.starts_with(b":") {
            has_trailing = true;
            break;
        }
        // Try to parse a middle parameter
        match preceded(space0, parse_middle).parse(input) {
            Ok((rest2, param)) => {
                params.push(param);
                input = rest2;
            }
            Err(_) => break,
        }
    }

    // Now try to parse a trailing parameter (if present)
    let (mut input, _) = space0(input)?;
    if has_trailing {
        let (rest, trailing) = preceded(space1, parse_trailing)
            .parse(input)
            .or_else(|_| parse_trailing(input))?; // allow no space before trailing
        params.push(trailing);
        input = rest;
    }

    Ok((input, params))
}

fn parse_message(buf: &[u8]) -> anyhow::Result<IRCMessage<'_>> {
    let (buf, parsed_source) = source::parse_source(buf);

    let (buf, command) = match parse_command(buf) {
        Ok((buf, command)) => (buf, command),
        Err(_) => (buf, None),
    };

    let (_buf, parameters) = match parse_parameters(buf) {
        Ok((buf, parameters)) => (buf, parameters),
        Err(_) => (buf, vec![]),
    };

    Ok(IRCMessage {
        source: parsed_source,
        command,
        parameters,
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
    fn test_parse_command_alpha() {
        let input = b"PRIVMSG ";
        let (rest, cmd) = parse_command(input).unwrap();
        assert_eq!(cmd, Some(&b"PRIVMSG"[..]));
        assert_eq!(rest, b"");
    }

    #[test]
    fn test_parse_command_numeric() {
        let input = b"001 ";
        let (rest, cmd) = parse_command(input).unwrap();
        assert_eq!(cmd, Some(&b"001"[..]));
        assert_eq!(rest, b"");
    }

    #[test]
    fn test_parse_middle() {
        let input = b"middle rest";
        let (rest, param) = parse_middle(input).unwrap();
        assert_eq!(param, &b"middle"[..]);
        assert_eq!(rest, b" rest");
    }

    #[test]
    fn test_parse_trailing() {
        let input = b":this is trailing\r\n";
        let (rest, param) = parse_trailing(input).unwrap();
        assert_eq!(param, &b"this is trailing"[..]);
        assert_eq!(rest, b"\r\n");
    }

    #[test]
    fn test_parse_parameters_middle_and_trailing() {
        let input = b" param1 param2 :trailing param\r\n";
        let (rest, params) = parse_parameters(input).unwrap();
        assert_eq!(
            params,
            vec![&b"param1"[..], &b"param2"[..], &b"trailing param"[..]]
        );
        assert_eq!(rest, b"\r\n");
    }

    #[test]
    fn test_parse_message_full() {
        let input = b":nick!user@host PRIVMSG #chan :hello world\r\n";
        let msg = parse_message(input).unwrap();
        assert_eq!(
            msg.source,
            Some(source::Source::new(
                Some(source::SOURCE::Nick(&b"nick"[..])),
                Some(&b"user"[..]),
                Some(&b"host"[..])
            ))
        );
        assert_eq!(msg.command, Some(&b"PRIVMSG"[..]));
        assert_eq!(msg.parameters, vec![&b"#chan"[..], &b"hello world"[..]]);
    }

    #[test]
    fn test_parse_message_no_source() {
        let input = b"PING :server\r\n";
        let msg = parse_message(input).unwrap();
        assert_eq!(msg.source, None);
        assert_eq!(msg.command, Some(&b"PING"[..]));
        assert_eq!(msg.parameters, vec![&b"server"[..]]);
    }

    #[test]
    fn test_create_message() {
        let input = b":nick!user@host PRIVMSG #chan :hello world\r\n";
        let msg = create_message(input).unwrap();
        let internal = msg.borrow_internal();
        assert_eq!(internal.command, Some(&b"PRIVMSG"[..]));
        assert_eq!(
            internal.parameters,
            vec![&b"#chan"[..], &b"hello world"[..]]
        );
    }
}
