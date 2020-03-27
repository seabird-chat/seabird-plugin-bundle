use std::io;

use futures::{AsyncReadExt, TryStreamExt};
use regex::Regex;

use crate::prelude::*;

pub struct UrlPlugin {
    link_finder: linkify::LinkFinder,
    newline_re: Regex,
    title_selector: scraper::Selector,
}

impl UrlPlugin {
    pub fn new() -> Arc<Self> {
        let mut link_finder = linkify::LinkFinder::new();
        link_finder.kinds(&[linkify::LinkKind::Url]);

        Arc::new(UrlPlugin {
            link_finder,
            newline_re: Regex::new(r#"\s*\n\s*"#).unwrap(),
            title_selector: scraper::Selector::parse("title").unwrap(),
        })
    }
}

impl UrlPlugin {
    fn parse_title(self: &Arc<Self>, buf: &str) -> Option<String> {
        let html = scraper::Html::parse_fragment(buf);

        let mut titles = html.select(&self.title_selector);
        if let Some(title_element) = titles.next() {
            if let Some(title) = title_element.text().next() {
                return Some(self.newline_re.replace_all(&title, " ").into_owned());
            }
        }

        None
    }
}

#[async_trait]
impl Plugin for Arc<UrlPlugin> {
    async fn handle_message(&self, ctx: &Arc<Context>) -> Result<()> {
        let urls: Vec<_> = if let Event::Privmsg(_, msg) = ctx.as_event() {
            self.link_finder
                .links(msg)
                .map(|link| link.as_str().to_string())
                .collect()
        } else {
            Vec::new()
        };

        if !urls.is_empty() {
            for url_str in urls {
                let ctx = (*ctx).clone();
                let plugin = (*self).clone();

                crate::spawn(async move {
                    let mut buf = String::new();

                    let parsed_url = url::Url::parse(&url_str)?;

                    // Read in at most 4k of data
                    reqwest::get(parsed_url)
                        .await?
                        .bytes_stream()
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
                        .into_async_read()
                        .take(1024 * 1024)
                        .read_to_string(&mut buf)
                        .await?;

                    if let Some(title) = plugin.parse_title(&buf) {
                        ctx.reply(&format!("Title: {}", title.trim())).await?;
                    }

                    Ok(())
                });
            }
        }

        Ok(())
    }
}
