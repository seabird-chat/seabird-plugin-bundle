use tokio::sync::mpsc;
use tokio::time::{delay_for, Duration};

struct Message {
    command: String,
}

impl<'a> From<irc::Message<'a>> for Message {
    fn from(msg: irc::Message<'a>) -> Self {
        Message {
            command: msg.command.to_string(),
        }
    }
}

async fn read_task() -> Result<(), anyhow::Error> {
    Ok(())
}

async fn send_task(mut msgs: mpsc::Receiver<String>) -> Result<(), anyhow::Error> {
    loop {
        delay_for(Duration::from_secs(1)).await;
        match msgs.recv().await {
            Some(line) => println!("GOT LINE: {}", line),
            None => anyhow::bail!("Dead send queue"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let (mut tx_send, rx_send) = mpsc::channel(100);

    let read = tokio::spawn(read_task());
    let send = tokio::spawn(send_task(rx_send));

    tx_send.send("NICK seabird51".to_string()).await?;
    tx_send.send(format!("USER seabird 0.0.0.0 0.0.0.0 :Seabird Bot")).await?;

    match tokio::try_join!(read, send) {
        Err(e) => {
            println!("{}", e);
            Ok(())
        }
        Ok(_) => Ok(()),
    }
}
