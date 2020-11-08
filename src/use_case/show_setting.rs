use crate::context::{ChannelContext, SettingContext};
use crate::error::Result;
use crate::model::message::Message;

#[async_trait::async_trait]
pub trait ShowSetting: SettingContext + ChannelContext {
    async fn show_setting(&self) -> Result<()> {
        let requires_permission = self.requires_permission().await?;
        let timezone = self.timezone().await?;

        let message = Message::Setting {
            requires_permission,
            timezone,
        };
        self.message(message).await?;

        Ok(())
    }
}

impl<T: SettingContext + ChannelContext> ShowSetting for T {}
