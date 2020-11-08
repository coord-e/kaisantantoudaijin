use crate::error::Result;

use serenity::model::id::{ChannelId, UserId};

#[async_trait::async_trait]
pub trait GuildContext {
    async fn connected_voice_channel(&self, user_id: UserId) -> Result<Option<ChannelId>>;
    async fn voice_channel_users(&self, channel_id: ChannelId) -> Result<Vec<UserId>>;
    async fn disconnect_user(&self, user_id: UserId) -> Result<()>;
}
