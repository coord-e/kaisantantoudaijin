use crate::context::MessageContext;
use crate::error::Result;
use crate::model::message::Message;

#[async_trait::async_trait]
pub trait Help: MessageContext {
    async fn help(&self) -> Result<()> {
        self.reply(Message::Help).await
    }
}

impl<T: MessageContext> Help for T {}
