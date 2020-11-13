use crate::say::{fmt, Say};

use chrono::Duration;
use redis::{FromRedisValue, RedisResult, RedisWrite, ToRedisArgs, Value};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy)]
pub struct Reminder(u32);

impl Reminder {
    pub const fn before_minutes(minutes: u32) -> Reminder {
        Reminder(minutes)
    }

    pub fn before_duration(&self) -> Duration {
        Duration::minutes(self.0.into())
    }
}

impl ToRedisArgs for Reminder {
    fn write_redis_args<W: ?Sized>(&self, out: &mut W)
    where
        W: RedisWrite,
    {
        self.0.write_redis_args(out);
    }
}

impl FromRedisValue for Reminder {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        u32::from_redis_value(v).map(Reminder)
    }
}

impl Say for Reminder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        say!(f, "{}前", self.before_duration())
    }
}
