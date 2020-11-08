use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use chrono_tz::Tz;
use log::error;
use serenity::client::{Client, EventHandler};
use structopt::{clap::ArgGroup, StructOpt};

use kaisantantoudaijin::{
    context::{ChannelContext, Config, Context},
    model::message::Message,
};

struct Handler {
    config: Config,
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
            self.config.clone(),
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
    #[structopt(short, long, env = "KAISANDAIJIN_TIMEZONE")]
    timezone: Tz,
    #[structopt(short, long)]
    requires_permission: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::from_args();

    let config = Config {
        timezone: opt.timezone,
        requires_permission: opt.requires_permission,
    };

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
        .event_handler(Handler { config })
        .await
        .context("Failed to create client")?;

    client.start().await.context("Client error")
}
