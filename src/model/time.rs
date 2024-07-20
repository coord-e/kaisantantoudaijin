use chrono::{DateTime, Duration, FixedOffset, TimeZone, Timelike, Utc};
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
pub enum AfterTimeSpecifier {
    Hour(u8),
    Minute(u8),
    HourMinute(u8, u8),
    Second(u8),
}

impl AfterTimeSpecifier {
    pub fn with_hour(h: u8, m: Option<u8>) -> AfterTimeSpecifier {
        match m {
            Some(m) => AfterTimeSpecifier::HourMinute(h, m),
            None => AfterTimeSpecifier::Hour(h),
        }
    }

    pub fn with_minute(m: u8, h: Option<u8>) -> AfterTimeSpecifier {
        match h {
            Some(h) => AfterTimeSpecifier::HourMinute(h, m),
            None => AfterTimeSpecifier::Minute(m),
        }
    }

    fn calculate_duration(&self) -> Duration {
        match *self {
            AfterTimeSpecifier::Hour(h) => Duration::hours(h.into()),
            AfterTimeSpecifier::Minute(m) => Duration::minutes(m.into()),
            AfterTimeSpecifier::HourMinute(h, m) => {
                Duration::hours(h.into()) + Duration::minutes(m.into())
            }
            AfterTimeSpecifier::Second(s) => Duration::seconds(s.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum AtTimeSpecifier {
    Hour {
        hour: Hour,
        is_tomorrow: bool,
    },
    Minute(Minute),
    HourMinute {
        hour: Hour,
        minute: Minute,
        is_tomorrow: bool,
    },
}

impl AtTimeSpecifier {
    pub fn with_hour(hour: Hour, minute: Option<Minute>, is_tomorrow: bool) -> AtTimeSpecifier {
        match minute {
            Some(minute) => AtTimeSpecifier::HourMinute {
                hour,
                minute,
                is_tomorrow,
            },
            None => AtTimeSpecifier::Hour { hour, is_tomorrow },
        }
    }

    pub fn with_minute(minute: Minute, hour: Option<Hour>) -> AtTimeSpecifier {
        match hour {
            Some(hour) => AtTimeSpecifier::HourMinute {
                hour,
                minute,
                is_tomorrow: false,
            },
            None => AtTimeSpecifier::Minute(minute),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum TimeSpecifier {
    After(AfterTimeSpecifier),
    At(AtTimeSpecifier),
    Exactly(DateTime<FixedOffset>),
}

impl TimeSpecifier {
    pub fn calculate_time<T: TimeZone>(&self, now: DateTime<Utc>, tz: T) -> Option<DateTime<Utc>> {
        match self {
            TimeSpecifier::After(dur) => Some(now + dur.calculate_duration()),
            TimeSpecifier::At(time) => {
                let now = now.with_timezone(&tz);
                let now_date = now.date_naive();
                match time {
                    AtTimeSpecifier::Hour { hour, is_tomorrow } => {
                        let t = now_date.and_hms_opt(hour.as_u32(), 0, 0)?;
                        if *is_tomorrow {
                            t + Duration::days(1)
                        } else {
                            t
                        }
                    }
                    AtTimeSpecifier::Minute(m) => {
                        now_date.and_hms_opt(now.hour(), m.as_u32(), 0)?
                    }
                    AtTimeSpecifier::HourMinute {
                        hour,
                        minute,
                        is_tomorrow,
                    } => {
                        let t = now_date.and_hms_opt(hour.as_u32(), minute.as_u32(), 0)?;
                        if *is_tomorrow {
                            t + Duration::days(1)
                        } else {
                            t
                        }
                    }
                }
                .and_local_timezone(tz)
                .single()
                .map(|t| t.to_utc())
            }
            TimeSpecifier::Exactly(time) => Some(time.with_timezone(&Utc)),
        }
    }

    pub fn is_interested_in_time(&self) -> bool {
        !matches!(self, TimeSpecifier::At(_))
    }

    pub fn is_interested_in_duration(&self) -> bool {
        !matches!(self, TimeSpecifier::After(_))
    }
}

#[cfg(test)]
mod tests {
    use super::{AfterTimeSpecifier, AtTimeSpecifier, Hour, Minute, TimeSpecifier};

    use chrono::{DateTime, Duration, FixedOffset, Utc};

    #[test]
    fn test_calculate_time_after() {
        let now = DateTime::parse_from_rfc3339("2024-07-20T13:15:00Z")
            .unwrap()
            .to_utc();
        let spec = TimeSpecifier::After(AfterTimeSpecifier::HourMinute(3, 15));
        let expected = DateTime::parse_from_rfc3339("2024-07-20T16:30:00Z")
            .unwrap()
            .to_utc();
        assert_eq!(spec.calculate_time(now, Utc), Some(expected));
        assert_eq!(
            spec.calculate_time(now, FixedOffset::east_opt(3600).unwrap()),
            Some(expected)
        );
    }

    #[test]
    fn test_calculate_time_at() {
        let now = DateTime::parse_from_rfc3339("2024-07-20T13:15:00Z")
            .unwrap()
            .to_utc();
        let spec = TimeSpecifier::At(AtTimeSpecifier::HourMinute {
            hour: Hour::from_u8(12).unwrap(),
            minute: Minute::from_u8(35).unwrap(),
            is_tomorrow: false,
        });
        let expected = DateTime::parse_from_rfc3339("2024-07-20T12:35:00Z")
            .unwrap()
            .to_utc();
        assert_eq!(spec.calculate_time(now, Utc), Some(expected));
    }

    #[test]
    fn test_calculate_time_at_minute_with_tz() {
        let tz = FixedOffset::east_opt(9 * 3600).unwrap();
        let now_utc = DateTime::parse_from_rfc3339("2024-07-20T13:15:00+09:00")
            .unwrap()
            .to_utc();
        let spec = TimeSpecifier::At(AtTimeSpecifier::Minute(Minute::from_u8(35).unwrap()));
        let expected = DateTime::parse_from_rfc3339("2024-07-20T13:35:00+09:00")
            .unwrap()
            .to_utc();
        assert_eq!(spec.calculate_time(now_utc, tz), Some(expected));
    }

    #[test]
    fn test_calculate_time_at_tomorrow() {
        let now = DateTime::parse_from_rfc3339("2024-07-20T13:15:00Z")
            .unwrap()
            .to_utc();
        let spec = TimeSpecifier::At(AtTimeSpecifier::HourMinute {
            hour: Hour::from_u8(23).unwrap(),
            minute: Minute::from_u8(25).unwrap(),
            is_tomorrow: true,
        });
        let expected = DateTime::parse_from_rfc3339("2024-07-21T23:25:00Z")
            .unwrap()
            .to_utc();
        assert_eq!(spec.calculate_time(now, Utc), Some(expected));
    }

    #[test]
    fn test_calculate_time_at_with_tz() {
        let now = DateTime::parse_from_rfc3339("2024-07-20T03:05:00+09:00")
            .unwrap()
            .to_utc();
        let spec = TimeSpecifier::At(AtTimeSpecifier::HourMinute {
            hour: Hour::from_u8(7).unwrap(),
            minute: Minute::from_u8(15).unwrap(),
            is_tomorrow: false,
        });
        let tz = FixedOffset::east_opt(9 * 3600).unwrap();
        let expected = DateTime::parse_from_rfc3339("2024-07-20T07:15:00+09:00")
            .unwrap()
            .to_utc();
        assert_eq!(spec.calculate_time(now, tz), Some(expected));
    }

    #[test]
    fn test_calculate_time_exactly() {
        let now = Utc::now();
        let expected = now + Duration::seconds(10);
        let spec = TimeSpecifier::Exactly(
            expected.with_timezone(&FixedOffset::east_opt(5 * 3600).unwrap()),
        );
        assert_eq!(spec.calculate_time(now, Utc), Some(expected));
        assert_eq!(
            spec.calculate_time(now, FixedOffset::east_opt(3600).unwrap()),
            Some(expected)
        );
    }
}
