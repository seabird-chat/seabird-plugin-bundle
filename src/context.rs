use tokio::sync::mpsc;

pub(crate) struct Context {
    pub msg: irc::Message,
    sender: mpsc::Sender<String>,
}

impl Context {
    pub fn new(msg: irc::Message, sender: mpsc::Sender<String>) -> Self {
        Context { msg, sender }
    }

    pub async fn send(&mut self, command: &str, params: Vec<&str>) -> Result<(), anyhow::Error> {
        self.send_msg(&irc::Message::new(
            command.to_string(),
            params.into_iter().map(|s| s.to_string()).collect(),
        ))
        .await
    }

    pub async fn send_msg(&mut self, msg: &irc::Message) -> Result<(), anyhow::Error> {
        self.sender.send(msg.to_string()).await?;
        Ok(())
    }
}
