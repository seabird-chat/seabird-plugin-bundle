use std::sync::Arc;

use async_trait::async_trait;
use rand::{thread_rng, Rng};
use tokio::sync::Mutex;

use crate::prelude::*;

pub struct ChancePlugin {
    gun_size: u8,
    shots_left: Arc<Mutex<u8>>,
}

impl ChancePlugin {
    pub fn new() -> Self {
        ChancePlugin {
            gun_size: 6,
            shots_left: Arc::new(Mutex::new(0)),
        }
    }
}

#[async_trait]
impl Plugin for ChancePlugin {
    async fn handle_message(&self, ctx: &Arc<Context>) -> Result<()> {
        if let Event::Command("roulette", _) = ctx.as_event() {
            let (reloaded, shot) = {
                let mut shots_left = self.shots_left.lock().await;
                let reloaded = if *shots_left == 0 {
                    let mut rng = thread_rng();
                    *shots_left = rng.gen_range(1, self.gun_size + 1);
                    true
                } else {
                    false
                };

                *shots_left -= 1;

                println!("Shots: {}", *shots_left);

                (reloaded, *shots_left == 0)
            };

            let msg = if shot { "BANG!" } else { "Click." };
            if reloaded {
                ctx.mention_reply(&format!("Reloading the gun... {}", msg)[..])
                    .await?;
            } else {
                ctx.mention_reply(msg).await?;
            }
        }

        Ok(())
    }
}
