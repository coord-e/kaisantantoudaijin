use crate::context::{GuildContext, MessageContext, SettingContext};
use crate::error::{Error, Result};

use chrono_tz::Tz;
use serenity::model::permissions::Permissions;

#[async_trait::async_trait]
pub trait SetTimeZone: SettingContext + GuildContext + MessageContext {
    async fn set_timezone(&self, timezone: Tz) -> Result<()> {
        if !self
            .member_permissions(self.author_id())
            .await?
            .manage_guild()
        {
            return Err(Error::InsufficientPermission(Permissions::MANAGE_GUILD));
        }

        SettingContext::set_timezone(self, timezone).await?;
        self.react('✅').await?;
        Ok(())
    }
}

impl<T: SettingContext + GuildContext + MessageContext> SetTimeZone for T {}

#[cfg(test)]
mod tests {
    use super::SetTimeZone;
    use crate::{
        error::Error,
        test::{MockContext, MOCK_AUTHOR_1, MOCK_AUTHOR_2},
    };
    use chrono_tz::Tz;

    #[tokio::test]
    async fn test_success() {
        let ctx = MockContext::with_author(MOCK_AUTHOR_2);
        ctx.set_timezone(Tz::UTC).await.unwrap();
        assert_eq!(*ctx.timezone.lock().await, Tz::UTC);
        ctx.set_timezone(Tz::Japan).await.unwrap();
        assert_eq!(*ctx.timezone.lock().await, Tz::Japan);
    }

    #[tokio::test]
    async fn test_insufficient_permission() {
        let ctx = MockContext::with_author(MOCK_AUTHOR_1);
        assert!(matches!(
            ctx.set_timezone(Tz::UTC).await,
            Err(Error::InsufficientPermission(_))
        ));
    }
}
