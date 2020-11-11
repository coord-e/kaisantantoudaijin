use crate::context::{GuildContext, MessageContext, SettingContext};
use crate::error::{Error, Result};

use serenity::model::permissions::Permissions;

#[async_trait::async_trait]
pub trait SetRemindsRandomKaisan: SettingContext + GuildContext + MessageContext {
    async fn set_reminds_random_kaisan(&self, reminds_random_kaisan: bool) -> Result<()> {
        if !self
            .member_permissions(self.author_id())
            .await?
            .manage_guild()
        {
            return Err(Error::InsufficientPermission(Permissions::MANAGE_GUILD));
        }

        SettingContext::set_reminds_random_kaisan(self, reminds_random_kaisan).await?;
        self.react('✅').await?;
        Ok(())
    }
}

impl<T: SettingContext + GuildContext + MessageContext> SetRemindsRandomKaisan for T {}

#[cfg(test)]
mod tests {
    use super::SetRemindsRandomKaisan;
    use crate::{
        error::Error,
        test::{MockContext, MOCK_AUTHOR_1, MOCK_AUTHOR_2},
    };
    use std::sync::atomic::Ordering;

    #[tokio::test]
    async fn test_success() {
        let ctx = MockContext::with_author(MOCK_AUTHOR_2);
        ctx.set_reminds_random_kaisan(false).await.unwrap();
        assert!(!ctx.reminds_random_kaisan.load(Ordering::SeqCst));
        ctx.set_reminds_random_kaisan(true).await.unwrap();
        assert!(ctx.reminds_random_kaisan.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_insufficient_permission() {
        let ctx = MockContext::with_author(MOCK_AUTHOR_1);
        assert!(matches!(
            ctx.set_reminds_random_kaisan(true).await,
            Err(Error::InsufficientPermission(_))
        ));
    }
}
