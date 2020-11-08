use chrono::{DateTime, Duration, FixedOffset, TimeZone, Utc};
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

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
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

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum TimeSpecifier {
    Now,
    After(HourMinuteSpecifier<u8, u8>),
    At {
        time: HourMinuteSpecifier<Hour, Minute>,
        is_tomorrow: bool,
    },
    Exactly(DateTime<FixedOffset>),
}

pub fn calculate_time<T: TimeZone>(
    spec: TimeSpecifier,
    now: DateTime<Utc>,
    tz: T,
) -> DateTime<Utc> {
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

#[cfg(test)]
mod tests {
    use super::{calculate_time, Hour, HourMinuteSpecifier, Minute, TimeSpecifier};

    use chrono::{Duration, FixedOffset, Utc};

    #[test]
    fn test_calculate_time_now() {
        let now = Utc::now();
        let spec = TimeSpecifier::Now;
        assert_eq!(calculate_time(spec, now, Utc), now);
        assert_eq!(calculate_time(spec, now, FixedOffset::east(3600)), now);
    }

    #[test]
    fn test_calculate_time_after() {
        let now = Utc::now();
        let spec = TimeSpecifier::After(HourMinuteSpecifier::Both(3, 15));
        let expected = now + Duration::hours(3) + Duration::minutes(15);
        assert_eq!(calculate_time(spec, now, Utc), expected);
        assert_eq!(calculate_time(spec, now, FixedOffset::east(3600)), expected);
    }

    #[test]
    fn test_calculate_time_at() {
        let now = Utc::now();
        let spec = TimeSpecifier::At {
            time: HourMinuteSpecifier::Both(
                Hour::from_u8(12).unwrap(),
                Minute::from_u8(35).unwrap(),
            ),
            is_tomorrow: false,
        };
        let expected = now.date().and_hms(12, 35, 0);
        assert_eq!(calculate_time(spec, now, Utc), expected);
    }

    #[test]
    fn test_calculate_time_at_tomorrow() {
        let now = Utc::now();
        let spec = TimeSpecifier::At {
            time: HourMinuteSpecifier::Both(
                Hour::from_u8(23).unwrap(),
                Minute::from_u8(25).unwrap(),
            ),
            is_tomorrow: true,
        };
        let expected = now.date().and_hms(23, 25, 0) + Duration::days(1);
        assert_eq!(calculate_time(spec, now, Utc), expected);
    }

    #[test]
    fn test_calculate_time_at_with_tz() {
        let now = Utc::now();
        let spec = TimeSpecifier::At {
            time: HourMinuteSpecifier::Both(
                Hour::from_u8(7).unwrap(),
                Minute::from_u8(15).unwrap(),
            ),
            is_tomorrow: false,
        };
        let expected = now.date().and_hms(22, 15, 0) - Duration::days(1);
        assert_eq!(
            calculate_time(spec, now, FixedOffset::east(9 * 3600)),
            expected
        );
    }

    #[test]
    fn test_calculate_time_exactly() {
        let now = Utc::now();
        let expected = now + Duration::seconds(10);
        let spec = TimeSpecifier::Exactly(expected.with_timezone(&FixedOffset::east(5 * 3600)));
        assert_eq!(calculate_time(spec, now, Utc), expected);
        assert_eq!(calculate_time(spec, now, FixedOffset::east(3600)), expected);
    }
}
