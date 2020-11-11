use std::collections::HashSet;

use crate::error::Result;
use crate::model::reminder::Reminder;

use chrono_tz::Tz;

#[async_trait::async_trait]
pub trait SettingContext {
    async fn timezone(&self) -> Result<Tz>;
    async fn set_timezone(&self, timezone: Tz) -> Result<()>;
    async fn requires_permission(&self) -> Result<bool>;
    async fn set_requires_permission(&self, requires_permission: bool) -> Result<()>;
    async fn reminders(&self) -> Result<HashSet<Reminder>>;
    async fn add_reminder(&self, reminder: Reminder) -> Result<bool>;
    async fn remove_reminder(&self, reminder: Reminder) -> Result<bool>;
    async fn reminds_random_kaisan(&self) -> Result<bool>;
    async fn set_reminds_random_kaisan(&self, reminds_random_kaisan: bool) -> Result<()>;
}
