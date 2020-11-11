use crate::context::{ChannelContext, SettingContext};
use crate::error::Result;
use crate::model::message::Message;

#[async_trait::async_trait]
pub trait ShowSetting: SettingContext + ChannelContext {
    async fn show_setting(&self) -> Result<()> {
        let (requires_permission, timezone, reminders) = futures::future::try_join3(
            self.requires_permission(),
            self.timezone(),
            self.reminders(),
        )
        .await?;

        let message = Message::Setting {
            requires_permission,
            timezone,
            reminders,
        };
        self.message(message).await?;

        Ok(())
    }
}

impl<T: SettingContext + ChannelContext> ShowSetting for T {}

#[cfg(test)]
mod tests {
    use super::ShowSetting;
    use crate::{model::message::Message, test::MockContext};
    use std::sync::atomic::Ordering;

    #[tokio::test]
    async fn test() {
        let ctx = MockContext::new();
        let perm = ctx.requires_permission.load(Ordering::SeqCst);
        let tz = *ctx.timezone.lock().await;
        ctx.show_setting().await.unwrap();

        assert!(matches!(
            ctx.sent_messages.lock().await.as_slice(),
            &[Message::Setting { requires_permission, timezone }] if requires_permission == perm && timezone == tz
        ));
    }
}
