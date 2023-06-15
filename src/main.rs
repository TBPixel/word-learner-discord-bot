#![forbid(unsafe_code)]
#![warn(
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::todo,
    clippy::unimplemented
)]
#![allow(clippy::use_self)] // disabling use_self lints due to a bug where proc-macro's (such as serde::Serialize) can trigger it to hinted on type definitions

use std::env;
use std::path::PathBuf;

use async_recursion::async_recursion;
use color_eyre::eyre::{Result, WrapErr};
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::prelude::*;
use serenity::prelude::*;
use words::WordDefinition;

mod words;

struct Handler {
    words_file_path: PathBuf,
    total_lines: u64,
}

impl Handler {
    fn new(file_path: &PathBuf) -> Result<Handler> {
        Ok(Handler {
            words_file_path: file_path.clone(),
            total_lines: words::get_line_count(file_path)?,
        })
    }
}

#[async_recursion]
async fn send_new_word(handler: &Handler, ctx: Context, msg: Message) {
    match words::get_random_word(&handler.words_file_path, handler.total_lines).await {
        Ok(w) => send_word(ctx, msg, w).await,
        Err(_) => send_new_word(handler, ctx, msg).await,
    }
}

async fn send_word(ctx: Context, msg: Message, w: WordDefinition) {
    let meanings = w
        .meanings
        .iter()
        .map(|m| {
            format!(
                "`{}`:\n  - {}",
                m.part_of_speech,
                m.definitions
                    .iter()
                    .map(|d| format!("{}", d.definition))
                    .collect::<String>()
            )
        })
        .collect::<String>();
    let body = format!("_**{}**_:\n{}", w.word, meanings);
    if let Err(e) = msg.channel_id.say(&ctx.http, body).await {
        println!("Error sending message: {:?}", e);
    }
}

#[async_trait]
impl EventHandler for Handler {
    // Set a handler for the `message` event - so that whenever a new message
    // is received - the closure (or function) passed will be called.
    //
    // Event handlers are dispatched through a threadpool, and so multiple
    // events can be dispatched simultaneously.
    async fn message(&self, ctx: Context, msg: Message) {
        if !msg.mentions_me(&ctx.http).await.unwrap_or(false) {
            return;
        }

        let mut command: Vec<&str> = msg.content.split(" ").collect();
        command.remove(0);
        if let Some(cmd) = command.first() {
            match *cmd {
                "help" => {
                    if let Err(e) = msg
                        .channel_id
                        .say(
                            &ctx.http,
                            format!(
                                "`help` - This help message.
`new` - Gives you a new word of the day.
`define <word>` - Pulls up the definition for a given word.
`worddies` - Registers for a daily word in this channel."
                            ),
                        )
                        .await
                    {
                        println!("Error sending message: {:?}", e);
                    }
                }
                "new" => send_new_word(self, ctx, msg).await,
                "define" => match command.get(1) {
                    Some(input) => match words::get_word(input).await {
                        Ok(w) => send_word(ctx, msg, w).await,
                        Err(e) => println!("unexpected error: {:?}", e),
                    },
                    None => println!("<word> input is required!"),
                },
                input => println!("unknown command {}", input),
            }
        }
    }

    // Set a handler to be called on the `ready` event. This is called when a
    // shard is booted, and a READY payload is sent by Discord. This payload
    // contains data like the current user's guild Ids, current user data,
    // private channels, and more.
    //
    // In this case, just print what the current user's username is.
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // error tracing
    color_eyre::install()?;

    // load .env file; choose not to handle errors as .env file is only a convenience
    dotenvy::dotenv().ok();

    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").wrap_err("Expected a token in the environment")?;
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let handler = Handler::new(&env::current_dir()?.join("words_alpha.txt"))?;
    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await
        .wrap_err("Err creating client")?;

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }

    Ok(())
}
