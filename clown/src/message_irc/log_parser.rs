use crate::message_irc::message_logger::LoggedTimedMessage;

use super::message_logger::LoggedMessage;
use chrono::TimeZone;
use nom::bytes::complete::tag;
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::{take_till, take_while, take_while_m_n},
    character::complete::char,
    combinator::map_res,
    sequence::preceded,
};
fn is_ascii_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn buf_to_u32(s: &[u8]) -> anyhow::Result<u32> {
    std::str::from_utf8(s)?
        .parse::<u32>()
        .map_err(|e| anyhow::anyhow!(e))
}

fn buf_to_i32(s: &[u8]) -> anyhow::Result<i32> {
    std::str::from_utf8(s)?
        .parse::<i32>()
        .map_err(|e| anyhow::anyhow!(e))
}

fn parse_date(buf: &[u8]) -> IResult<&[u8], (i32, u32, u32)> {
    let (input, year) = map_res(take_while_m_n(4, 4, is_ascii_digit), buf_to_i32).parse(buf)?;
    let (input, _) = char('-').parse(input)?;
    let (input, month) = map_res(take_while_m_n(2, 2, is_ascii_digit), buf_to_u32).parse(input)?;
    let (input, _) = char('-').parse(input)?;
    let (input, day) = map_res(take_while_m_n(2, 2, is_ascii_digit), buf_to_u32).parse(input)?;
    //let (input, _) = take_till(|c| c == b' ').parse(input)?;
    Ok((input, (year, month, day)))
}

fn parse_time(input: &[u8]) -> IResult<&[u8], (u32, u32, u32)> {
    let (input, hour) = map_res(take_while_m_n(2, 2, is_ascii_digit), buf_to_u32).parse(input)?;
    let (input, _) = char(':').parse(input)?;
    let (input, min) = map_res(take_while_m_n(2, 2, is_ascii_digit), buf_to_u32).parse(input)?;
    let (input, _) = char(':').parse(input)?;
    let (input, sec) = map_res(take_while_m_n(2, 2, is_ascii_digit), buf_to_u32).parse(input)?;
    Ok((input, (hour, min, sec)))
}

fn parse_date_time(input: &[u8]) -> IResult<&[u8], std::time::SystemTime> {
    let (input, date) = parse_date(input)?;
    let (input, _) = take_till(|c: u8| c.is_ascii_digit()).parse(input)?;
    let (input, time) = parse_time(input)?;

    Ok((input, convert_date_time(date, time)))
}

fn convert_date_time(date: (i32, u32, u32), time: (u32, u32, u32)) -> std::time::SystemTime {
    let (year, month, day) = date;
    let (hour, min, sec) = time;

    let utc_dt = chrono::Utc
        .with_ymd_and_hms(year, month, day, hour, min, sec)
        .unwrap();

    std::time::SystemTime::from(utc_dt)
}

fn buf_to_str(s: &[u8]) -> anyhow::Result<&str> {
    std::str::from_utf8(s).map_err(|e| anyhow::anyhow!(e))
}

fn parse_topic(input: &[u8]) -> IResult<&[u8], LoggedMessage<'static>> {
    let (input, source) =
        map_res(take_till(|c: u8| c.is_ascii_whitespace()), buf_to_str).parse(input)?;
    let (input, _) = tag(" has changed topic for ").parse(input)?;
    let (input, channel) =
        map_res(take_till(|c: u8| c.is_ascii_whitespace()), buf_to_str).parse(input)?;
    let (input, _) = tag(" \"").parse(input)?;
    let (input, content) = map_res(take_till(|c: u8| c == b'\"'), buf_to_str).parse(input)?;

    Ok((
        input,
        LoggedMessage::Topic {
            source: std::borrow::Cow::Owned(source.to_string()),
            channel: std::borrow::Cow::Owned(channel.to_string()),
            content: std::borrow::Cow::Owned(content.to_string()),
        },
    ))
}

fn parse_join<'a>(input: &'a [u8], source: &str) -> IResult<&'a [u8], LoggedMessage<'static>> {
    let (input, _) = tag(" has joined ").parse(input)?;
    println!("{source} '{:?}'", buf_to_str(input));

    let (input, channel) = map_res(nom::combinator::rest, buf_to_str).parse(input)?;

    Ok((
        input,
        LoggedMessage::Join {
            source: std::borrow::Cow::Owned(source.to_string()),
            channel: std::borrow::Cow::Owned(channel.to_string()),
        },
    ))
}

fn parse_nick<'a>(input: &'a [u8], source: &str) -> IResult<&'a [u8], LoggedMessage<'static>> {
    let (input, _) = tag(" has changed their nickname to ").parse(input)?;
    let (input, new) = map_res(nom::combinator::rest, buf_to_str).parse(input)?;

    Ok((
        input,
        LoggedMessage::NickChange {
            old: std::borrow::Cow::Owned(source.to_string()),
            new: std::borrow::Cow::Owned(new.to_string()),
        },
    ))
}

