#[non_exhaustive]
pub enum Event<'a> {
    // PRIVMSG target :msg
    Privmsg(&'a str, &'a str),

    // 001 nick :welcome text
    RplWelcome(&'a str, &'a str),

    // PRIVMSG somewhere !command arg
    Command(&'a str, Option<&'a str>),

    // If it didn't match anything else, it falls back to Raw.
    Raw(&'a str, Vec<&'a str>),
}

impl<'a> Event<'a> {
    pub fn from_message(command_prefix: char, msg: &'a irc::Message) -> Self {
        match (&msg.command[..], msg.params.len()) {
            ("PRIVMSG", 2) => {
                let message = &msg.params[1][..];
                if message.starts_with(command_prefix) {
                    let mut parts = message[1..].splitn(2, ' ');
                    let cmd = parts.next().unwrap_or("");
                    let arg = parts.next().unwrap_or("").trim();
                    Event::Command(cmd, if arg.is_empty() { None } else { Some(arg) })
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
