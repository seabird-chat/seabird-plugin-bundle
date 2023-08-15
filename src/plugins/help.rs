use crate::prelude::*;

pub struct HelpPlugin {}

impl HelpPlugin {
    pub fn new() -> Self {
        Self {}
    }

    async fn handle_help(&self, ctx: &Context, argument: Option<&str>) -> Result<()> {
        let response = ctx.registered_commands().await?;

        match argument {
            Some(command) => match response.commands.get(command) {
                Some(metadata) => {
                    ctx.mention_reply(&metadata.short_help).await?;
                }
                None => {
                    ctx.mention_reply(&format!("unknown command \"{}\"", command))
                        .await?;
                }
            },
            None => {
                // Print all available commands
                let mut commands: Vec<String> = response.commands.into_keys().collect();
                commands.sort();

                ctx.mention_reply(&format!("available commands: {}", commands.join(", ")))
                    .await?;
                ctx.mention_reply("run \"help <command>\" for more information on a command")
                    .await?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Plugin for HelpPlugin {
    fn new_from_env() -> Result<Self> {
        Ok(HelpPlugin::new())
    }

    fn command_metadata(&self) -> Vec<CommandMetadata> {
        vec![CommandMetadata {
            name: "help".to_string(),
            short_help: "usage: help <command>. gives usage and help for a command.".to_string(),
            full_help: "gives usage and other information for a command.".to_string(),
        }]
    }

    async fn run(self, bot: Arc<Client>) -> Result<()> {
        let mut stream = bot.subscribe();

        while let Ok(ctx) = stream.recv().await {
            let res = match ctx.as_event() {
                Ok(Event::Command("help", arg)) => self.handle_help(&ctx, arg).await,
                _ => Ok(()),
            };

            crate::check_err(&ctx, res).await;
        }

        Err(format_err!("help plugin lagged"))
    }
}
