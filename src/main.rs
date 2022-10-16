use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use clap::Parser;
use futures::lock::Mutex;
use serenity::{
    client::{Client, EventHandler},
    model::gateway::GatewayIntents,
};

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

    async fn cache_ready(
        &self,
        _ctx: serenity::client::Context,
        guild_ids: Vec<serenity::model::id::GuildId>,
    ) {
        tracing::info!(?guild_ids, "cache is ready");
    }
}

#[derive(Parser)]
#[command(group(clap::ArgGroup::new("tokens").required(true).multiple(false).args(["token", "token_file"])))]
struct Args {
    #[arg(long, env = "KAISANDAIJIN_DISCORD_TOKEN", hide_env_values = true)]
    token: Option<String>,
    #[arg(long, env = "KAISANDAIJIN_DISCORD_TOKEN_FILE")]
    token_file: Option<PathBuf>,
    #[arg(short, long, env = "KAISANDAIJIN_REDIS_URI")]
    redis_uri: String,
    #[arg(
        short = 'p',
        long,
        default_value = "kaisandaijin",
        env = "KAISANDAIJIN_REDIS_PREFIX"
    )]
    redis_prefix: String,
    /// Specify log level filter, configured in conjunction with KAISANDAIJIN_LOG environment variable
    #[arg(short, long)]
    log_level: Option<tracing_subscriber::filter::LevelFilter>,
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

    let env_filter = tracing_subscriber::EnvFilter::from_env("KAISANDAIJIN_LOG");
    let env_filter = args
        .log_level
        .into_iter()
        .fold(env_filter, |filter, level| {
            filter.add_directive(level.into())
        });
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();

    let intents = [
        GatewayIntents::GUILDS,
        GatewayIntents::GUILD_MESSAGES,
        GatewayIntents::GUILD_VOICE_STATES,
        GatewayIntents::MESSAGE_CONTENT,
    ]
    .into_iter()
    .collect();
    let mut client = Client::builder(token, intents)
        .event_handler(Handler {
            redis_prefix: args.redis_prefix,
            redis: redis_conn,
        })
        .await
        .context("Failed to create client")?;

    client.start().await.context("Client error")
}
