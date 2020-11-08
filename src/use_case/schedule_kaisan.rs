use crate::context::{
    ChannelContext, GuildContext, MessageContext, RandomContext, SettingContext, TimeContext,
};
use crate::error::{Error, Result};
use crate::model::{
    command::TimeRangeSpecifier, kaisanee::KaisaneeSpecifier, message::Message,
    time::calculate_time,
};

use chrono::{DateTime, Duration, Utc};
use futures::future;
use log::{error, info};
use serenity::model::{
    id::{ChannelId, UserId},
    permissions::Permissions,
};
use tokio::{spawn, time};

#[async_trait::async_trait]
pub trait ScheduleKaisan:
    GuildContext
    + ChannelContext
    + MessageContext
    + SettingContext
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

        if kaisanee.may_include_others(author_id)
            && self.requires_permission().await?
            && !self.member_permissions(author_id).await?.move_members()
        {
            return Err(Error::InsufficientPermission(Permissions::MOVE_MEMBERS));
        }

        let voice_channel_id = match self.connected_voice_channel(author_id).await? {
            Some(id) => id,
            None => return Err(Error::NotInVoiceChannel),
        };

        let now = self.current_time();
        let tz = self.timezone().await?;
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
            + SettingContext
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
            let _ = future::try_join(ctx.react('❌'), ctx.message(Message::KaisanError(e))).await;
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

    let mut futures = Vec::new();
    for user_id in &target_users {
        info!("disconnect {:?}", user_id);
        futures.push(ctx.disconnect_user(*user_id));
    }

    if !target_users.is_empty() {
        futures.push(ctx.message(Message::Kaisan(target_users)));
    }

    future::try_join_all(futures).await?;

    ctx.react('✅').await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::ScheduleKaisan;
    use crate::{
        error::Error,
        model::{
            command::TimeRangeSpecifier, kaisanee::KaisaneeSpecifier, message::Message,
            time::TimeSpecifier,
        },
        test::{MockContext, MOCK_AUTHOR_1, MOCK_AUTHOR_2},
    };
    use chrono::{FixedOffset, Utc};
    use std::{sync::atomic::Ordering, time::Duration};

    #[tokio::test]
    async fn test_all() {
        let ctx = MockContext::with_author(MOCK_AUTHOR_2);

        ctx.schedule_kaisan(
            KaisaneeSpecifier::All,
            TimeRangeSpecifier::At(TimeSpecifier::Now),
        )
        .await
        .unwrap();

        // TODO: more reliable way to wait for change
        tokio::time::delay_for(Duration::from_millis(200)).await;

        {
            let users = &*ctx.disconnected_users.lock().await;
            assert!(users.contains(&MOCK_AUTHOR_1));
            assert!(users.contains(&MOCK_AUTHOR_2));
        }

        {
            let messages = &*ctx.sent_messages.lock().await;
            assert!(messages
                .iter()
                .find(|m| matches!(m, Message::Kaisan(_)))
                .is_some());
        }
    }

    #[tokio::test]
    async fn test_me() {
        let ctx = MockContext::with_author(MOCK_AUTHOR_2);

        ctx.schedule_kaisan(
            KaisaneeSpecifier::Me,
            TimeRangeSpecifier::At(TimeSpecifier::Now),
        )
        .await
        .unwrap();

        // TODO: more reliable way to wait for change
        tokio::time::delay_for(Duration::from_millis(200)).await;

        {
            let users = &*ctx.disconnected_users.lock().await;
            assert!(!users.contains(&MOCK_AUTHOR_1));
            assert!(users.contains(&MOCK_AUTHOR_2));
        }

        {
            let messages = &*ctx.sent_messages.lock().await;
            assert!(messages
                .iter()
                .find(|m| matches!(m, Message::Kaisan(_)))
                .is_some());
        }
    }

    #[tokio::test]
    async fn test_unreachable_time() {
        let now = Utc::now();
        let ctx = MockContext::with_current_time(now);

        let now_with_tz = now.with_timezone(&FixedOffset::east(3600));
        let res = ctx
            .schedule_kaisan(
                KaisaneeSpecifier::Me,
                TimeRangeSpecifier::At(TimeSpecifier::Exactly(
                    now_with_tz - chrono::Duration::minutes(1),
                )),
            )
            .await;

        assert!(matches!(res, Err(Error::UnreachableTime { .. })));
    }

    #[tokio::test]
    async fn test_insufficient_permission() {
        let ctx = MockContext::with_author(MOCK_AUTHOR_1);
        ctx.requires_permission.store(true, Ordering::SeqCst);

        let res = ctx
            .schedule_kaisan(
                KaisaneeSpecifier::All,
                TimeRangeSpecifier::At(TimeSpecifier::Now),
            )
            .await;
        assert!(matches!(res, Err(Error::InsufficientPermission(_))));
    }

    #[tokio::test]
    async fn test_sufficient_permission() {
        let ctx = MockContext::with_author(MOCK_AUTHOR_1);
        ctx.requires_permission.store(false, Ordering::SeqCst);

        let res = ctx
            .schedule_kaisan(
                KaisaneeSpecifier::All,
                TimeRangeSpecifier::At(TimeSpecifier::Now),
            )
            .await;
        assert!(matches!(res, Ok(())));
    }
}
