use std::fmt;
use std::io;
use std::net::ToSocketAddrs;

use tokio::codec::{Decoder, Encoder, Framed, LinesCodec, LinesCodecError};
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio_tls::TlsConnector;

use bytes::{BufMut, BytesMut};
use failure::Error;

#[derive(Debug)]
struct Message {
    tags: Option<String>,
    prefix: Option<String>,
    command: String,
    args: Vec<String>,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(tags) = &self.tags {
            write!(f, "@{} ", tags)?;
        }

        if let Some(prefix) = &self.prefix {
            write!(f, ":{} ", prefix)?;
        }

        write!(f, "{}", self.command)?;

        if self.args.len() > 0 {
            let args = &self.args[..self.args.len() - 1];
            let trailing = &self.args[self.args.len() - 1];

            for arg in args {
                write!(f, " {}", arg)?;
            }

            write!(f, " :{}", trailing)?;
        }

        Ok(())
    }
}

struct IRCCodec {
    inner: LinesCodec,
}

impl IRCCodec {
    pub fn new() -> IRCCodec {
        IRCCodec {
            inner: LinesCodec::new(),
        }
    }
}

impl Encoder for IRCCodec {
    type Item = Message;
    type Error = IRCError;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let line = item.to_string();
        dst.reserve(line.len() + 2);
        dst.put(line);
        dst.put("\r\n");
        Ok(())
    }
}

impl Decoder for IRCCodec {
    type Item = Message;
    type Error = IRCError;

    fn decode(&mut self, data: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Read a single line
        let data = self.inner.decode(data)?;
        if data.is_none() {
            return Ok(None);
        }

        let mut data = data.unwrap();

        // Parse out IRC tags
        let tags = if data.starts_with("@") {
            let tags_idx = data.find(" ");
            let tags = tags_idx.map(|i| String::from(&data[1..i]));
            data = tags_idx.map_or("", |i| &data[i + 1..]).to_string();
            tags
        } else {
            None
        };

        // Parse out the prefix
        let prefix = if data.starts_with(":") {
            let prefix_idx = data.find(" ");
            let prefix = prefix_idx.map(|i| String::from(&data[1..i]));
            data = prefix_idx.map_or("", |i| &data[i + 1..]).to_string();
            prefix
        } else {
            None
        };

        let line_ending_len = if data.ends_with("\r\n") {
            "\r\n"
        } else if data.ends_with('\r') {
            "\r"
        } else if data.ends_with('\n') {
            "\n"
        } else {
            ""
        }
        .len();

        let trailing_idx = data.find(" :");
        let trailing = if trailing_idx.is_some() {
            let trailing =
                trailing_idx.map(|i| String::from(&data[i + 2..data.len() - line_ending_len]));
            data = trailing_idx.map_or("", |i| &data[..i + 1]).to_string();
            trailing
        } else {
            data = String::from(&data[..data.len() - line_ending_len]);
            None
        };

        let command_idx = data.find(" ");
        let command = command_idx.map_or(&data[..], |i| &data[..i]).to_string();
        data = command_idx.map_or("", |i| &data[i + 1..]).to_string();

        let mut args: Vec<String> = data
            .split(" ")
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        if trailing.is_some() {
            args.push(trailing.unwrap());
        }

        return Ok(Some(Message {
            tags: tags,
            prefix: prefix,
            command: command,
            args: args,
        }));
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let stream = TcpStream::connect("irc.hs.gy:9999").await?;

    let connector = native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()?;
    let connector = TlsConnector::from(connector);

    let stream = connector.connect("irc.hs.gy", stream).await?;

    let mut lines = Framed::new(stream, IRCCodec::new());

    // In a loop, read data from the socket and write the data back.
    loop {
        let msg = lines.next().await;

        match msg.unwrap() {
            Ok(msg) => println!("{}: {}", msg.command, msg),
            Err(_err) => println!("Error"),
        };
    }
}

/// An error occured while encoding or decoding a line.
#[derive(Debug)]
pub enum IRCError {
    Lines(LinesCodecError),
    /// An IO error occured.
    Io(io::Error),
}

impl fmt::Display for IRCError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IRCError::Lines(e) => write!(f, "{}", e),
            IRCError::Io(e) => write!(f, "{}", e),
        }
    }
}

impl From<LinesCodecError> for IRCError {
    fn from(e: LinesCodecError) -> IRCError {
        IRCError::Lines(e)
    }
}

impl From<io::Error> for IRCError {
    fn from(e: io::Error) -> IRCError {
        IRCError::Io(e)
    }
}

impl std::error::Error for IRCError {}
