use std::collections::VecDeque;
use std::env;
use std::time::{Duration, SystemTime};

use anyhow::{bail, Result};
use chrono::{FixedOffset, Utc};
use serenity::{
    async_trait,
    builder::EditMember,
    model::{channel::Message, gateway::Ready, guild::Guild, id::UserId, voice::VoiceState},
    prelude::*,
};
use tokio::time;

struct Handler;

impl Handler {
    fn schedule_after(
        &self,
        ctx: Context,
        guild: Guild,
        users: impl IntoIterator<Item = impl Into<UserId>>,
        duration: Duration,
    ) {
        let now = time::Instant::now();
        let time = now + duration;
        let users: Vec<_> = users.into_iter().map(Into::into).collect();

        tokio::spawn(async move {
            time::delay_until(time).await;

            for user_id in users {
                println!("disconnect {:?}", user_id);
                guild
                    .edit_member(&ctx.http, user_id, EditMember::disconnect_member)
                    .await
                    .expect("wtf");
            }
        });
    }

    fn target_users(&self, guild: &Guild, user_id: UserId) -> Result<Vec<UserId>> {
        let channel_id = match guild.voice_states.get(&user_id) {
            None
            | Some(VoiceState {
                channel_id: None, ..
            }) => bail!("wtf"),
            Some(VoiceState {
                channel_id: Some(id),
                ..
            }) => id,
        };

        let mut target_users = Vec::new();
        for (user_id, state) in &guild.voice_states {
            if state.channel_id == Some(*channel_id) {
                target_users.push(user_id.clone());
            }
        }

        Ok(target_users)
    }

    async fn handle(&self, ctx: Context, msg: Message) -> Result<()> {
        let guild_id = match msg.guild_id {
            None => bail!("non-guild context"),
            Some(id) => id,
        };
        let guild = match ctx.cache.guild(guild_id).await {
            None => bail!("non-guild context"),
            Some(guild) => guild,
        };

        if let Some(content) = msg.content.strip_prefix("!kaisan") {
            let mut args: VecDeque<_> = content.trim().splitn(3, ' ').collect();
            let target_users = match args.pop_front() {
                Some("me") => vec![msg.author.id],
                Some("all") => self.target_users(&guild, msg.author.id)?,
                _ => bail!("wtf"),
            };

            match (args.get(0).copied(), args.get(1)) {
                (Some("at"), Some(time)) => {
                    let offset = Duration::from_secs(9 * 3600);
                    let time = humantime::parse_rfc3339_weak(time)? - offset;
                    let now = SystemTime::now();

                    let jst = FixedOffset::east(9 * 3600);
                    let display_time = chrono::DateTime::<Utc>::from(time).with_timezone(&jst);
                    println!("register kaisamne {:?}", display_time);
                    msg.channel_id
                        .say(&ctx.http, format!("はい: {:?}", display_time))
                        .await?;

                    self.schedule_after(ctx, guild, target_users, time.duration_since(now)?);
                }
                (Some("after"), Some(duration)) => {
                    let duration = humantime::parse_duration(duration)?;

                    let jst = FixedOffset::east(9 * 3600);
                    let display_time =
                        (Utc::now() + chrono::Duration::from_std(duration)?).with_timezone(&jst);
                    println!("register kaisamne {:?}", display_time);
                    msg.channel_id
                        .say(&ctx.http, format!("はい: {:?}", display_time))
                        .await?;

                    self.schedule_after(ctx, guild, target_users, duration);
                }
                _ => {
                    msg.channel_id.say(&ctx.http, "わからん").await?;
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if let Err(e) = self.handle(ctx, msg).await {
            println!("Error sending message: {}", e);
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
