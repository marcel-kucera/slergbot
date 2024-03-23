use std::env;
use std::process::Stdio;

use dotenvy::dotenv;
use rand::Rng;
use serenity::all::CreateMessage;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

struct Handler;

#[derive(Error, Debug)]
enum AppError {
    #[error("Discord Error")]
    DiscordError(#[from] SerenityError),

    #[error("IO Error")]
    IOError(#[from] std::io::Error),

    #[error("Child IO Error")]
    ChildIOError,
}

impl Handler {
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
            let mut cmd = Command::new("jshell")
                .arg("-")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let mut stdin = cmd.stdin.take().ok_or(AppError::ChildIOError)?;
            stdin.write(stmt.as_bytes()).await?;
            stdin.shutdown().await?;
            drop(stdin); // Send EOF to jshell and stop execution

            let mut output = String::new();
            let mut stdout = cmd.stdout.take().ok_or(AppError::ChildIOError)?;
            stdout.read_to_string(&mut output).await?;

            let mut stderr = cmd.stderr.take().ok_or(AppError::ChildIOError)?;
            stderr.read_to_string(&mut output).await?;

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
            println!("{:?}", err.to_string());

            // do not error out on error logging
            let _ = msg
                .channel_id
                .say(
                    &ctx.http,
                    format!("Der Slerg ist im Saal ☹️ : {}", err.to_string()),
                )
                .await;
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
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    // Start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
