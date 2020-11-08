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
