use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::context::{
    BotContext, ChannelContext, GuildContext, MessageContext, RandomContext, SettingContext,
    TimeContext,
};
use crate::error::Result;
use crate::model::message::Message;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use futures::lock::Mutex;
use serenity::model::{
    channel::ReactionType,
    id::{ChannelId, UserId},
    permissions::Permissions,
};

pub const MOCK_BOT_ID: UserId = UserId(6455241911587596288);
pub const MOCK_CHANNEL_ID: ChannelId = ChannelId(7933013268500803584);
pub const MOCK_VOICE_CHANNEL_ID: ChannelId = ChannelId(8549307414562138112);

pub const MOCK_AUTHOR_1: UserId = UserId(17308610930080528384);
pub const MOCK_AUTHOR_2: UserId = UserId(4081392650864611328);

pub const FIXED_RANDOM: i64 = 12345;

lazy_static::lazy_static! {
    pub static ref MOCK_USERS: HashMap<UserId, Permissions> = {
        let mut m = HashMap::new();
        m.insert(MOCK_AUTHOR_1, Permissions::empty());
        m.insert(MOCK_AUTHOR_2, Permissions::all());
        m
    };
    pub static ref MOCK_VOICE_STATES: HashMap<UserId, ChannelId> = {
        let mut m = HashMap::new();
        m.insert(MOCK_AUTHOR_1, MOCK_VOICE_CHANNEL_ID);
        m.insert(MOCK_AUTHOR_2, MOCK_VOICE_CHANNEL_ID);
        m
    };
}

#[derive(Clone)]
pub struct MockContext {
    pub author_id: UserId,
    pub current_time: DateTime<Utc>,
    pub sent_messages: Arc<Mutex<Vec<Message>>>,
    pub disconnected_users: Arc<Mutex<Vec<UserId>>>,
    pub added_reactions: Arc<Mutex<Vec<ReactionType>>>,
    pub requires_permission: Arc<AtomicBool>,
    pub timezone: Arc<Mutex<Tz>>,
}

impl MockContext {
    pub fn new() -> MockContext {
        MockContext::with_author(MOCK_AUTHOR_2)
    }

    pub fn with_author(author_id: UserId) -> MockContext {
        MockContext::with_author_current_time(author_id, Utc::now())
    }

    pub fn with_current_time(current_time: DateTime<Utc>) -> MockContext {
        MockContext::with_author_current_time(MOCK_AUTHOR_2, current_time)
    }

    pub fn with_author_current_time(author_id: UserId, current_time: DateTime<Utc>) -> MockContext {
        MockContext {
            author_id,
            current_time,
            sent_messages: Arc::new(Mutex::new(Vec::new())),
            disconnected_users: Arc::new(Mutex::new(Vec::new())),
            added_reactions: Arc::new(Mutex::new(Vec::new())),
            requires_permission: Arc::new(AtomicBool::new(true)),
            timezone: Arc::new(Mutex::new(Tz::Japan)),
        }
    }
}

impl BotContext for MockContext {
    fn bot_id(&self) -> UserId {
        MOCK_BOT_ID
    }
}

#[async_trait::async_trait]
impl GuildContext for MockContext {
    async fn member_permissions(&self, user_id: UserId) -> Result<Permissions> {
        Ok(MOCK_USERS[&user_id])
    }

    async fn connected_voice_channel(&self, user_id: UserId) -> Result<Option<ChannelId>> {
        Ok(MOCK_VOICE_STATES.get(&user_id).copied())
    }

    async fn voice_channel_users(&self, channel_id: ChannelId) -> Result<Vec<UserId>> {
        let mut users = Vec::new();
        for (user_id, state_channel_id) in MOCK_VOICE_STATES.iter() {
            if state_channel_id == &channel_id {
                users.push(*user_id);
            }
        }
        Ok(users)
    }

    async fn disconnect_user(&self, user_id: UserId) -> Result<()> {
        self.disconnected_users.lock().await.push(user_id);
        Ok(())
    }
}

#[async_trait::async_trait]
impl ChannelContext for MockContext {
    fn channel_id(&self) -> ChannelId {
        MOCK_CHANNEL_ID
    }

    async fn message(&self, message: Message) -> Result<()> {
        self.sent_messages.lock().await.push(message);
        Ok(())
    }
}

#[async_trait::async_trait]
impl MessageContext for MockContext {
    fn author_id(&self) -> UserId {
        self.author_id
    }

    async fn react(&self, reaction: impl Into<ReactionType> + 'async_trait + Send) -> Result<()> {
        self.added_reactions.lock().await.push(reaction.into());
        Ok(())
    }
}

#[async_trait::async_trait]
impl RandomContext for MockContext {
    async fn random_range(&self, from: i64, to: i64) -> i64 {
        let r = from + FIXED_RANDOM;
        if r >= to {
            to
        } else {
            r
        }
    }
}

impl TimeContext for MockContext {
    fn current_time(&self) -> DateTime<Utc> {
        self.current_time
    }
}

#[async_trait::async_trait]
impl SettingContext for MockContext {
    async fn set_timezone(&self, timezone: Tz) -> Result<()> {
        *self.timezone.lock().await = timezone;
        Ok(())
    }

    async fn timezone(&self) -> Result<Tz> {
        Ok(*self.timezone.lock().await)
    }

    async fn set_requires_permission(&self, requires_permission: bool) -> Result<()> {
        self.requires_permission
            .store(requires_permission, Ordering::SeqCst);
        Ok(())
    }

    async fn requires_permission(&self) -> Result<bool> {
        Ok(self.requires_permission.load(Ordering::SeqCst))
    }
}
