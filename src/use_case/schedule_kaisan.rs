use crate::context::{
    ChannelContext, GuildContext, MessageContext, RandomContext, SettingContext, TimeContext,
};
use crate::error::{Error, Result};
use crate::model::{
    command::TimeRangeSpecifier, kaisanee::KaisaneeSpecifier, message::Message, reminder::Reminder,
};

use chrono::{DateTime, Duration, Utc};
use futures::future;
use log::{error, info};
use serenity::model::{
    id::{ChannelId, UserId},
    permissions::Permissions,
};
use tokio::spawn;

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
            TimeRangeSpecifier::Now => {
                return kaisan(self, voice_channel_id, &kaisanee).await;
            }
            TimeRangeSpecifier::At(spec) => {
                let time = spec.calculate_time(now, tz);
                if time < now {
                    return Err(Error::UnreachableTime {
                        specified: time,
                        at: now,
                    });
                }

                self.message(Message::Scheduled {
                    spec: time_range,
                    kaisanee: kaisanee.clone(),
                    time: time.with_timezone(&tz),
                    now: now.with_timezone(&tz),
                })
                .await?;
                time
            }
            TimeRangeSpecifier::By(spec) => {
                let by = spec.calculate_time(now, tz);
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

                self.message(Message::Scheduled {
                    spec: time_range,
                    kaisanee: kaisanee.clone(),
                    time: by.with_timezone(&tz),
                    now: now.with_timezone(&tz),
                })
                .await?;
                time
            }
        };

        let ctx = self.clone();
        schedule_kaisan_at(ctx.clone(), voice_channel_id, time, kaisanee.clone());
        info!("scheduled kaisan for {:?} at {}", kaisanee, time);

        let reminders = self.reminders().await?;
        for reminder in reminders {
            let remind_time = time - reminder.before_duration();
            if remind_time <= now {
                continue;
            }

            schedule_reminder_at(
                self.clone(),
                voice_channel_id,
                remind_time,
                kaisanee.clone(),
                reminder,
            );
            info!("scheduled remind for {:?} at {}", kaisanee, remind_time);
        }

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
    voice_channel_id: ChannelId,
    time: DateTime<Utc>,
    kaisanee: KaisaneeSpecifier,
) {
    spawn(async move {
        ctx.delay_until(time).await;

        if let Err(e) = kaisan(&ctx, voice_channel_id, &kaisanee).await {
            error!("failed to kaisan: {}", &e);
            let _ = future::try_join(ctx.react('❌'), ctx.message(Message::KaisanError(e))).await;
        }
    });
}

fn schedule_reminder_at<C: ScheduleKaisan + Sync>(
    ctx: C,
    voice_channel_id: ChannelId,
    remind_time: DateTime<Utc>,
    kaisanee: KaisaneeSpecifier,
    reminder: Reminder,
) {
    spawn(async move {
        ctx.delay_until(remind_time).await;

        if let Err(e) = remind(&ctx, voice_channel_id, &kaisanee, reminder).await {
            error!("failed to remind: {}", &e);
            let _ = future::try_join(ctx.react('❌'), ctx.message(Message::RemindError(e))).await;
        }
    });
}

