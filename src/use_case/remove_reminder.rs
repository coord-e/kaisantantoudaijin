use crate::context::{GuildContext, MessageContext, SettingContext};
use crate::error::{Error, Result};
use crate::model::reminder::Reminder;

use serenity::model::permissions::Permissions;

#[async_trait::async_trait]
pub trait RemoveReminder: SettingContext + GuildContext + MessageContext {
    async fn remove_reminder(&self, reminder: Reminder) -> Result<()> {
        if !self
            .member_permissions(self.author_id())
            .await?
            .manage_guild()
        {
            return Err(Error::InsufficientPermission(Permissions::MANAGE_GUILD));
        }

        if !SettingContext::remove_reminder(self, reminder).await? {
            Err(Error::NoSuchReminder(reminder))
        } else {
            self.react('✅').await?;
            Ok(())
        }
    }
}

impl<T: SettingContext + GuildContext + MessageContext> RemoveReminder for T {}
