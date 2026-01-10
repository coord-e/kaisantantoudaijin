use crate::database::DatabaseValue;
use crate::say::{fmt, Say};

use chrono::Duration;

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

impl From<Reminder> for DatabaseValue {
    fn from(value: Reminder) -> Self {
        DatabaseValue::U32(value.0)
    }
}

impl TryFrom<DatabaseValue> for Reminder {
    type Error = <u32 as TryFrom<DatabaseValue>>::Error;

    fn try_from(value: DatabaseValue) -> Result<Self, Self::Error> {
        let n = u32::try_from(value)?;
        Ok(Reminder(n))
    }
}

impl Say for Reminder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        say!(f, "{}前", self.before_duration())
    }
}
