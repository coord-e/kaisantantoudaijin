use crate::context::ChannelContext;
use crate::error::Result;
use crate::model::message::Message;

#[async_trait::async_trait]
pub trait Help: ChannelContext {
    async fn help(&self) -> Result<()> {
        self.message(Message::Help).await
    }
}

impl<T: ChannelContext> Help for T {}
