pub use std::sync::Arc;

pub use anyhow::format_err;
pub use async_trait::async_trait;
pub use itertools::Itertools;
pub use tokio::stream::{Stream, StreamExt};
pub use tokio::sync::mpsc::Receiver;

pub use irc::Message;

pub use crate::client::{Client, ClientConfig, ClientState, Context};
pub use crate::error::Result;
pub use crate::event::Event;
pub use crate::plugin::Plugin;
