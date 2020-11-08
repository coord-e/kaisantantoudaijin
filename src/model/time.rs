use chrono::{DateTime, Duration, FixedOffset, Utc};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
#[error("invalid hour")]
pub struct InvalidHourError(());

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy)]
pub struct Hour(u8);

impl Hour {
    pub fn from_u8(x: u8) -> Result<Hour, InvalidHourError> {
        if x < 24 {
            Ok(Hour(x))
        } else {
            Err(InvalidHourError(()))
        }
    }

    pub fn as_u32(&self) -> u32 {
        self.0 as u32
    }
}

#[derive(Debug, Clone, Error)]
#[error("invalid hour")]
pub struct InvalidMinuteError(());

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy)]
pub struct Minute(u8);

impl Minute {
    pub fn from_u8(x: u8) -> Result<Minute, InvalidMinuteError> {
        if x < 60 {
            Ok(Minute(x))
        } else {
            Err(InvalidMinuteError(()))
        }
    }

    pub fn as_u32(&self) -> u32 {
        self.0 as u32
    }
}

#[derive(Debug, Clone, Copy)]
pub enum HourMinuteSpecifier<H, M> {
    Hour(H),
    Minute(M),
    Both(H, M),
}

impl<H, M> HourMinuteSpecifier<H, M> {
    pub fn with_hour(h: H, m: Option<M>) -> HourMinuteSpecifier<H, M> {
        match m {
            Some(m) => HourMinuteSpecifier::Both(h, m),
            None => HourMinuteSpecifier::Hour(h),
        }
    }

    pub fn with_minute(m: M, h: Option<H>) -> HourMinuteSpecifier<H, M> {
        match h {
            Some(h) => HourMinuteSpecifier::Both(h, m),
            None => HourMinuteSpecifier::Minute(m),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TimeSpecifier {
    Now,
    After(HourMinuteSpecifier<u8, u8>),
    At {
        time: HourMinuteSpecifier<Hour, Minute>,
        is_tomorrow: bool,
    },
    Exactly(DateTime<FixedOffset>),
}

pub fn calculate_time(spec: TimeSpecifier, now: DateTime<Utc>, tz: FixedOffset) -> DateTime<Utc> {
    match spec {
        TimeSpecifier::Now => now,
        TimeSpecifier::After(dur) => now + calculate_duration(dur),
        TimeSpecifier::At { time, is_tomorrow } => {
            let now_date = now.with_timezone(&tz).date();
            let t = match time {
                HourMinuteSpecifier::Hour(h) => now_date.and_hms(h.as_u32(), 0, 0),
                HourMinuteSpecifier::Minute(m) => now_date.and_hms(0, m.as_u32(), 0),
                HourMinuteSpecifier::Both(h, m) => now_date.and_hms(h.as_u32(), m.as_u32(), 0),
            };
            if is_tomorrow {
                t + Duration::days(1)
            } else {
                t
            }
            .with_timezone(&Utc)
        }
        TimeSpecifier::Exactly(time) => time.with_timezone(&Utc),
    }
}

fn calculate_duration(spec: HourMinuteSpecifier<u8, u8>) -> Duration {
    match spec {
        HourMinuteSpecifier::Hour(h) => Duration::hours(h.into()),
        HourMinuteSpecifier::Minute(m) => Duration::minutes(m.into()),
        HourMinuteSpecifier::Both(h, m) => Duration::hours(h.into()) + Duration::minutes(m.into()),
    }
}
