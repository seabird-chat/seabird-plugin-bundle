pub use std::collections::HashMap;
pub use std::pin::Pin;
pub use std::sync::Arc;

pub use anyhow::{format_err, Context as AnyhowContext};
pub use async_trait::async_trait;
pub use itertools::Itertools;
pub use tokio::stream::{Stream, StreamExt};
pub use unicode_segmentation::UnicodeSegmentation;

pub use crate::client::{Client, ClientConfig, Context, Event};
pub use crate::error::Result;
pub use crate::plugin::{CommandMetadata, Plugin};
pub use crate::proto;
pub use crate::proto::event::Inner as SeabirdEvent;
pub(crate) use crate::utils;
