use crate::prelude::*;

#[non_exhaustive]
pub enum Event<'a> {
    // PRIVMSG target :msg
    Privmsg(&'a str, &'a str),

    // 001 nick :welcome text
    RplWelcome(&'a str, &'a str),

    // PRIVMSG somewhere :!command arg
    Command(&'a str, Option<&'a str>),

    // PRIVMSG somewhere :seabird: arg
    Mention(&'a str),

    // If it didn't match anything else, it falls back to Raw.
    Raw(&'a str, Vec<&'a str>),
}

impl<'a> Event<'a> {
    pub fn from_message(state: Arc<ClientState>, msg: &'a irc::Message) -> Self {
        match (&msg.command[..], msg.params.len()) {
            ("PRIVMSG", 2) => {
                let message = &msg.params[1][..];
                if message.starts_with(&state.config.command_prefix) {
                    let mut parts = message[state.config.command_prefix.len()..].splitn(2, ' ');
                    let cmd = parts.next().unwrap_or("");
                    let arg = parts.next().unwrap_or("").trim();
                    Event::Command(cmd, if arg.is_empty() { None } else { Some(arg) })
                } else if message.starts_with(&state.current_nick)
                    && message[state.current_nick.len()..].starts_with(':')
                {
                    let arg = &message[state.current_nick.len() + 1..].trim();
                    Event::Mention(arg)
                } else {
                    Event::Privmsg(&msg.params[0][..], message)
                }
            }
            ("001", 2) => Event::RplWelcome(&msg.params[0][..], &msg.params[1][..]),
            _ => Event::Raw(
                &msg.command[..],
                msg.params.iter().map(|s| s.as_str()).collect(),
            ),
        }
    }
}
