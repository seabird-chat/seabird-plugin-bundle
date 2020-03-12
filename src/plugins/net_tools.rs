use std::sync::Arc;

use crate::prelude::*;

use trust_dns_resolver::{
    proto::rr::RecordType, proto::xfer::DnsRequestOptions, AsyncResolver, IntoName,
};

pub struct NetToolsPlugin {}

impl NetToolsPlugin {
    pub fn new() -> Self {
        NetToolsPlugin {}
    }
}

#[async_trait]
impl Plugin for NetToolsPlugin {
    async fn handle_message(&self, ctx: &Arc<Context>) -> Result<()> {
        match ctx.as_event() {
            Event::Command("dig", Some(arg)) => {
                let resolver = AsyncResolver::tokio_from_system_conf().await?;

                let mut args = arg.splitn(2, " ");
                let (name, record_type) = match (args.next(), args.next()) {
                    (Some(record_type), Some(name)) => (name.into_name()?, record_type.parse()?),
                    (Some(name), None) => (name.into_name()?, RecordType::A),
                    _ => unreachable!(),
                };

                for record in resolver
                    .lookup(name, record_type, DnsRequestOptions::default())
                    .await?
                    .iter()
                {
                    ctx.mention_reply(&format!("{:?}", record)).await?;
                }
            }
            Event::Command("dig", None) => {}
            _ => {}
        }

        Ok(())
    }
}
