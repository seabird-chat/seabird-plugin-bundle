use bytes::BytesMut;
use tokio_util::codec::{Decoder, LinesCodec};

use crate::Message;

pub(crate) struct IrcCodec {
    inner: LinesCodec,
}

impl IrcCodec {
    pub fn new() -> Self {
        IrcCodec {
            inner: LinesCodec::new(),
        }
    }
}

impl Decoder for IrcCodec {
    type Error = anyhow::Error;
    type Item = Message;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if let Some(line) = self.inner.decode(src)? {
            Ok(Some(line.parse()?))
        } else {
            Ok(None)
        }
    }
}
