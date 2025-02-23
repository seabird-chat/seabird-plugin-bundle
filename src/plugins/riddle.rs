use crate::prelude::*;
use rand::seq::SliceRandom;
use tokio::sync::Mutex;

pub struct RiddlePlugin {
    riddle_answers: Mutex<HashMap<String, &'static str>>,
}

const RIDDLES: &[(&str, &str)] = &[
    // The Hobbit or There and Back Again by J.R.R. Tolkien - Chapter 5: Riddles in the Dark
    ("What has roots as nobody sees, is taller than trees Up, up, up it goes, and yet never grows?", "a mountain"),
    ("Thirty white horses on a red hill, first they champ, then they stamp, then they stand still.", "teeth"),
    ("Voiceless it cries, wingless flutters, toothless bites, mouthless mutters.", "the wind"),
    ("An eye in a blue face, saw an eye in a green face. 'That eye is like to this eye', said the first eye, 'But in low place, not in high place.'", "sun shining on daisies"),
    ("It cannot be seen, cannot be felt, cannot be heard, cannot be smelt. It lies behind stars and under hills, and empty holes it fills. It comes first and follows after, ends life, kills laughter.", "darkness"),
    ("A box without hinges, key or lid, yet golden treasure inside is hid.", "an egg"),
    ("Alive without breath, as cold as death; never thirsty, ever drinking, all in mail never clinking", "a fish"),
    ("No-legs lay on one-leg, two legs sat near on three legs, four legs got some.", "a fish on a little one-legged table, man at table sitting on a three-legged stool, the cat gets the bones"),
    ("This thing all things devours: birds, beasts, trees, flowers; Gnaws iron, bites steel; Grinds hard stones to meal; Slays king, ruins town, and beats high mountain down", "time"),
];

impl RiddlePlugin {
    async fn handle_riddle_ask(&self, ctx: &Arc<Context>) -> Result<()> {
        let target = ctx.sender().unwrap_or_else(|| "someone");
        let riddle = RIDDLES.choose(&mut rand::thread_rng()).unwrap();
        ctx.action_reply(&format!("asks {}: {}", target, riddle.0))
            .await?;

        if let Some(channel_id) = ctx.target_channel_id() {
            self.riddle_answers
                .lock()
                .await
                .insert(String::from(channel_id), riddle.1);
        }

        Ok(())
    }

    async fn handle_riddle_answer(&self, ctx: &Arc<Context>) -> Result<()> {
        if let Some(channel_id) = ctx.target_channel_id() {
            if let Some(previous_answer) = self.riddle_answers.lock().await.remove(channel_id) {
                ctx.action_reply(&format!("answers: {}", previous_answer))
                    .await?;
            } else {
                ctx.action_reply(&format!("cannot remember...")).await?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Plugin for RiddlePlugin {
    fn new_from_env() -> Result<Self> {
        Ok(RiddlePlugin {
            riddle_answers: Default::default(),
        })
    }

    fn command_metadata(&self) -> Vec<CommandMetadata> {
        vec![
            CommandMetadata {
                name: "riddle".to_string(),
                short_help: "usage: riddle. Get a riddle from the bot.".to_string(),
                full_help: "gives a random riddle".to_string(),
            },
            CommandMetadata {
                name: "answer".to_string(),
                short_help: "usage: answer. Get the answer to the riddle.".to_string(),
                full_help: "gives the answer to the last given riddle".to_string(),
            },
        ]
    }

    async fn run(self, bot: Arc<Client>) -> Result<()> {
        let mut stream = bot.subscribe();

        while let Ok(ctx) = stream.recv().await {
            let res = match ctx.as_event() {
                Ok(Event::Command("riddle", _arg)) => self.handle_riddle_ask(&ctx).await,
                Ok(Event::Command("answer", _arg)) => self.handle_riddle_answer(&ctx).await,
                _ => Ok(()),
            };

            crate::check_err(&ctx, res).await;
        }

        Err(format_err!("riddle plugin lagged"))
    }
}
