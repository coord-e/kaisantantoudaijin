use crate::context::{GuildContext, MessageContext, SettingContext};
use crate::error::{Error, Result};
use crate::model::reminder::Reminder;

use serenity::model::permissions::Permissions;

#[async_trait::async_trait]
pub trait AddReminder: SettingContext + GuildContext + MessageContext {
    async fn add_reminder(&self, reminder: Reminder) -> Result<()> {
        if !self
            .member_permissions(self.author_id())
            .await?
            .manage_guild()
        {
            return Err(Error::InsufficientPermission(Permissions::MANAGE_GUILD));
        }

        if !SettingContext::add_reminder(self, reminder).await? {
            Err(Error::DuplicatedReminders(reminder))
        } else {
            self.react('✅').await?;
            Ok(())
        }
    }
}

impl<T: SettingContext + GuildContext + MessageContext> AddReminder for T {}

#[cfg(test)]
mod tests {
    use super::AddReminder;
    use crate::{
        error::Error,
        model::reminder::Reminder,
        test::{MockContext, MOCK_AUTHOR_1, MOCK_AUTHOR_2},
    };

    #[tokio::test]
    async fn test_success() {
        let ctx = MockContext::with_author(MOCK_AUTHOR_2);
        let reminder1 = Reminder::before_minutes(10);
        ctx.add_reminder(reminder1).await.unwrap();
        assert!(ctx.reminders.lock().await.contains(&reminder1));
        let reminder2 = Reminder::before_minutes(15);
        ctx.add_reminder(reminder2).await.unwrap();
        assert!(ctx.reminders.lock().await.contains(&reminder2));
    }

    #[tokio::test]
    async fn test_duplicate() {
        let ctx = MockContext::with_author(MOCK_AUTHOR_2);
        let reminder = Reminder::before_minutes(10);
        ctx.add_reminder(reminder).await.unwrap();
        assert!(matches!(
            ctx.add_reminder(reminder).await,
            Err(Error::DuplicatedReminders(_))
        ));
    }

    #[tokio::test]
    async fn test_insufficient_permission() {
        let ctx = MockContext::with_author(MOCK_AUTHOR_1);
        assert!(matches!(
            ctx.add_reminder(Reminder::before_minutes(5)).await,
            Err(Error::InsufficientPermission(_))
        ));
    }
}
