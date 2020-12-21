use std::borrow::Cow;
use std::fmt::Write;

use crate::prelude::*;
use crate::utils::HexSlice;

use trust_dns_resolver::{
    proto::rr::rdata::caa::Value as CAAValue, proto::rr::RData, proto::xfer::DnsRequestOptions,
    AsyncResolver,
};

pub struct NetToolsPlugin;

fn display_rdata(rdata: RData) -> String {
    match rdata {
        RData::A(addr) => format!("A {}", addr),
        RData::AAAA(addr) => format!("AAAA {}", addr),
        RData::ANAME(name) => format!("ANAME {}", name),
        RData::CAA(caa) => format!(
            "CAA {} {} {}",
            if caa.issuer_critical() { 1 } else { 0 },
            caa.tag().as_str(),
            match caa.value() {
                CAAValue::Issuer(name, kv) => {
                    // Note that we use unwrap here because the implementation
                    // of fmt::Write on String will always return Ok(())
                    let mut buf = String::new();
                    if let Some(name) = name {
                        write!(buf, "{}", name).unwrap();
                        if !kv.is_empty() {
                            buf.write_char(' ').unwrap();
                        }
                    }
                    if !kv.is_empty() {
                        write!(buf, "{:?}", kv).unwrap();
                    }
                    Cow::Owned(buf)
                }
                CAAValue::Url(url) => Cow::Owned(url.to_string()),
                CAAValue::Unknown(data) => String::from_utf8_lossy(data),
            },
        ),
        RData::CNAME(name) => format!("CNAME {}", name),
        RData::MX(mx) => format!("MX {} {}", mx.preference(), mx.exchange()),
        RData::NAPTR(_naptr) => unimplemented!(),
        RData::NULL(null) => format!(
            "NULL {}",
            String::from_utf8_lossy(null.anything().unwrap_or(b"")),
        ),
        RData::NS(name) => format!("NS {}", name),
        RData::OPENPGPKEY(_key) => unimplemented!(),
        RData::OPT(_opt) => unimplemented!(),
        RData::PTR(name) => format!("PTR {}", name),
        RData::SOA(soa) => format!(
            "SOA {} {} {} {} {} {} {}",
            soa.mname(),
            soa.rname(),
            soa.serial(),
            soa.refresh(),
            soa.retry(),
            soa.expire(),
            soa.minimum()
        ),
        RData::SRV(srv) => format!(
            "SRV {} {} {} {}",
            srv.priority(),
            srv.weight(),
            srv.port(),
            srv.target()
        ),
        RData::SSHFP(sshfp) => {
            let algorithm: u8 = sshfp.algorithm().into();
            let fingerprint_type: u8 = sshfp.fingerprint_type().into();
            format!(
                "SSHFP {} {} {}",
                algorithm,
                fingerprint_type,
                HexSlice::new(sshfp.fingerprint()),
            )
        }
        RData::TLSA(_tlsa) => unimplemented!(),
        RData::TXT(txt) => format!(
            "TXT {:?}",
            txt.txt_data()
                .iter()
                .map(|data| String::from_utf8_lossy(data))
                .collect::<Vec<_>>()
        ),
        RData::Unknown { code, rdata } => format!(
            "UNKNOWN ({}) {}",
            code,
            String::from_utf8_lossy(rdata.anything().unwrap_or(b"")),
        ),
        RData::ZERO => "ZERO".to_string(),
    }
}

impl NetToolsPlugin {
    async fn handle_dig(&self, ctx: &Context, arg: &str) -> Result<()> {
        let resolver = AsyncResolver::tokio_from_system_conf().await?;

        // There seems to be a bug in Rust where it can't infer the proper
        // lifetime, so we just convert everything to owned values to avoid
        // worrying about it.
        let mut iter = arg.splitn(2, ' ').map(String::from);
        let arg0 = iter.next();
        let arg1 = iter.next();

        let records: Vec<_> = match (arg0, arg1) {
            // If a record_type was provided, we need to try and
            // convert it.
            (Some(record_type), Some(name)) => resolver
                .lookup(
                    name,
                    record_type.to_uppercase().parse()?,
                    DnsRequestOptions::default(),
                )
                .await?
                .into_iter()
                .map(display_rdata)
                .collect(),

            // If they didn't provide a lookup type, default to A/AAAA
            // records.
            (Some(name), None) => resolver
                .lookup_ip(name)
                .await?
                .iter()
                .map(|ip| ip.to_string())
                .collect(),

            // It should be impossible to get no results from this
            // iterator as even the empty string will have that as the
            // first result.
            (None, None) => unreachable!(),
            (None, Some(_)) => unreachable!(),
        };

        for record in records {
            ctx.mention_reply(&record.to_string()).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Plugin for NetToolsPlugin {
    fn new_from_env() -> Result<Self> {
        Ok(NetToolsPlugin {})
    }

    fn command_metadata(&self) -> Vec<CommandMetadata> {
        vec![CommandMetadata {
            name: "dig".to_string(),
            short_help: "usage: dig [record_type] <domain>. resolves the given domain.".to_string(),
            full_help: "resolves the given domain. if no record_type is provided, assumes A/AAAA."
                .to_string(),
        }]
    }

    async fn run(self, bot: Arc<Client>) -> Result<()> {
        let mut stream = bot.subscribe();

        while let Ok(ctx) = stream.recv().await {
            let res = match ctx.as_event() {
                Ok(Event::Command("dig", Some(arg))) => self.handle_dig(&ctx, arg).await,
                Ok(Event::Command("dig", None)) => Err(anyhow::format_err!("Not enough arguments")),
                _ => Ok(()),
            };

            crate::check_err(&ctx, res).await;
        }

        Err(format_err!("net_tools plugin lagged"))
    }
}
