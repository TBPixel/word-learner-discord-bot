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
use std::path::{Path, PathBuf};

use async_recursion::async_recursion;
use color_eyre::eyre::{Result, WrapErr};
use redis::AsyncCommands;
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
    redis: redis::Client,
}

impl Handler {
    fn new(file_path: &Path, redis: redis::Client) -> Result<Handler> {
        Ok(Handler {
            words_file_path: file_path.to_path_buf(),
            total_lines: words::get_line_count(file_path)?,
            redis,
        })
    }
}

#[async_recursion]
async fn send_new_word(handler: &Handler, ctx: Context, msg: Message) -> Result<()> {
    match words::get_random_word(&handler.words_file_path, handler.total_lines).await {
        Ok(w) => send_word(handler, ctx, msg, w).await,
        Err(_) => send_new_word(handler, ctx, msg).await,
    }
}

async fn send_word(handler: &Handler, ctx: Context, msg: Message, w: WordDefinition) -> Result<()> {
    let meanings = w
        .meanings
        .iter()
        .map(|m| {
            format!(
                "`{}`:{}",
                m.part_of_speech,
                m.definitions
                    .iter()
                    .map(|d| format!("\n- {}", d.definition))
                    .collect::<String>()
            )
        })
        .collect::<String>();

    let mut conn = handler.redis.get_async_connection().await?;
    let nickname: Option<String> = conn.get(format!("nickname:{}", msg.author.id)).await.ok();
    let formality = match nickname {
        Some(name) => format!("\n\nDoes that help, {name}?"),
        None => String::new(),
    };
    let body = format!("_**{}**_:\n{}\n{}", w.word, meanings, formality);
    if let Err(e) = msg.channel_id.say(&ctx.http, body).await {
        println!("Error sending message: {:?}", e);
    }

    Ok(())
}

#[async_trait]
impl EventHandler for Handler {
    // Set a handler for the `message` event - so that whenever a new message
    // is received - the closure (or function) passed will be called.
    //
    // Event handlers are dispatched through a threadpool, and so multiple
    // events can be dispatched simultaneously.
    async fn message(&self, ctx: Context, msg: Message) {
        let is_bot_mentioned = msg.mentions_me(&ctx.http).await.unwrap_or(false);
        if !is_bot_mentioned {
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
`worddies` - Registers for a daily word in this channel.
`nickname <name>` - Sets a nickname for the bot to call you."
                            ),
                        )
                        .await
                    {
                        println!("Error sending message: {:?}", e);
                    }
                }
                "new" => send_new_word(self, ctx, msg).await.unwrap(),
                "define" => match command.get(1) {
                    Some(input) => match words::get_word(input).await {
                        Ok(w) => send_word(self, ctx, msg, w).await.unwrap(),
                        Err(e) => println!("unexpected error: {:?}", e),
                    },
                    None => println!("<word> input is required!"),
                },
                "nickname" => match msg.content.split("nickname").last() {
                    Some(nickname) => {
                        let name = nickname
                            .replace("#", "")
                            .replace(r"\n", "")
                            .trim()
                            .to_string();

                        if name.contains("http") {
                            if let Err(e) = msg
                                .channel_id
                                .say(&ctx.http, "**Error**: A nickname cannot contain a link!")
                                .await
                            {
                                println!("Error sending message: {:?}", e);
                            }
                            return;
                        }
                        // we allow exactly one mention because the bot has to be
                        // mentioned to reply.
                        if msg.mentions.len() != 1 {
                            if let Err(e) = msg
                                .channel_id
                                .say(&ctx.http, "**Error**: A nickname cannot mention anyone!")
                                .await
                            {
                                println!("Error sending message: {:?}", e);
                            }
                            return;
                        }

                        let mut conn = self.redis.get_async_connection().await.unwrap();
                        let _: () = conn
                            .set(format!("nickname:{}", msg.author.id), &name)
                            .await
                            .unwrap();

                        if let Err(e) = msg.channel_id.say(&ctx.http, format!("Hi {name}!")).await {
                            println!("Error sending message: {:?}", e);
                        }
                    }
                    None => println!("<nickname> is required!"),
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

    // Redis client
    let redis_client = redis::Client::open(
        env::var("REDIS_CONN").wrap_err("Expected a REDIS_CONN in the environment")?,
    )?;

    // Configure the client with your Discord bot token in the environment.
    let token =
        env::var("DISCORD_TOKEN").wrap_err("Expected a DISCORD_TOKEN in the environment")?;
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let handler = Handler::new(&env::current_dir()?.join("words_alpha.txt"), redis_client)?;
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
