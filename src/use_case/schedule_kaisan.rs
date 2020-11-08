use crate::context::{
    ChannelContext, ConfigContext, GuildContext, MessageContext, RandomContext, TimeContext,
};
use crate::error::{Error, Result};
use crate::model::{
    command::TimeRangeSpecifier, kaisanee::KaisaneeSpecifier, message::Message,
    time::calculate_time,
};

use chrono::{DateTime, Duration, Utc};
use log::{error, info};
use serenity::model::id::{ChannelId, UserId};
use tokio::{spawn, time};

#[async_trait::async_trait]
pub trait ScheduleKaisan:
    GuildContext
    + ChannelContext
    + MessageContext
    + ConfigContext
    + TimeContext
    + RandomContext
    + Clone
    + Send
    + 'static
{
    async fn schedule_kaisan(
        &self,
        kaisanee: KaisaneeSpecifier,
        time_range: TimeRangeSpecifier,
    ) -> Result<()> {
        let author_id = self.author_id();
        let voice_channel_id = match self.connected_voice_channel(author_id).await? {
            Some(id) => id,
            None => return Err(Error::NotInVoiceChannel),
        };

        let now = self.current_time();
        let tz = self.timezone();
        let time = match time_range {
            TimeRangeSpecifier::At(spec) => {
                let time = calculate_time(spec, now, tz);
                if time < now {
                    return Err(Error::UnreachableTime {
                        specified: time,
                        at: now,
                    });
                }

                self.message(Message::ScheduledAt(
                    kaisanee.clone(),
                    time.with_timezone(&tz),
                ))
                .await?;
                time
            }
            TimeRangeSpecifier::By(spec) => {
                let by = calculate_time(spec, now, tz);
                if by < now {
                    return Err(Error::UnreachableTime {
                        specified: by,
                        at: now,
                    });
                }

                let duration = by - now;
                let random_secs = self.random_range(0, duration.num_seconds()).await;
                let random_duration = Duration::seconds(random_secs);
                let time = now + random_duration;

                self.message(Message::ScheduledBy(
                    kaisanee.clone(),
                    by.with_timezone(&tz),
                ))
                .await?;
                time
            }
        };

        let ctx = self.clone();
        schedule_kaisan_at(ctx, author_id, voice_channel_id, time, kaisanee.clone());

        info!("scheduled kaisan for {:?} at {}", kaisanee, time);
        Ok(())
    }
}

impl<
        T: GuildContext
            + ChannelContext
            + MessageContext
            + ConfigContext
            + TimeContext
            + RandomContext
            + Clone
            + Send
            + 'static,
    > ScheduleKaisan for T
{
}

fn schedule_kaisan_at<C: ScheduleKaisan + Send + Sync>(
    ctx: C,
    author_id: UserId,
    voice_channel_id: ChannelId,
    time: DateTime<Utc>,
    kaisanee: KaisaneeSpecifier,
) {
    spawn(async move {
        let now = ctx.current_time();
        if let Ok(duration) = (time - now).to_std() {
            time::delay_for(duration).await;
        }

        if let Err(e) = kaisan(&ctx, author_id, voice_channel_id, kaisanee).await {
            error!("failed to kaisan: {}", &e);
            let _ = ctx.react('❌').await;
            let _ = ctx.message(Message::KaisanError(e)).await;
        }
    });
}

async fn kaisan<C: ScheduleKaisan>(
    ctx: &C,
    author_id: UserId,
    voice_channel_id: ChannelId,
    kaisanee: KaisaneeSpecifier,
) -> Result<()> {
    let in_users = ctx.voice_channel_users(voice_channel_id).await?;

    let target_users = match kaisanee {
        KaisaneeSpecifier::Me => {
            if in_users.contains(&author_id) {
                vec![author_id]
            } else {
                vec![]
            }
        }
        KaisaneeSpecifier::All => in_users,
        KaisaneeSpecifier::Users(users) => {
            users.into_iter().filter(|u| in_users.contains(u)).collect()
        }
    };

    for user_id in &target_users {
        info!("disconnect {:?}", user_id);
        ctx.disconnect_user(*user_id).await?;
    }

    ctx.react('✅').await?;
    if !target_users.is_empty() {
        ctx.message(Message::Kaisan(target_users)).await?;
    }

    Ok(())
}
