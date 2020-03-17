use std::io;

use futures::stream::TryStreamExt;
use quick_xml::{events::Event as XmlEvent, Reader};
use regex::Regex;
use tokio::io::AsyncReadExt;

use crate::prelude::*;
use crate::utils::StreamReader;

pub struct UrlPlugin {
    re: Regex,
    newline_re: Regex,
}

impl UrlPlugin {
    pub fn new() -> Arc<Self> {
        Arc::new(UrlPlugin {
            re: Regex::new(r#"https?://[^ ]*[^ ?]+"#).unwrap(),
            newline_re: Regex::new(r#"\s*\n\s*"#).unwrap(),
        })
    }
}

#[async_trait]
impl Plugin for Arc<UrlPlugin> {
    async fn handle_message(&self, ctx: &Arc<Context>) -> Result<()> {
        let urls: Vec<_> = if let Event::Privmsg(_, msg) = ctx.as_event() {
            self.re
                .captures_iter(msg)
                .map(|c| String::from(&c[0]))
                .collect()
        } else {
            Vec::new()
        };

        if !urls.is_empty() {
            let ctx = (*ctx).clone();
            let plugin = (*self).clone();

            crate::spawn(async move {
                for url in urls {
                    let body = reqwest::get(&url)
                        .await?
                        .bytes_stream()
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e));

                    // Read in at most 4k of data
                    let mut buf = String::new();
                    StreamReader::new(body)
                        .take(4096)
                        .read_to_string(&mut buf)
                        .await?;

                    let mut xml_buf = Vec::new();
                    let mut reader = Reader::from_str(&buf[..]);
                    reader.trim_text(true);

                    loop {
                        match reader.read_event(&mut xml_buf)? {
                            XmlEvent::Start(ref e) if e.name() == b"title" => {
                                let mut text_buf = Vec::new();
                                let title_buf = reader.read_text(e.name(), &mut text_buf)?;
                                let title = plugin.newline_re.replace_all(&title_buf, " ");
                                ctx.reply(&format!("Title: {}", title.trim())).await?;
                                return Ok(());
                            }

                            // We actually want to ignore most event types.
                            XmlEvent::DocType(_)
                            | XmlEvent::PI(_)
                            | XmlEvent::Comment(_)
                            | XmlEvent::CData(_)
                            | XmlEvent::Decl(_)
                            | XmlEvent::Start(_)
                            | XmlEvent::End(_)
                            | XmlEvent::Empty(_)
                            | XmlEvent::Text(_) => {}

                            XmlEvent::Eof => {
                                warn!("Failed to get title for {}", url);
                                break;
                            }
                        };
                    }
                }

                Ok(())
            });
        }

        Ok(())
    }
}
