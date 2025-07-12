use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{take_till, take_while_m_n, take_while1},
    character::complete::{alpha1, space0, space1},
    combinator::opt,
    sequence::{delimited, preceded},
};

pub fn parse_command(buf: &[u8]) -> IResult<&[u8], Option<&[u8]>> {
    opt(preceded(
        space0,                                                          // optional leading spaces
        alt((alpha1, take_while_m_n(3, 3, |c: u8| c.is_ascii_digit()))), // letter* / 3digit
    ))
    .parse(buf)
}

/// Parse IRC parameters into a Vec<&[u8]>
pub fn parse_parameters(input: &[u8]) -> IResult<&[u8], Vec<&[u8]>> {
    let pos_colon = input
        .windows(2)
        .position(|w| w == b" :")
        .unwrap_or(input.len());

    let (input, parsed) =
        take_while_m_n(0, pos_colon, |c| c != b'\r' && c != b'\n' && c != 0)(input)?;
    let params = parsed
        .split(|v| *v == b' ')
        .filter(|v| !v.is_empty())
        .collect();
    Ok((input, params))
}

/// Parse IRC parameters into a Vec<&[u8]>
pub fn parse_trailing(input: &[u8]) -> IResult<&[u8], std::option::Option<&[u8]>> {
    opt(preceded(
        nom::bytes::complete::tag(" :"),
        take_till(|c| c == b'\r' || c == b'\n' || c == 0),
    ))
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_alpha() {
        let input = b"PRIVMSG ";
        let (rest, cmd) = parse_command(input).unwrap();
        assert_eq!(cmd, Some(&b"PRIVMSG"[..]));
        assert_eq!(rest, b" ");
    }

    #[test]
    fn test_parse_command_numeric() {
        let input = b"001 ";
        let (rest, cmd) = parse_command(input).unwrap();
        assert_eq!(cmd, Some(&b"001"[..]));
        assert_eq!(rest, b" ");
    }
    #[test]
    fn test_parse_trailing() {
        let input = b" :this is trailing\r\n";
        let (rest, param) = parse_trailing(input).unwrap();
        assert_eq!(param, Some(&b"this is trailing"[..]));
        assert_eq!(rest, b"\r\n");
    }

    #[test]
    fn test_parse_parameters_middle() {
        let input = b" param1 param2 :trailing param\r\n";
        let (rest, params) = parse_parameters(input).unwrap();
        assert_eq!(params, vec![&b"param1"[..], &b"param2"[..]]);
        assert_eq!(rest, b" :trailing param\r\n");
    }

    #[test]
    fn test_parse_parameters_trailing() {
        let input = b":nick!user@host PRIVMSG #chan :hello world\r\n";
        let (rest, params) = parse_parameters(input).unwrap();
        let (rest, trailing) = parse_trailing(rest).unwrap();

        assert_eq!(trailing, Some(&b"hello world"[..]));
    }

    #[test]
    fn test_parse_parameters_trailing_no_parameters() {
        let input = b"PRIVMSG :hello world\r\n";
        let (rest, command) = parse_command(input).unwrap();
        let (rest, params) = parse_parameters(rest).unwrap();
        let (rest, trailing) = parse_trailing(rest).unwrap();

        assert_eq!(trailing, Some(&b"hello world"[..]));
    }

    #[test]
    fn test_parse_colon_middle() {
        let input = b" param:1 param2 :trailing param\r\n";
        let (rest, params) = parse_parameters(input).unwrap();
        assert_eq!(params, vec![&b"param:1"[..], &b"param2"[..]]);
        assert_eq!(rest, b" :trailing param\r\n");
    }
}