async fn kaisan<C: ScheduleKaisan + Sync>(
    ctx: &C,
    voice_channel_id: ChannelId,
    kaisanee: &KaisaneeSpecifier,
) -> Result<()> {
    let target_users = collect_target_users(ctx, voice_channel_id, kaisanee).await?;

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

async fn remind<C: ScheduleKaisan + Sync>(
    ctx: &C,
    voice_channel_id: ChannelId,
    kaisanee: &KaisaneeSpecifier,
    reminder: Reminder,
) -> Result<()> {
    let target_users = collect_target_users(ctx, voice_channel_id, kaisanee).await?;

    if !target_users.is_empty() {
        ctx.message(Message::Remind(target_users, reminder)).await?;
    }

    Ok(())
}

async fn collect_target_users<C: ScheduleKaisan + Sync>(
    ctx: &C,
    voice_channel_id: ChannelId,
    kaisanee: &KaisaneeSpecifier,
) -> Result<Vec<UserId>> {
    let in_users = ctx.voice_channel_users(voice_channel_id).await?;
    let author_id = ctx.author_id();

    Ok(match kaisanee {
        KaisaneeSpecifier::Me => {
            if in_users.contains(&author_id) {
                vec![author_id]
            } else {
                vec![]
            }
        }
        KaisaneeSpecifier::All => in_users,
        KaisaneeSpecifier::Users(users) => users
            .iter()
            .filter(|u| in_users.contains(u))
            .copied()
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::ScheduleKaisan;
    use crate::{
        error::Error,
        model::{
            command::TimeRangeSpecifier,
            kaisanee::KaisaneeSpecifier,
            message::Message,
            reminder::Reminder,
            time::{AfterTimeSpecifier, TimeSpecifier},
        },
        test::{MockContext, MOCK_AUTHOR_1, MOCK_AUTHOR_2},
        use_case,
    };
    use chrono::{Duration, FixedOffset, Utc};
    use std::sync::atomic::Ordering;

    #[tokio::test]
    async fn test_all() {
        let ctx = MockContext::with_author(MOCK_AUTHOR_2);

        ctx.schedule_kaisan(KaisaneeSpecifier::All, TimeRangeSpecifier::Now)
            .await
            .unwrap();

        ctx.set_current_time(Utc::now() + Duration::seconds(1));
        wait_a_little(ctx.wait_for_message(|m| matches!(m, Message::Kaisan(_)))).await;

        {
            let users = &*ctx.disconnected_users.lock().await;
            assert!(users.contains(&MOCK_AUTHOR_1));
            assert!(users.contains(&MOCK_AUTHOR_2));
        }
    }

    #[tokio::test]
    async fn test_me() {
        let time = Utc::now();
        let ctx = MockContext::with_author_current_time(MOCK_AUTHOR_2, time);

        ctx.schedule_kaisan(
            KaisaneeSpecifier::Me,
            TimeRangeSpecifier::At(TimeSpecifier::Exactly(
                time.with_timezone(&FixedOffset::east(0)) + Duration::minutes(10),
            )),
        )
        .await
        .unwrap();

        ctx.set_current_time(time + Duration::minutes(10));
        wait_a_little(ctx.wait_for_message(|m| matches!(m, Message::Kaisan(_)))).await;

        {
            let users = &*ctx.disconnected_users.lock().await;
            assert!(!users.contains(&MOCK_AUTHOR_1));
            assert!(users.contains(&MOCK_AUTHOR_2));
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
    async fn test_reminders() {
        let time = Utc::now();
        let ctx = MockContext::with_author_current_time(MOCK_AUTHOR_2, time);

        let reminder1 = Reminder::before_minutes(3);
        use_case::AddReminder::add_reminder(&ctx, reminder1)
            .await
            .unwrap();
        let reminder2 = Reminder::before_minutes(1);
        use_case::AddReminder::add_reminder(&ctx, reminder2)
            .await
            .unwrap();

        ctx.schedule_kaisan(
            KaisaneeSpecifier::All,
            TimeRangeSpecifier::At(TimeSpecifier::After(AfterTimeSpecifier::Minute(5))),
        )
        .await
        .unwrap();

        ctx.set_current_time(time + Duration::minutes(2));
        wait_a_little(
            ctx.wait_for_message(|m| matches!(m, Message::Remind(_, r) if r == &reminder1)),
        )
        .await;

        ctx.set_current_time(time + Duration::minutes(4));
        wait_a_little(
            ctx.wait_for_message(|m| matches!(m, Message::Remind(_, r) if r == &reminder2)),
        )
        .await;

        ctx.set_current_time(time + Duration::minutes(5));
        wait_a_little(ctx.wait_for_message(|m| matches!(m, Message::Kaisan(_)))).await;

        {
            let users = &*ctx.disconnected_users.lock().await;
            assert!(users.contains(&MOCK_AUTHOR_1));
            assert!(users.contains(&MOCK_AUTHOR_2));
        }
    }

    #[tokio::test]
    async fn test_insufficient_permission() {
        let ctx = MockContext::with_author(MOCK_AUTHOR_1);
        ctx.requires_permission.store(true, Ordering::SeqCst);

        let res = ctx
            .schedule_kaisan(KaisaneeSpecifier::All, TimeRangeSpecifier::Now)
            .await;
        assert!(matches!(res, Err(Error::InsufficientPermission(_))));
    }

    #[tokio::test]
    async fn test_sufficient_permission() {
        let ctx = MockContext::with_author(MOCK_AUTHOR_1);
        ctx.requires_permission.store(false, Ordering::SeqCst);

        let res = ctx
            .schedule_kaisan(KaisaneeSpecifier::All, TimeRangeSpecifier::Now)
            .await;
        assert!(matches!(res, Ok(())));
    }

    async fn wait_a_little<F: std::future::Future>(future: F) {
        tokio::time::timeout(std::time::Duration::from_millis(100), future)
            .await
            .unwrap();
    }
}