fn parse_priv_message<'a>(input: &'a [u8]) -> IResult<&'a [u8], LoggedMessage<'static>> {
    let (input, source) =
        map_res(take_till(|c: u8| c.is_ascii_whitespace()), buf_to_str).parse(input)?;
    let (input, rest) = map_res(nom::combinator::rest, buf_to_str).parse(input)?;

    Ok((
        input,
        LoggedMessage::Message {
            source: std::borrow::Cow::Owned(source.to_string()),
            content: std::borrow::Cow::Owned(rest.to_string()),
        },
    ))
}

fn parse_part<'a>(input: &'a [u8], source: &str) -> IResult<&'a [u8], LoggedMessage<'static>> {
    let (input, _) = tag(" has left ").parse(input)?;
    let (input, channel) =
        map_res(take_till(|c: u8| c.is_ascii_whitespace()), buf_to_str).parse(input)?;

    Ok((
        input,
        LoggedMessage::Part {
            source: std::borrow::Cow::Owned(source.to_string()),
            channel: std::borrow::Cow::Owned(channel.to_string()),
        },
    ))
}

fn parse_quit<'a>(input: &'a [u8], source: &str) -> IResult<&'a [u8], LoggedMessage<'static>> {
    let (input, _) = tag(" has quit").parse(input)?;

    Ok((
        input,
        LoggedMessage::Quit {
            source: std::borrow::Cow::Owned(source.to_string()),
        },
    ))
}

fn parse_event(input: &[u8]) -> IResult<&[u8], LoggedMessage<'static>> {
    alt((
        preceded(tag("\t<--\t "), parse_outgoing_subevents),
        preceded(tag("\t-->\t "), parse_ingoing_subevents),
        preceded(tag("\t--\t "), parse_network_subevents),
        parse_priv_message,
    ))
    .parse(input)
}

fn parse_outgoing_subevents(input: &[u8]) -> IResult<&[u8], LoggedMessage<'static>> {
    let (input, source) =
        map_res(take_till(|c: u8| c.is_ascii_whitespace()), buf_to_str).parse(input)?;

    alt((
        |i| parse_nick(i, source),
        |i| parse_part(i, source),
        |i| parse_quit(i, source),
    ))
    .parse(input)
}

fn parse_ingoing_subevents(input: &[u8]) -> IResult<&[u8], LoggedMessage<'static>> {
    let (input, source) =
        map_res(take_till(|c: u8| c.is_ascii_whitespace()), buf_to_str).parse(input)?;

    alt((|i| parse_join(i, source),)).parse(input)
}

fn parse_network_subevents(input: &[u8]) -> IResult<&[u8], LoggedMessage<'static>> {
    alt((|i| parse_topic(i),)).parse(input)
}

pub fn parse(input: &[u8]) -> anyhow::Result<LoggedTimedMessage<'static>> {
    let (input, time) =
        parse_date_time(input).map_err(|e| anyhow::anyhow!("Date parsing failed: {}", e))?;

    let (_, event) =
        parse_event(input).map_err(|e| anyhow::anyhow!("Event parsing failed: {}", e))?;

    Ok(LoggedTimedMessage {
        time,
        message: event,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    // Helper to format SystemTime to a readable string for comparison if needed
    fn to_utc_string(st: SystemTime) -> String {
        let datetime: chrono::DateTime<chrono::Utc> = st.into();
        datetime.to_rfc3339()
    }

    #[test]
    fn test_parse_join() {
        let raw = b"2024-05-20 14:30:05\t-->\t Alice has joined #rust-lang";
        let result = parse(raw);
        let result = result.expect("failed");
        assert_eq!(to_utc_string(result.time), "2024-05-20T14:30:05+00:00");
        if let LoggedMessage::Join { source, channel } = result.message {
            assert_eq!(source, "Alice");
            assert_eq!(channel, "#rust-lang");
        } else {
            panic!("Expected Join message");
        }
    }

    #[test]
    fn test_parse_quit() {
        let raw = b"2024-05-20 15:00:00\t<--\t Bob has quit";
        let result = parse(raw).expect("Should parse quit");

        if let LoggedMessage::Quit { source } = result.message {
            assert_eq!(source, "Bob");
        } else {
            panic!("Expected Quit message");
        }
    }

    #[test]
    fn test_parse_nick_change() {
        let raw = b"2026-03-21 19:59:19\t<--\t Charlie has changed their nickname to Chuck";
        let result = parse(raw).expect("Should parse nick change");

        if let LoggedMessage::NickChange { old, new } = result.message {
            assert_eq!(old, "Charlie");
            assert_eq!(new, "Chuck");
        } else {
            panic!("Expected NickChange message");
        }
    }

    #[test]
    fn test_parse_topic() {
        let raw =
            b"2024-05-20 16:00:00\t--\t Dave has changed topic for #help \"No hello, just ask\"";
        let result = parse(raw).expect("Should parse topic");

        if let LoggedMessage::Topic {
            source,
            channel,
            content,
        } = result.message
        {
            assert_eq!(source, "Dave");
            assert_eq!(channel, "#help");
            assert_eq!(content, "No hello, just ask");
        } else {
            panic!("Expected Topic message");
        }
    }

    #[test]
    fn test_invalid_date() {
        let raw = b"not-a-date\t-->\t Alice has joined #rust-lang";
        let result = parse(raw);
        assert!(result.is_err());
    }
}
