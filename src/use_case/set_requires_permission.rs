use crate::context::{GuildContext, MessageContext, SettingContext};
use crate::error::{Error, Result};

use serenity::model::permissions::Permissions;

#[async_trait::async_trait]
pub trait SetRequiresPermission: SettingContext + GuildContext + MessageContext {
    async fn set_requires_permission(&self, requires_permission: bool) -> Result<()> {
        if !self
            .member_permissions(self.author_id())
            .await?
            .manage_guild()
        {
            return Err(Error::InsufficientPermission(Permissions::MANAGE_GUILD));
        }

        SettingContext::set_requires_permission(self, requires_permission).await?;
        self.react('✅').await?;
        Ok(())
    }
}

impl<T: SettingContext + GuildContext + MessageContext> SetRequiresPermission for T {}

#[cfg(test)]
mod tests {
    use super::SetRequiresPermission;
    use crate::{
        error::Error,
        test::{MockContext, MOCK_AUTHOR_1, MOCK_AUTHOR_2},
    };
    use std::sync::atomic::Ordering;

    #[tokio::test]
    async fn test_success() {
        let ctx = MockContext::with_author(MOCK_AUTHOR_2);
        ctx.set_requires_permission(false).await.unwrap();
        assert!(!ctx.requires_permission.load(Ordering::SeqCst));
        ctx.set_requires_permission(true).await.unwrap();
        assert!(ctx.requires_permission.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_insufficient_permission() {
        let ctx = MockContext::with_author(MOCK_AUTHOR_1);
        assert!(matches!(
            ctx.set_requires_permission(true).await,
            Err(Error::InsufficientPermission(_))
        ));
    }
}
