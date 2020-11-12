use chrono::{DateTime, Utc};

#[async_trait::async_trait]
pub trait TimeContext {
    fn current_time(&self) -> DateTime<Utc>;
    async fn delay_until(&self, time: DateTime<Utc>);
}
