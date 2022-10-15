use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use clap::Parser;
use futures::lock::Mutex;
use serenity::client::{Client, EventHandler};

use kaisantantoudaijin::{
    context::{ChannelContext, Context},
    model::message::Message,
};

struct Handler {
    redis_prefix: String,
    redis: Arc<Mutex<redis::aio::Connection>>,
}

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

        let ctx = match Context::new(
            Arc::clone(&ctx.http),
            Arc::clone(&ctx.cache),
            self.redis_prefix.clone(),
            Arc::clone(&self.redis),
            &msg,
        )
        .await
        {
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
            tracing::error!(error = %e, "error in handling message");
            let _ = ctx.message(Message::HandleError(e)).await;
        }
    }
}

#[derive(Parser)]
#[clap(group = clap::ArgGroup::new("tokens").required(true).multiple(false))]
struct Args {
    #[clap(
        long,
        env = "KAISANDAIJIN_DISCORD_TOKEN",
        hide_env_values = true,
        group = "tokens"
    )]
    token: Option<String>,
    #[clap(
        long,
        env = "KAISANDAIJIN_DISCORD_TOKEN_FILE",
        parse(from_os_str),
        group = "tokens"
    )]
    token_file: Option<PathBuf>,
    #[clap(short, long, env = "KAISANDAIJIN_REDIS_URI")]
    redis_uri: String,
    #[clap(
        short = 'p',
        long,
        default_value = "kaisandaijin",
        env = "KAISANDAIJIN_REDIS_PREFIX"
    )]
    redis_prefix: String,
    #[clap(short, long, env = "KAISANDAIJIN_LOG", default_value = "warn")]
    log_filter: tracing_subscriber::filter::EnvFilter,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let redis_client = redis::Client::open(args.redis_uri)?;
    let redis_conn = Arc::new(Mutex::new(redis_client.get_async_connection().await?));

    let token = if let Some(token) = args.token {
        token
    } else {
        tokio::fs::read_to_string(args.token_file.unwrap()).await?
    };
    let token = token.trim();

    tracing_subscriber::fmt()
        .with_env_filter(args.log_filter)
        .with_writer(std::io::stderr)
        .init();

    let mut client = Client::builder(token)
        .event_handler(Handler {
            redis_prefix: args.redis_prefix,
            redis: redis_conn,
        })
        .await
        .context("Failed to create client")?;

    client.start().await.context("Client error")
}
