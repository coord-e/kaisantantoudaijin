use std::path::PathBuf;

use anyhow::{Context as _, Result};
use clap::Parser;
use serenity::{
    client::{Client, EventHandler},
    model::gateway::GatewayIntents,
};

use kaisantantoudaijin::{
    context::{ChannelContext, ContextBuilder},
    model::message::Message,
};

fn strip_affix<'a>(content: &'a str, affix: &str) -> Option<&'a str> {
    content
        .strip_prefix(affix)
        .or_else(|| content.strip_suffix(affix))
}

struct Handler {
    command_prefix: String,
    redis_prefix: String,
    redis: deadpool_redis::Pool,
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

        let bot_id = ctx.cache.current_user().id;
        let command = strip_affix(&msg.content, &format!("<@{}>", bot_id))
            .or_else(|| strip_affix(&msg.content, &format!("<@!{}>", bot_id)))
            .or_else(|| msg.content.strip_prefix(&self.command_prefix))
            .map(str::trim);

        let Some(command) = command else {
            return;
        };

        let Some(guild_id) = msg.guild_id else {
            let _ = msg
                .channel_id
                .say(&ctx.http, "サーバー内で使ってください")
                .await;
            return;
        };

        let redis_conn = match self.redis.get().await {
            Ok(x) => x,
            Err(e) => {
                tracing::error!("error in getting redis connection: {:#}", e);
                let _ = msg.channel_id.say(&ctx.http, "エラーが発生しました").await;
                return;
            }
        };

        let ctx = ContextBuilder::with_serenity(&ctx)
            .redis_prefix(self.redis_prefix.clone())
            .redis_conn(redis_conn)
            .guild_id(guild_id)
            .message(&msg)
            .build()
            .unwrap();

        if let Err(e) = ctx.handle_command(command).await {
            tracing::error!("error in handling command: {:#}", e);
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
    #[arg(long, default_value = "!kaisan", env = "KAISANDAIJIN_COMMAND_PREFIX")]
    command_prefix: String,
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

    let redis = deadpool_redis::Config::from_url(args.redis_uri)
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))?;

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
            command_prefix: args.command_prefix,
            redis_prefix: args.redis_prefix,
            redis,
        })
        .await
        .context("Failed to create client")?;

    client.start().await.context("Client error")
}
