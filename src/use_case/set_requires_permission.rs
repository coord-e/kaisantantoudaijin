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
