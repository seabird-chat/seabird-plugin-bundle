#[non_exhaustive]
pub enum Event<'a> {
    // Privmsg(target, msg)
    Privmsg(&'a str, &'a str),

    RPL_WELCOME(&'a str, &'a str),

    // If it didn't match anything else, it falls back to Raw.
    Raw(&'a str, Vec<&'a str>),
}

impl<'a> From<&'a irc::Message> for Event<'a> {
    fn from(msg: &'a irc::Message) -> Self {
        match (&msg.command[..], msg.params.len()) {
            ("PRIVMSG", 2) => Event::Privmsg(&msg.params[0][..], &msg.params[1][..]),
            ("001", 2) => Event::RPL_WELCOME(&msg.params[0][..], &msg.params[1][..]),
            _ => Event::Raw(
                &msg.command[..],
                msg.params.iter().map(|s| s.as_str()).collect(),
            ),
        }
    }
}
