use std::path::PathBuf;

use anyhow::{Context as _, Result};
use clap::Parser;
use serenity::{
    client::{Client, EventHandler},
    model::gateway::GatewayIntents,
};

use kaisantantoudaijin::{
    context::{ChannelContext, ContextBuilder},
    database::{AnyDatabaseHandle, DynamoDbHandle, RedisHandle},
    model::message::Message,
};

fn strip_affix<'a>(content: &'a str, affix: &str) -> Option<&'a str> {
    content
        .strip_prefix(affix)
        .or_else(|| content.strip_suffix(affix))
}

#[derive(Debug, Clone)]
enum DatabaseClient {
    Redis {
        prefix: String,
        pool: deadpool_redis::Pool,
    },
    DynamoDb {
        table_name: String,
        client: aws_sdk_dynamodb::Client,
    },
}

impl DatabaseClient {
    pub async fn obtain_handle(
        &self,
        guild_id: serenity::model::id::GuildId,
    ) -> Result<AnyDatabaseHandle> {
        match self {
            DatabaseClient::Redis { prefix, pool } => {
                let conn = pool.get().await?;
                let db = RedisHandle::new(prefix.clone(), guild_id, conn);
                Ok(db.into())
            }
            DatabaseClient::DynamoDb { table_name, client } => {
                let db = DynamoDbHandle::new(client.clone(), guild_id, table_name.clone());
                Ok(db.into())
            }
        }
    }
}

struct Handler {
    command_prefix: String,
    database_client: DatabaseClient,
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

        let db = match self.database_client.obtain_handle(guild_id).await {
            Ok(x) => x,
            Err(e) => {
                tracing::error!("error in getting DB connection: {:#}", e);
                let _ = msg.channel_id.say(&ctx.http, "エラーが発生しました").await;
                return;
            }
        };

        let ctx = ContextBuilder::with_serenity(&ctx)
            .db(db)
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
#[command(group(clap::ArgGroup::new("database_config").required(true).multiple(false).args(["redis_uri", "dynamodb_table_name"])))]
struct Args {
    #[arg(long, default_value = "!kaisan", env = "KAISANDAIJIN_COMMAND_PREFIX")]
    command_prefix: String,
    #[arg(long, env = "KAISANDAIJIN_DISCORD_TOKEN", hide_env_values = true)]
    token: Option<String>,
    #[arg(long, env = "KAISANDAIJIN_DISCORD_TOKEN_FILE")]
    token_file: Option<PathBuf>,
    #[arg(short, long, env = "KAISANDAIJIN_REDIS_URI")]
    redis_uri: Option<String>,
    #[arg(
        short = 'p',
        long,
        default_value = "kaisandaijin",
        env = "KAISANDAIJIN_REDIS_PREFIX"
    )]
    redis_prefix: String,
    #[arg(short, long, env = "KAISANDAIJIN_DYNAMODB_TABLE_NAME")]
    dynamodb_table_name: Option<String>,
    /// Specify log level filter, configured in conjunction with KAISANDAIJIN_LOG environment variable
    #[arg(short, long)]
    log_level: Option<tracing_subscriber::filter::LevelFilter>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

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

    let database_client = match (&args.redis_uri, &args.dynamodb_table_name) {
        (Some(redis_uri), None) => DatabaseClient::Redis {
            prefix: args.redis_prefix.clone(),
            pool: deadpool_redis::Config::from_url(redis_uri)
                .create_pool(Some(deadpool_redis::Runtime::Tokio1))?,
        },
        (None, Some(table_name)) => DatabaseClient::DynamoDb {
            table_name: table_name.clone(),
            client: {
                let config = aws_config::load_from_env().await;
                aws_sdk_dynamodb::Client::new(&config)
            },
        },
        _ => anyhow::bail!("either Redis URI or DynamoDB table name must be specified"),
    };

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
            database_client,
        })
        .await
        .context("Failed to create client")?;

    client.start().await.context("Client error")
}
