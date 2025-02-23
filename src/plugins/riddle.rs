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
    // r/riddles
    ("I have a tail, and I have a head, but i have no body. I am NOT a snake. What am I?", "a coin"),
    ("What falls, but does not break, and what breaks but does not fall?", "night and day"),
    ("Where may you find roads without carts, forests without trees, cities without houses?", "on a map"),
    ("What crosses the river but doesn't move?", "a bridge"),
    ("What pine has the longest and sharpest needles?", "a porcupine"),
    ("What knows all languages?", "an echo"),
    ("What turns everything around but does not move?", "a mirror"),
    ("Turn us on our backs and open our stomachs, you will be the wisest of men, though at the start a lummox. What am I?", "a book"),
    ("A long snake with a stinging bite, I stay coiled up unless I must fight.", "a whip"),
    ("I'm rarely touched but often held. If you have wit, you'll use me well.", "tongue"),
    ("The man who invented it doesn't want it. The man who bought it doesn't need it. The man who needs it doesn't know it. What is it?", "a coffin"),
    ("What's black when you get it, red when you use it, and white when you're done with it?", "charcoal"),
    ("What's Black and Blue and Red in between, can never be touched but can only be seen?", "the sky"),
    ("What always runs but never walks, often murmurs, never talks, has a bed but never sleeps, has a mouth but never eats?", "a river"),
    ("Forwards I'm heavy, backwards I'm not. What am I?", "a ton"),
    ("With pointed fangs it sits in wait. With piercing force it doles out fate, Over bloodless victims proclaiming its might. Eternally joining in a single bite", "a stapler"),
    ("Tall I am young, short I am old. While with life I do glow, and wind is my foe.", "a candle"),
    ("What flies forever but never rests?", "the wind"),
    ("Capable of Kindness and cruelty, I take victims when I sour. I can be on your side or wrong you. I bring gifts though you already have me.", "fate"),
    ("The beginning of eternity, the end of time and space; the beginning of every end, the end if every place. What am I?", "the letter E"),
    ("Lose me once I'll come back stronger, lose me twice I'll leave forever, what am I?", "a tooth"),
    ("We sound like Eden as a pair. Make us weight, we won't play fair. Sometimes consensus, most times schism. Usually locked away in prism. If by chance you seek, then throw. The serpent sees where we meet low. We carry freight when we meet high, But separate us, and we die.", "a pair of dice"),
    ("As a stone inside a tree, I'll help your words outlive thee. But if you push me as I stand, the more I move the less I am.", "a pencil"),
    ("You do not want me to be permanent, but to avoid me is a mistake. You can let me help you, but precious time it will take.", "sleep"),
    ("If you are to keep it, you must first give it to me.", "your word"),
    ("I can bring a tear to the eye, I can resurrect the dead. I am formed in an instant, and kept for a lifetime. What am I?", "a memory"),
    ("I have power enough to smash ships and crush roofs, yet I still fear the sun.", "ice"),
    ("I run forever, but never move at all. Though I have neither lungs or throat, I make a roaring call.", "a waterfall"),
    ("I'm a way above the water. I touch it not, but I neither swim nor move.", "a bridge"),
    ("What can run but never walks, has a mouth but never talks, has a bed but never sleeps, has a head but never weeps?", "a river"),
    ("I have an eye, but I cannot see. I am chaos at the fringe, but calm at my core. I live for a while, but I die soon enough, and when you least expect it, I am reborn.", "a hurricane"),
    ("Weight in my belly, trees on my back, nails in my ribs, but feet I do lack.", "a boat"),
    ("What ship has no captain but two mates?", "courtship"),
    ("What can be swallowed but can also swallow you?", "water"),
    ("What is it that you keep when you need it not, but throw out when you do need it?", "an anchor"),
    ("In the form of fork or sheet, I hit the ground. And if you wait a heartbeat, you can hear my roaring sound.", "lightning"),
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
