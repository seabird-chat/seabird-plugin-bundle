use crate::prelude::*;

#[non_exhaustive]
pub enum Event<'a> {
    // PRIVMSG target :msg
    Message(&'a str, &'a str),
    PrivateMessage(&'a str, &'a str),

    // PRIVMSG somewhere :!command arg
    Command(&'a str, Option<&'a str>),

    // PRIVMSG somewhere :seabird: arg
    Mention(&'a str),

    Unknown(&'a SeabirdEvent),
}

impl<'a> From<&'a SeabirdEvent> for Event<'a> {
    fn from(event: &'a SeabirdEvent) -> Self {
        match event {
            SeabirdEvent::Message(msg) => {
                Event::Message(msg.reply_to.as_str(), msg.message.as_str())
            }
            SeabirdEvent::PrivateMessage(msg) => {
                Event::PrivateMessage(msg.reply_to.as_str(), msg.message.as_str())
            }
            SeabirdEvent::Command(msg) => {
                let inner = msg.arg.trim();
                Event::Command(
                    msg.command.as_str(),
                    if inner.is_empty() { None } else { Some(inner) },
                )
            }
            SeabirdEvent::Mention(msg) => Event::Mention(msg.message.as_str()),

            #[allow(unreachable_patterns)]
            event => Event::Unknown(event),
        }
    }
}
