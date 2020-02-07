use std::sync::Arc;

use crate::client::Context;
use crate::error::Result;

use async_trait::async_trait;

#[async_trait]
pub trait Plugin: Sync + Send {
    async fn handle_connect(&self) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&self, _ctx: &Arc<Context>) -> Result<()> {
        Ok(())
    }
}
