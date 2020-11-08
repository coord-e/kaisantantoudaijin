use crate::error::Result;

use chrono_tz::Tz;

#[async_trait::async_trait]
pub trait SettingContext {
    async fn timezone(&self) -> Result<Tz>;
    async fn set_timezone(&self, timezone: Tz) -> Result<()>;
    async fn requires_permission(&self) -> Result<bool>;
    async fn set_requires_permission(&self, requires_permission: bool) -> Result<()>;
}
