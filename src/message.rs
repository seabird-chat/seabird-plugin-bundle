use std::fmt;
use std::str::FromStr;

pub struct Message(pub irc::Message);

impl Message {
    pub fn as_command(&self) -> Command<'_> {
        self.into()
    }
}

impl FromStr for Message {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Ok(Message(input.parse()?))
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[non_exhaustive]
pub enum Command<'a> {
    // Privmsg(target, msg)
    Privmsg(&'a str, &'a str),

    // If it didn't match anything else, it falls back to Raw.
    Raw(&'a str, Vec<&'a str>),
}

impl<'a> From<&'a Message> for Command<'a> {
    fn from(m: &'a Message) -> Self {
        match (&m.0.command[..], m.0.params.len()) {
            ("PRIVMSG", 2) => Command::Privmsg(&m.0.params[0][..], &m.0.params[1][..]),
            _ => Command::Raw(
                &m.0.command[..],
                m.0.params.iter().map(|s| s.as_str()).collect(),
            ),
        }
    }
}
