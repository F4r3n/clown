use crate::command::Command;

#[derive(Debug)]
pub enum Reply {
    Cmd(Command),
    Rpl(ReplyNumber),
}
#[derive(Debug)]

pub enum ReplyNumber {
    WELCOME(String),
}

pub struct ReplyBuilder;

impl ReplyBuilder {
    fn make_reply_1<F>(parameters: Vec<&str>, ctor: F) -> Option<ReplyNumber>
    where
        F: Fn(String) -> ReplyNumber,
    {
        parameters.first().map(|target| ctor(target.to_string()))
    }

    pub fn get_reply(reply_number: u16, trailing: Vec<&str>) -> Option<ReplyNumber> {
        match reply_number {
            1 => ReplyBuilder::make_reply_1(trailing, ReplyNumber::WELCOME),
            _ => None,
        }
    }
}
