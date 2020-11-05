use std::env;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use anyhow::{bail, Context as _, Result};
use chrono::{FixedOffset, Utc};
use log::{error, info};
use serenity::{
    async_trait,
    builder::EditMember,
    model::{
        channel::Message,
        gateway::Ready,
        guild::Guild,
        id::{GuildId, UserId},
        voice::VoiceState,
    },
    prelude::*,
};
use tokio::time;

async fn handle(ctx: &Context, msg: &Message) -> Result<()> {
    let channel_id = msg.channel_id;
    let guild_id = match msg.guild_id {
        None => bail!("not in guild"),
        Some(id) => id,
    };
    let guild = match ctx.cache.guild(guild_id).await {
        None => bail!("inaccessible guild"),
        Some(guild) => guild,
    };

    if let Some(content) = msg.content.strip_prefix("!kaisan ") {
        let args: Vec<_> = content.splitn(3, ' ').collect();
        let target_users = match args.first().copied() {
            Some("me") => vec![msg.author.id],
            Some("all") => same_channel_users(&guild, msg.author.id)?,
            _ => {
                channel_id.say(&ctx.http, "わからん: me か all").await?;
                return Ok(());
            }
        };

        let duration = match args.as_slice() {
            &[_, "at", time] => {
                let offset = Duration::from_secs(9 * 3600);
                let time = humantime::parse_rfc3339_weak(time)? - offset;
                let duration = time.duration_since(SystemTime::now())?;

                channel_id
                    .say(
                        &ctx.http,
                        format!("はい ({} 後)", humantime::format_duration(duration)),
                    )
                    .await?;

                duration
            }
            &[_, "after", duration] => {
                let duration = humantime::parse_duration(duration)?;

                let jst = FixedOffset::east(9 * 3600);
                let time = Utc::now() + chrono::Duration::from_std(duration)?;
                channel_id
                    .say(&ctx.http, format!("はい ({})", time.with_timezone(&jst)))
                    .await?;

                duration
            }
            _ => {
                channel_id.say(&ctx.http, "わからん: at か after").await?;
                return Ok(());
            }
        };

        schedule_after(ctx, guild, target_users, duration);
    }

    Ok(())
}

fn schedule_after(
    ctx: &Context,
    guild: impl Into<GuildId>,
    users: impl IntoIterator<Item = impl Into<UserId>>,
    duration: Duration,
) {
    let users: Vec<_> = users.into_iter().map(Into::into).collect();
    let guild_id = guild.into();
    let http = Arc::clone(&ctx.http);

    tokio::spawn(async move {
        time::delay_for(duration).await;

        for user_id in users {
            info!("disconnect {:?}", user_id);
            guild_id
                .edit_member(&http, user_id, EditMember::disconnect_member)
                .await
                .expect("wtf");
        }
    });
}

fn same_channel_users(guild: &Guild, user_id: UserId) -> Result<Vec<UserId>> {
    let channel_id = match guild.voice_states.get(&user_id) {
        Some(VoiceState {
            channel_id: Some(id),
            ..
        }) => id,
        _ => bail!("not in voice channel"),
    };

    let mut target_users = Vec::new();
    for (user_id, state) in &guild.voice_states {
        if state.channel_id == Some(*channel_id) {
            target_users.push(user_id.clone());
        }
    }

    Ok(target_users)
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        if let Err(e) = handle(&ctx, &msg).await {
            let _ = msg.channel_id.say(&ctx.http, "ダメそう").await;
            error!("Error in handler: {}", e);
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
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
