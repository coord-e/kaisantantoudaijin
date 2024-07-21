use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::model::{command::Command, reminder::Reminder};
use crate::say::SayExt;
use crate::use_case;

use anyhow::Context as _;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use futures::lock::Mutex;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use redis::{AsyncCommands, FromRedisValue, ToRedisArgs};
use serenity::{
    builder::EditMember,
    cache::Cache,
    http::Http,
    model::{
        channel::{Message, ReactionType},
        id::{ChannelId, GuildId, MessageId, UserId},
        permissions::Permissions,
        voice::VoiceState,
    },
};

mod bot;
mod channel;
mod guild;
mod message;
mod random;
mod setting;
mod time;

pub use bot::BotContext;
pub use channel::ChannelContext;
pub use guild::GuildContext;
pub use message::MessageContext;
pub use random::RandomContext;
pub use setting::SettingContext;
pub use time::TimeContext;

#[derive(Clone)]
pub struct Context {
    http: Arc<Http>,
    cache: Arc<Cache>,
    bot_id: UserId,
    guild_id: GuildId,
    author_id: UserId,
    channel_id: ChannelId,
    message_id: MessageId,
    redis_prefix: String,
    redis: Arc<Mutex<deadpool_redis::Connection>>,
    rng: Arc<Mutex<SmallRng>>,
}

impl Context {
    async fn voice_states(&self) -> Result<HashMap<UserId, VoiceState>> {
        Ok(self
            .cache
            .guild(self.guild_id)
            .ok_or(Error::InaccessibleGuild)?
            .voice_states
            .clone())
    }

    fn redis_key(&self, key: &str) -> String {
        format!("{}:{}:{}", self.redis_prefix, u64::from(self.guild_id), key)
    }

    async fn redis_get<T: FromRedisValue>(&self, key: &str) -> Result<Option<T>> {
        let r = self
            .redis
            .lock()
            .await
            .get(self.redis_key(key))
            .await
            .context("cannot read from redis")?;
        Ok(r)
    }

    async fn redis_set<T: ToRedisArgs + Send + Sync>(&self, key: &str, value: T) -> Result<()> {
        self.redis
            .lock()
            .await
            .set(self.redis_key(key), value)
            .await
            .context("cannot write to redis")?;
        Ok(())
    }

    async fn redis_set_members<T: Eq + Hash + FromRedisValue>(
        &self,
        key: &str,
    ) -> Result<HashSet<T>> {
        let r = self
            .redis
            .lock()
            .await
            .smembers(self.redis_key(key))
            .await
            .context("cannot read from redis")?;
        Ok(r)
    }

    async fn redis_set_add<T: ToRedisArgs + Send + Sync>(
        &self,
        key: &str,
        value: T,
    ) -> Result<bool> {
        let n: i32 = self
            .redis
            .lock()
            .await
            .sadd(self.redis_key(key), value)
            .await
            .context("cannot write to redis")?;
        Ok(n != 0)
    }

    async fn redis_set_remove<T: ToRedisArgs + Send + Sync>(
        &self,
        key: &str,
        value: T,
    ) -> Result<bool> {
        let n: i32 = self
            .redis
            .lock()
            .await
            .srem(self.redis_key(key), value)
            .await
            .context("cannot write to redis")?;
        Ok(n != 0)
    }

    async fn redis_flag_get(&self, key: &str, default: bool) -> Result<bool> {
        Ok(match self.redis_get::<u32>(key).await? {
            None => default,
            Some(r) => r != 0,
        })
    }

    async fn redis_flag_set(&self, key: &str, flag: bool) -> Result<()> {
        self.redis_set(key, flag as u32).await
    }
}

impl BotContext for Context {
    fn bot_id(&self) -> UserId {
        self.bot_id
    }
}

#[async_trait::async_trait]
impl GuildContext for Context {
    async fn member_permissions(&self, user_id: UserId) -> Result<Permissions> {
        let member = self
            .guild_id
            .member((&self.cache, &*self.http), user_id)
            .await
            .context("cannot obtain member")?;
        match self.cache.guild(self.guild_id) {
            None => Err(Error::InaccessibleGuild),
            Some(guild) => Ok(guild.member_permissions(&member)),
        }
    }

    async fn connected_voice_channel(&self, user_id: UserId) -> Result<Option<ChannelId>> {
        let voice_states = self.voice_states().await?;

        Ok(match voice_states.get(&user_id) {
            Some(VoiceState {
                channel_id: Some(id),
                ..
            }) => Some(*id),
            _ => None,
        })
    }

    async fn voice_channel_users(&self, channel_id: ChannelId) -> Result<Vec<UserId>> {
        let voice_states = self.voice_states().await?;

        let mut users = Vec::new();
        for (user_id, state) in &voice_states {
            if state.channel_id == Some(channel_id) {
                users.push(*user_id);
            }
        }

        Ok(users)
    }

