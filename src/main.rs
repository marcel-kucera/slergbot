use std::env;
use std::io::ErrorKind;
use std::ops::DerefMut;
use std::time::Duration;

use dotenvy::dotenv;
use jshell::{JShell, JShellError};
use rand::Rng;
use serenity::all::CreateMessage;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;
use thiserror::Error;
use tokio::time::{error::Elapsed, timeout};

mod jshell;

struct Handler {
    jshell: Mutex<JShell>,
}

#[derive(Error, Debug)]
enum AppError {
    #[error("discord error: {0:?}")]
    DiscordError(#[from] SerenityError),

    #[error("jshell error: {0}")]
    JShellError(#[from] JShellError),

    #[error("timeout")]
    TimeoutError(#[from] Elapsed),
}

impl Handler {
    async fn revive_jshell(&self) -> Result<(), AppError> {
        let instance = JShell::new().await?;

        // replace the jshell instance behind the mutex
        let mut lock = self.jshell.lock().await;
        let jshell = lock.deref_mut();
        *jshell = instance;

        Ok(())
    }

    async fn message_handler(&self, ctx: &Context, msg: &Message) -> Result<(), AppError> {
        // do not reply to self
        if msg.author.bot {
            return Ok(());
        }

        // Slerg whisper
        if msg.content.to_lowercase().contains("system.out.println") {
            let slergimg = "https://cdn.discordapp.com/attachments/1105467484372475978/1219268951671050311/ohgott.png?ex=660aafb3&is=65f83ab3&hm=8ff2d8c3beafbb8e77d265bbe53b8a4d4e6224c5c4caa8a0f65356762be7c8ac&";
            let _ = msg
                .author
                .create_dm_channel(&ctx)
                .await?
                .say(&ctx, slergimg)
                .await;
        }

        // jshell
        if msg.content.starts_with("jshell> ") {
            let stmt = msg
                .content
                .split_at(8)
                .1
                .replace("„", "\"")
                .replace("“", "\"");

            let mut jshell = self.jshell.lock().await;

            jshell.input(&format!("{stmt}\n")).await?;

            let output = timeout(Duration::from_secs(5), jshell.read_output()).await?;
            let output = output?;

            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Die JSHELL hat gesprochen: ```{output}```"),
                )
                .await?;
        }

        // 3. Block
        if rand::thread_rng().gen_range(1..=25) == 1 {
            let message = CreateMessage::new()
                .content("# Sie kommen bitte in den 3. Block")
                .reference_message(msg);
            msg.channel_id.send_message(&ctx.http, message).await?;

            // disconnect member from voice when in guild
            if let Some(id) = msg.guild_id {
                let _ = id.disconnect_member(&ctx, msg.author.id).await;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if let Err(err) = self.message_handler(&ctx, &msg).await {
            println!("error during message handler: {err:?}");

            // do not error out on error logging
            let _ = msg
                .channel_id
                .say(
                    &ctx.http,
                    format!("Der Slerg ist im Saal ☹️ : ```{}```", err.to_string()),
                )
                .await;

            // revive jshell if it died, closed or times out
            let needs_revive = if let AppError::JShellError(JShellError::IOError(io_err)) = err {
                io_err.kind() == ErrorKind::BrokenPipe
            } else if let AppError::JShellError(JShellError::ClosedError) = err {
                true
            } else if let AppError::TimeoutError(_) = err {
                true
            } else {
                false
            };

            if needs_revive {
                let revive_status = match self.revive_jshell().await {
                    Ok(()) => "# JShell wurde wiederbelebt".to_string(),
                    Err(err) => {
                        println!("failed to revive jshell: {err:?}");
                        format!("# JShell konnte nicht wiederbelebt werden:\n{err}")
                    }
                };

                let _ = msg.channel_id.say(&ctx.http, revive_status).await;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // Load environment and token from .env file if present
    let _ = dotenv();
    let token = env::var("TOKEN").expect("No token found in environment");

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot.
    let jshell = JShell::new().await.expect("failed to spawn jshell");

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler {
            jshell: Mutex::new(jshell),
        })
        .await
        .expect("Err creating client");

    // Start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
