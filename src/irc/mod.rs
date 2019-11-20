#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::str::FromStr;

use tokio::codec::{Decoder, Encoder, LinesCodec, LinesCodecError};

use bytes::{BufMut, BytesMut};

#[derive(Debug)]
pub struct Message {
    pub tags: BTreeMap<String, String>,
    pub prefix: Option<String>,
    pub command: String,
    pub params: Vec<String>,
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

        if self.params.len() > 0 {
            let params = &self.params[..self.params.len() - 1];
            let trailing = &self.params[self.params.len() - 1];

            for param in params {
                write!(f, " {}", param)?;
            }

            write!(f, " :{}", trailing)?;
        }

        Ok(())
    }
}

impl FromStr for Message {
    type Err = Error;

    fn from_str(s: &str) -> Result<Message, Self::Err> {
        let mut data = s;

        let mut tags = BTreeMap::new();

        // Parse out IRC tags
        if data.starts_with('@') {
            let tags_idx = data.find(' ');
            let tags = tags_idx.map(|i| data[1..i].to_string());
            data = tags_idx.map_or("", |i| &data[i + 1..]);
        }

        // Parse out the prefix
        let prefix = if data.starts_with(':') {
            let prefix_idx = data.find(' ');
            let prefix = prefix_idx.map(|i| data[1..i].to_string());
            data = prefix_idx.map_or("", |i| &data[i + 1..]);
            prefix
        } else {
            None
        };

        let trailing_idx = data.find(" :");
        let trailing = if let Some(trailing_idx) = trailing_idx {
            let trailing = data[trailing_idx + 2..data.len()].to_string();
            data = &data[..trailing_idx + 1];
            Some(trailing)
        } else {
            None
        };

        // If we found a space, the command is everything before the space.
        // Otherwise, it's the whole string.
        let command_idx = data.find(' ');
        let command = command_idx.map_or(&data[..], |i| &data[..i]).to_string();
        data = command_idx.map_or("", |i| &data[i + 1..]);

        let mut params: Vec<String> = data
            .split(" ")
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        if let Some(trailing) = trailing {
            params.push(trailing);
        }

        return Ok(Message {
            tags,
            prefix,
            command,
            params,
        });
    }
}

pub struct Codec {
    inner: LinesCodec,
}

impl Codec {
    pub fn new() -> Codec {
        Codec {
            inner: LinesCodec::new(),
        }
    }
}

impl Encoder for Codec {
    type Item = Message;
    type Error = Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let line = item.to_string();
        dst.reserve(line.len() + 2);
        dst.put(line);
        dst.put("\r\n");
        Ok(())
    }
}

impl Decoder for Codec {
    type Item = Message;
    type Error = Error;

    fn decode(&mut self, data: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Read a single line
        let data = self.inner.decode(data)?;

        match data {
            Some(data) => Ok(Some(data.parse::<Message>()?)),
            None => Ok(None),
        }
    }
}

/// An error occured while encoding or decoding a line.
#[derive(Debug)]
pub enum Error {
    Lines(LinesCodecError),
    /// An IO error occured.
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Lines(e) => write!(f, "{}", e),
            Error::Io(e) => write!(f, "{}", e),
        }
    }
}

impl From<LinesCodecError> for Error {
    fn from(e: LinesCodecError) -> Error {
        Error::Lines(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

impl std::error::Error for Error {}
