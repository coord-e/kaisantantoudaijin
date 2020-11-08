use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::model::command::Command;
use crate::use_case::{Help, ScheduleKaisan};

use anyhow::Context as _;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use futures::lock::Mutex;
use log::{debug, info};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use serenity::{
    builder::EditMember,
    cache::Cache,
    http::client::Http,
    model::{
        channel::{Message, ReactionType},
        id::{ChannelId, GuildId, MessageId, UserId},
        voice::VoiceState,
    },
};

mod bot;
mod channel;
mod config;
mod guild;
mod message;
mod random;
mod time;

pub use bot::BotContext;
pub use channel::ChannelContext;
pub use config::ConfigContext;
pub use guild::GuildContext;
pub use message::MessageContext;
pub use random::RandomContext;
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
    timezone: Tz,
    rng: Arc<Mutex<SmallRng>>,
}

impl Context {
    async fn voice_states(&self) -> Result<HashMap<UserId, VoiceState>> {
        match self
            .cache
            .guild_field(self.guild_id, |g| g.voice_states.clone())
            .await
        {
            None => Err(Error::InaccessibleGuild),
            Some(x) => Ok(x),
        }
    }
}

impl BotContext for Context {
    fn bot_id(&self) -> UserId {
        self.bot_id
    }
}

#[async_trait::async_trait]
impl GuildContext for Context {
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
        self.guild_id
            .edit_member(&self.http, user_id, EditMember::disconnect_member)
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
        debug!("send message: {}", message);
        self.channel_id
            .say(&self.http, message)
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
        self.rng.lock().await.gen_range(from, to)
    }
}

impl TimeContext for Context {
    fn current_time(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[async_trait::async_trait]
impl ConfigContext for Context {
    fn timezone(&self) -> Tz {
        self.timezone
    }
}

impl Context {
    pub async fn new(
        http: Arc<Http>,
        cache: Arc<Cache>,
        timezone: Tz,
        message: &Message,
    ) -> Option<Context> {
        let bot_id = cache.current_user_id().await;

        let guild_id = match message.guild_id {
            None => return None,
            Some(id) => id,
        };

        Some(Context {
            http,
            cache,
            bot_id,
            guild_id,
            author_id: message.author.id,
            channel_id: message.channel_id,
            message_id: message.id,
            timezone,
            rng: Arc::new(Mutex::new(SmallRng::from_entropy())),
        })
    }

    fn extract_command<'a>(&self, content: &'a str) -> Option<&'a str> {
        let my_mention = format!("<@!{}>", self.bot_id);
        content
            .strip_prefix(&my_mention)
            .or_else(|| content.strip_suffix(&my_mention))
            .or_else(|| content.strip_prefix("!kaisan"))
            .map(str::trim)
    }

    pub async fn handle_message(&self, message: Message) -> Result<()> {
        info!("handle message: {}", message.content);

        let command = match self.extract_command(&message.content) {
            None => return Ok(()),
            Some(s) => s.parse()?,
        };

        debug!("parsed message as command: {:?}", command);

        match command {
            Command::Help => self.help().await,
            Command::Kaisan {
                kaisanee,
                time_range,
            } => self.schedule_kaisan(kaisanee, time_range).await,
        }
    }
}
