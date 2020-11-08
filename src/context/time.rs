use chrono::{DateTime, Utc};

pub trait TimeContext {
    fn current_time(&self) -> DateTime<Utc>;
}
