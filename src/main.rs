use std::env;

use anyhow::{Context as _, Result};
use log::error;
use serenity::client::{Client, EventHandler};

use kaisantantoudaijin::{
    context::{ChannelContext, Context},
    model::message::Message,
};

struct Handler;

#[async_trait::async_trait]
impl EventHandler for Handler {
    async fn message(
        &self,
        ctx: serenity::client::Context,
        msg: serenity::model::channel::Message,
    ) {
        if msg.author.bot {
            return;
        }

        let ctx = match Context::from_message(&ctx, &msg).await {
            Some(x) => x,
            None => {
                let _ = msg
                    .channel_id
                    .say(&ctx.http, "サーバー内で使ってください")
                    .await;
                return;
            }
        };

        if let Err(e) = ctx.handle_message(msg).await {
            error!("error: {}", &e);
            let _ = ctx.message(Message::HandleError(e)).await;
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::try_init()?;

    let token = env::var("DISCORD_TOKEN").context("DISCORD_TOKEN is not set")?;

    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .await
        .context("Failed to create client")?;

    client.start().await.context("Client error")
}
