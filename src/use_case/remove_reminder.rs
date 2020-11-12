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

#[cfg(test)]
mod tests {
    use super::RemoveReminder;
    use crate::{
        error::Error,
        model::reminder::Reminder,
        test::{MockContext, MOCK_AUTHOR_1, MOCK_AUTHOR_2},
    };

    #[tokio::test]
    async fn test_success() {
        let ctx = MockContext::with_author(MOCK_AUTHOR_2);
        let reminder = Reminder::before_minutes(5);
        ctx.remove_reminder(reminder).await.unwrap();
        assert!(!ctx.reminders.lock().await.contains(&reminder));
    }

    #[tokio::test]
    async fn test_not_found() {
        let ctx = MockContext::with_author(MOCK_AUTHOR_2);
        let reminder = Reminder::before_minutes(10);
        let _ = ctx.remove_reminder(reminder).await;
        assert!(matches!(
            ctx.remove_reminder(reminder).await,
            Err(Error::NoSuchReminder(_))
        ));
    }

    #[tokio::test]
    async fn test_insufficient_permission() {
        let ctx = MockContext::with_author(MOCK_AUTHOR_1);
        assert!(matches!(
            ctx.remove_reminder(Reminder::before_minutes(5)).await,
            Err(Error::InsufficientPermission(_))
        ));
    }
}
