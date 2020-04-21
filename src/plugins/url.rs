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
    pub fn new() -> Self {
        let mut link_finder = linkify::LinkFinder::new();
        link_finder.kinds(&[linkify::LinkKind::Url]);

        UrlPlugin {
            link_finder,
            newline_re: Regex::new(r#"\s*\n\s*"#).unwrap(),
            title_selector: scraper::Selector::parse("title").unwrap(),
        }
    }
}

impl UrlPlugin {
    fn parse_title(&self, buf: &str) -> Option<String> {
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

impl UrlPlugin {
    async fn lookup_url_title(&self, ctx: &Context, arg: &str) -> Result<()> {
        let parsed_url = url::Url::parse(&arg)?;

        let mut buf = String::new();

        // Read in at most 1M of data
        let resp = reqwest::get(parsed_url).await?;

        // If it's not text/html, we want to return early
        if resp
            .headers()
            .get(http::header::CONTENT_TYPE)
            .map(|h| h.as_ref())
            .unwrap_or(b"")
            != b"text/html"
        {
            return Ok(());
        }

        resp.bytes_stream()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
            .into_async_read()
            .take(1024 * 1024)
            .read_to_string(&mut buf)
            .await?;

        if let Some(title) = self.parse_title(&buf) {
            ctx.reply(&format!("Title: {}", title.trim())).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Plugin for UrlPlugin {
    fn new_from_env() -> Result<Self> {
        Ok(UrlPlugin::new())
    }

    async fn run(self, _bot: Arc<Client>, mut stream: Receiver<Arc<Context>>) -> Result<()> {
        while let Some(ctx) = stream.next().await {
            let urls: Vec<_> = if let Event::Privmsg(_, msg) = ctx.as_event() {
                self.link_finder.links(msg).collect()
            } else {
                Vec::new()
            };

            if !urls.is_empty() {
                let res = futures::future::try_join_all(
                    urls.into_iter()
                        .map(|url| self.lookup_url_title(&ctx, url.as_str())),
                )
                .await;

                crate::check_err(&ctx, res).await;
            }
        }

        Err(format_err!("url plugin exited early"))
    }
}
