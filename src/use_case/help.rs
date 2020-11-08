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

#[cfg(test)]
mod tests {
    use super::Help;
    use crate::{model::message::Message, test::MockContext};

    #[tokio::test]
    async fn test() {
        let ctx = MockContext::new();
        ctx.help().await.unwrap();
        assert!(matches!(
            ctx.sent_messages.lock().await.as_slice(),
            &[Message::Help]
        ));
    }
}
