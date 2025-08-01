use pest::Parser;
use pest_derive::Parser;

/// Note: Server sources (used for server-to-server communications) are not handled.
#[derive(Debug, PartialEq, Eq)]
pub struct IRCMessage<'s> {
    source: Option<Source<'s>>,
    command: Option<&'s str>,
    parameters: Vec<&'s str>,
    trailing: Option<&'s str>,
}

#[derive(Debug, PartialEq, Eq)]
enum SourceKind<'a> {
    Nick(&'a str),
    Server(&'a str),
}

#[derive(Debug, PartialEq, Eq)]
struct Source<'s> {
    source: Option<SourceKind<'s>>,
    user: Option<&'s str>,
    host: Option<&'s str>,
}

#[derive(Parser)]
#[grammar = "irc.pest"] // relative path to the grammar file
struct IRCParser;

fn parse_pest_message(input: &str) -> Result<IRCMessage, pest::error::Error<Rule>> {
    let mut parsed = IRCParser::parse(Rule::irc_message, input)?;
    let message_pair = parsed.next().unwrap();

    let mut source = None;
    let mut command = None;
    let mut parameters = Vec::new();
    let mut trailing = None;

    for pair in message_pair.into_inner() {
        match pair.as_rule() {
            Rule::prefix => {
                let mut inner = pair.into_inner();
                if let Some(source_pair) = inner.next() {
                    // Source can be either a servername or nick!user@host
                    let mut source_kind = None;
                    let mut user = None;
                    let mut host = None;

                    match source_pair.as_rule() {
                        Rule::source => {
                            for part in source_pair.into_inner() {
                                match part.as_rule() {
                                    Rule::nickname => {
                                        source_kind = Some(SourceKind::Nick(part.as_str()))
                                    }
                                    Rule::servername => {
                                        source_kind = Some(SourceKind::Server(part.as_str()));
                                    }
                                    Rule::user => {
                                        // user_name is inner of user
                                        user = part.into_inner().next().map(|u| u.as_str())
                                    }
                                    Rule::host => {
                                        // host_name is inner of host
                                        host = part.into_inner().next().map(|h| h.as_str())
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                    source = Some(Source {
                        host,
                        user,
                        source: source_kind,
                    });
                }
            }
            Rule::command => {
                command = Some(pair.as_str());
            }
            Rule::parameters => {
                // parameters contain `param` rule(s) inside; flatten them
                for param_pair in pair.into_inner() {
                    parameters.push(param_pair.as_str());
                }
            }
            Rule::trailing => {
                // trailing_data is inner of trailing
                trailing = pair.into_inner().next().map(|inner| inner.as_str());
            }
            _ => {}
        }
    }

    Ok(IRCMessage {
        source,
        command,
        parameters,
        trailing,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_message_full() -> anyhow::Result<()> {
        let input = ":nick!user@host PRIVMSG #chan :hello world\r\n";
        let msg = parse_pest_message(input)?;
        assert_eq!(
            msg.source,
            Some(Source {
                source: Some(SourceKind::Nick(&"nick")),
                host: Some(&"host"),
                user: Some(&"user")
            })
        );
        assert_eq!(msg.command, Some(&"PRIVMSG"[..]));
        assert_eq!(msg.parameters, vec![&"#chan"[..]]);
        assert_eq!(msg.trailing, Some(&"hello world"[..]));
        Ok(())
    }
}