    async fn disconnect_user(&self, user_id: UserId) -> Result<()> {
        let builder = EditMember::new().disconnect_member();
        self.guild_id
            .edit_member(&self.http, user_id, builder)
            .await
            .context("cannot edit member for disconnection")?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl ChannelContext for Context {
    fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    async fn message(&self, message: crate::model::message::Message) -> Result<()> {
        let message = message.display_say();
        tracing::debug!(%message, "send message");
        self.channel_id
            .say(&self.http, message.to_string())
            .await
            .context("cannot create a message")?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl MessageContext for Context {
    fn author_id(&self) -> UserId {
        self.author_id
    }

    async fn react(&self, reaction: impl Into<ReactionType> + 'async_trait + Send) -> Result<()> {
        let reaction = reaction.into();
        self.channel_id
            .create_reaction(&self.http, self.message_id, reaction)
            .await
            .context("cannot create reaction")?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl RandomContext for Context {
    async fn random_range(&self, from: i64, to: i64) -> i64 {
        self.rng.lock().await.gen_range(from..to)
    }
}

#[async_trait::async_trait]
impl TimeContext for Context {
    fn current_time(&self) -> DateTime<Utc> {
        Utc::now()
    }

    async fn delay_until(&self, time: DateTime<Utc>) {
        let now = self.current_time();
        if let Ok(duration) = (time - now).to_std() {
            tokio::time::sleep(duration).await;
        }
    }
}

#[async_trait::async_trait]
impl SettingContext for Context {
    async fn set_timezone(&self, timezone: Tz) -> Result<()> {
        self.redis_set("timezone", timezone.name()).await
    }

    async fn timezone(&self) -> Result<Tz> {
        Ok(match self.redis_get::<String>("timezone").await? {
            None => chrono_tz::Japan,
            Some(tz_str) => tz_str.parse().unwrap(),
        })
    }

    async fn set_requires_permission(&self, requires_permission: bool) -> Result<()> {
        self.redis_flag_set("requires_permission", requires_permission)
            .await
    }

    async fn requires_permission(&self) -> Result<bool> {
        self.redis_flag_get("requires_permission", true).await
    }

    async fn reminders(&self) -> Result<HashSet<Reminder>> {
        self.redis_set_members("reminders").await
    }

    async fn add_reminder(&self, reminder: Reminder) -> Result<bool> {
        self.redis_set_add("reminders", reminder).await
    }

    async fn remove_reminder(&self, reminder: Reminder) -> Result<bool> {
        self.redis_set_remove("reminders", reminder).await
    }

    async fn reminds_random_kaisan(&self) -> Result<bool> {
        self.redis_flag_get("reminds_random_kaisan", false).await
    }

    async fn set_reminds_random_kaisan(&self, reminds_random_kaisan: bool) -> Result<()> {
        self.redis_flag_set("reminds_random_kaisan", reminds_random_kaisan)
            .await
    }
}

impl Context {
    pub async fn handle_command(&self, command: &str) -> Result<()> {
        let command = command.parse()?;
        tracing::debug!(?command, "parsed message as command");

        match command {
            Command::Help => use_case::Help::help(self).await,
            Command::ShowSetting => use_case::ShowSetting::show_setting(self).await,
            Command::TimeZone(tz) => use_case::SetTimeZone::set_timezone(self, tz).await,
            Command::RequirePermission(b) => {
                use_case::SetRequiresPermission::set_requires_permission(self, b).await
            }
            Command::AddReminder(r) => use_case::AddReminder::add_reminder(self, r).await,
            Command::RemoveReminder(r) => use_case::RemoveReminder::remove_reminder(self, r).await,
            Command::RemindRandomKaisan(b) => {
                use_case::SetRemindsRandomKaisan::set_reminds_random_kaisan(self, b).await
            }
            Command::Kaisan {
                kaisanee,
                time_range,
            } => use_case::ScheduleKaisan::schedule_kaisan(self, kaisanee, time_range).await,
        }
    }
}

#[derive(Clone)]
pub struct ContextBuilder {
    http: Arc<Http>,
    cache: Arc<Cache>,
    bot_id: UserId,
    guild_id: Option<GuildId>,
    author_id: Option<UserId>,
    channel_id: Option<ChannelId>,
    message_id: Option<MessageId>,
    redis_prefix: Option<String>,
    redis_conn: Option<Arc<Mutex<deadpool_redis::Connection>>>,
}

impl ContextBuilder {
    pub fn with_serenity(ctx: &serenity::client::Context) -> Self {
        let bot_id = ctx.cache.current_user().id;
        Self {
            http: Arc::clone(&ctx.http),
            cache: Arc::clone(&ctx.cache),
            bot_id,
            guild_id: None,
            author_id: None,
            channel_id: None,
            message_id: None,
            redis_prefix: None,
            redis_conn: None,
        }
    }

    pub fn redis_prefix(&mut self, prefix: String) -> &mut Self {
        self.redis_prefix = Some(prefix);
        self
    }

    pub fn redis_conn(&mut self, conn: deadpool_redis::Connection) -> &mut Self {
        self.redis_conn = Some(Arc::new(Mutex::new(conn)));
        self
    }

    pub fn guild_id(&mut self, guild_id: GuildId) -> &mut Self {
        self.guild_id = Some(guild_id);
        self
    }

    pub fn message(&mut self, message: &Message) -> &mut Self {
        self.author_id = Some(message.author.id);
        self.channel_id = Some(message.channel_id);
        self.message_id = Some(message.id);
        self
    }

    pub fn build(&self) -> Option<Context> {
        Some(Context {
            http: Arc::clone(&self.http),
            cache: Arc::clone(&self.cache),
            bot_id: self.bot_id,
            guild_id: self.guild_id?,
            author_id: self.author_id?,
            channel_id: self.channel_id?,
            message_id: self.message_id?,
            redis_prefix: self.redis_prefix.clone()?,
            redis: Arc::clone(self.redis_conn.as_ref()?),
            rng: Arc::new(Mutex::new(SmallRng::from_entropy())),
        })
    }
}
