use crate::error::Result;
use crate::model::message::Message;

use serenity::model::id::ChannelId;

#[async_trait::async_trait]
pub trait ChannelContext {
    fn channel_id(&self) -> ChannelId;
    async fn message(&self, message: Message) -> Result<()>;
}
