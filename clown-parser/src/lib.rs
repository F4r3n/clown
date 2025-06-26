use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{take_till, take_while_m_n, take_while1},
    character::complete::{alpha1, char, space0, space1},
    combinator::{opt, verify},
    sequence::delimited,
    sequence::preceded,
};

/// Note: Server sources (used for server-to-server communications) are not handled.
#[derive(Debug, PartialEq, Eq)]
pub struct Source<'s> {
    server_name: Option<&'s [u8]>,
    nickname: Option<&'s [u8]>,
    user: Option<&'s [u8]>,
    host: Option<&'s [u8]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct IRCMessage<'s> {
    source: Source<'s>,
    command: Option<&'s [u8]>,
    parameters: Vec<&'s [u8]>,
}

fn nickname(input: &[u8]) -> IResult<&[u8], Option<&[u8]>> {
    // Parse one or more bytes that are not whitespace
    let not_space = |c: u8| !c.is_ascii_whitespace();
    // Then verify that there is no dot in the result
    opt(verify(take_while1(not_space), |s: &[u8]| {
        !s.contains(&b'.')
    }))
    .parse(input)
}

fn server(input: &[u8]) -> IResult<&[u8], Option<&[u8]>> {
    // Parse one or more bytes that are not whitespace
    let not_space = |c: u8| !c.is_ascii_whitespace();
    // Then verify that there is a dot in the result
    opt(verify(take_while1(not_space), |s: &[u8]| s.contains(&b'.'))).parse(input)
}

fn user(buf: &[u8]) -> IResult<&[u8], &[u8]> {
    let is_valid_user_char = |c: u8| c.is_ascii_alphanumeric();

    let (buf, user) = take_while1(is_valid_user_char)(buf)?;
    Ok((buf, user))
}

fn host(buf: &[u8]) -> IResult<&[u8], &[u8]> {
    let is_valid_host_char = |c: u8| c.is_ascii_alphanumeric();

    let (buf, user) = take_while1(is_valid_host_char)(buf)?;
    Ok((buf, user))
}

fn parse_source_inner(buf: &[u8]) -> IResult<&[u8], Source<'_>> {
    let (buf, nickname) = nickname(buf)?;
    let (buf, server_name) = server(buf)?;
    let (buf, user) = opt(preceded(char('!'), user)).parse(buf)?;
    let (buf, host) = opt(preceded(char('@'), host)).parse(buf)?;

    let source = Source {
        server_name,
        nickname,
        user,
        host,
    };
    Ok((buf, source))
}

fn parse_source(buf: &[u8]) -> IResult<&[u8], Source<'_>> {
    let colon = char(':');
    let (buf, source) = preceded(colon, parse_source_inner).parse(buf)?;
    Ok((buf, source))
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
        match preceded(space1, parse_middle).parse(input) {
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

pub fn parse_message(buf: &[u8]) -> IResult<&[u8], IRCMessage<'_>> {
    let parsed_source = parse_source(&buf)?;
    let parsed_command = parse_command(parsed_source.0)?;
    let parsed_parameters = parse_parameters(parsed_command.0)?;

    let irc_message = IRCMessage {
        source: parsed_source.1,
        command: parsed_command.1,
        parameters: parsed_parameters.1,
    };
    Ok((buf, irc_message))
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_source() {
        //example :irc.example.com CAP LS * :multi-prefix extended-join sasl
        let string = ":irc.example.com CAP LS * :multi-prefix extended-join sasl";
        let result = parse_source(string.as_bytes());
        assert!(result.is_ok());
        if let Ok(source) = result {
            let server_name = source.1.server_name.unwrap_or_default();
            assert!(server_name.eq("irc.example.com".as_bytes()));
            assert!(source.1.nickname.is_none());
            assert!(source.1.user.is_none());
            assert!(source.1.host.is_none());
        }
    }

    #[test]
    fn test_server_name() -> Result<(), Box<dyn std::error::Error>> {
        let input = "irc.example.com";

        let parsed_server_name = server(input.as_bytes())?;
        assert!(parsed_server_name.1.is_some());

        let input = "ircexamplecom";
        let parsed_server_name = server(input.as_bytes())?;
        assert!(parsed_server_name.1.is_none());

        Ok(())
    }

    #[test]
    fn test_parameters() -> Result<(), Box<dyn std::error::Error>> {
        let input = " REQ :sasl message-tags foo";

        let parsed_parameters = parse_parameters(input.as_bytes())?;
        let expected = vec!["REQ".as_bytes(), "sasl message-tags foo".as_bytes()];
        assert!(parsed_parameters.1.iter().eq(expected.iter()));

        Ok(())
    }
}
