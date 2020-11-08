use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use futures::lock::Mutex;
use log::error;
use serenity::client::{Client, EventHandler};
use structopt::{clap::ArgGroup, StructOpt};

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
            error!("error: {}", &e);
            let _ = ctx.message(Message::HandleError(e)).await;
        }
    }
}

#[derive(StructOpt)]
#[structopt(group = ArgGroup::with_name("tokens").required(true).multiple(false))]
/// You may want to set "KAISANDAIJIN_LOG" "KAISANDAIJIN_LOG_STYLE" to configure logger.
struct Opt {
    #[structopt(
        long,
        env = "KAISANDAIJIN_DISCORD_TOKEN",
        hide_env_values = true,
        group = "tokens"
    )]
    token: Option<String>,
    #[structopt(
        long,
        env = "KAISANDAIJIN_DISCORD_TOKEN_FILE",
        parse(from_os_str),
        group = "tokens"
    )]
    token_file: Option<PathBuf>,
    #[structopt(short, long, env = "KAISANDAIJIN_REDIS_URI")]
    redis_uri: String,
    #[structopt(
        short = "p",
        long,
        default_value = "kaisandaijin",
        env = "KAISANDAIJIN_REDIS_PREFIX"
    )]
    redis_prefix: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::from_args();

    let redis_client = redis::Client::open(opt.redis_uri)?;
    let redis_conn = Arc::new(Mutex::new(redis_client.get_async_connection().await?));

    let token = if let Some(token) = opt.token {
        token
    } else {
        let mut file = File::open(opt.token_file.unwrap())?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        content
    };
    let token = token.trim();

    let env = env_logger::Env::new()
        .filter("KAISANDAIJIN_LOG")
        .write_style("KAISANDAIJIN_LOG_STYLE");
    env_logger::try_init_from_env(env)?;

    let mut client = Client::builder(token)
        .event_handler(Handler {
            redis_prefix: opt.redis_prefix,
            redis: redis_conn,
        })
        .await
        .context("Failed to create client")?;

    client.start().await.context("Client error")
}
