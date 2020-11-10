use std::error::Error;
use std::fmt::{self, Display};
use std::str::FromStr;

use chrono::DateTime;
use chrono_tz::Tz;
use serenity::model::id::UserId;

use crate::model::{
    kaisanee::KaisaneeSpecifier,
    time::{AfterTimeSpecifier, AtTimeSpecifier, Hour, Minute, TimeSpecifier},
};

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum TimeRangeSpecifier {
    By(TimeSpecifier),
    At(TimeSpecifier),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Kaisan {
        kaisanee: KaisaneeSpecifier,
        time_range: TimeRangeSpecifier,
    },
    ShowSetting,
    TimeZone(Tz),
    RequirePermission(bool),
    Help,
}

#[derive(Debug, Clone)]
pub struct ParseCommandError {
    got: Option<String>,
    expected: peg::error::ExpectedSet,
}

impl Display for ParseCommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} is expected", self.expected)?;
        if let Some(got) = &self.got {
            write!(f, ", but got {}", got)?;
        }
        Ok(())
    }
}

impl Error for ParseCommandError {}

impl FromStr for Command {
    type Err = ParseCommandError;

    fn from_str(input: &str) -> Result<Command, Self::Err> {
        parser::command(input).map_err(|e| ParseCommandError {
            got: input.get(e.location.offset..).map(ToOwned::to_owned),
            expected: e.expected,
        })
    }
}

peg::parser! {
  grammar parser() for str {
    rule _() = quiet! { [' ']* }

    rule me()
      = quiet! {
          "me" / "Me"
          / "私" / "わたし"
          / "俺" / "おれ" / "オレ"
          / "僕" / "ぼく" / "ボク"
      } / expected!("me")

    rule all()
      = quiet! {
          "all" / "All" / "全員" / "皆" / "みんな"
      } / expected!("all")

    rule user() -> UserId
      = "<@!" n:$(['0'..='9']+) ">" { UserId(n.parse().unwrap()) }

    rule users() -> Vec<UserId>
      = l:user() ** _ {? if l.is_empty() { Err("non-empty list of users") } else { Ok(l) } }

    pub rule kaisanee() -> KaisaneeSpecifier
      = me() { KaisaneeSpecifier::Me }
      / all() { KaisaneeSpecifier::All }
      / l:users() { KaisaneeSpecifier::Users(l) }

    rule second_suffix()
      = "seconds" / "second" / "sec" / "s" / "秒"

    rule minute_suffix()
      = "minutes" / "minute" / "min" / "m" / "分"

    rule hour_suffix()
      = "hours" / "hour" / "hr" / "h" / "時間"

    rule kanji_number_digit() -> u8
      = ['一'] { 1 }
      / ['二'] { 2 }
      / ['三'] { 3 }
      / ['四'] { 4 }
      / ['五'] { 5 }
      / ['六'] { 6 }
      / ['七'] { 7 }
      / ['八'] { 8 }
      / ['九'] { 9 }

    rule kanji_number_tail(x: u8) -> u8
      = ['十'] d:kanji_number_digit()? { x * 10 + d.unwrap_or(0) }
      / ['百'] d:kanji_number()? {?
          if d < Some(100) {
              Ok(x * 100 + d.unwrap_or(0))
          } else {
              Err("kanji number")
          }
      }

    rule kanji_number() -> u8
      = quiet! {
          kanji_number_tail(1)
          / x:kanji_number_digit() y:kanji_number_tail(x)? { y.unwrap_or(x) }
        } / expected!("kanji number")

    rule number() -> u8
      = quiet! {
          x:$(['0'..='9']*<1,3>) {? x.parse().map_err(|_| "0~255") }
          / kanji_number()
      } / expected!("number")

    rule boolean() -> bool
      = quiet! {
          "true" { true }
          / "false" { false }
          / "yes" { true }
          / "no" { false }
          / "はい" { true }
          / "いいえ" { false }
      } / expected!("boolean")

    rule minute() -> Minute
      = n:(
          t:$(['0'..='9']*<1,2>) { t.parse().unwrap() }
          / kanji_number()
      ) {? Minute::from_u8(n).map_err(|_| "minute") }

    rule hour() -> Hour
      = n:(
          t:$(['0'..='9']*<1,2>) { t.parse().unwrap() }
          / kanji_number()
      ) {? Hour::from_u8(n).map_err(|_| "hour") }

    rule spec_minute() -> Minute
      = ['半'] _ { Minute::from_u8(30).unwrap() }
      / m:minute() _ ['分'] _ { m }

    rule spec_at_tomorrow() -> TimeSpecifier
      = "明日の" _ h:hour() s:(
          [':'] m:minute() _ { AtTimeSpecifier::HourMinute { hour: h, minute: m, is_tomorrow: true } }
          / _ ['時'] _ m:spec_minute()? { AtTimeSpecifier::with_hour(h, m, true) }
      ) { TimeSpecifier::At(s) }

    rule spec_at_rfc3339() -> TimeSpecifier
      = "rfc3339" _ t:$(['T' | 'Z' | '+' | '-' | '.' | ':' | '0'..='9']+) _ {?
          match DateTime::parse_from_rfc3339(t) {
              Ok(t) => Ok(TimeSpecifier::Exactly(t)),
              Err(_) => Err("rfc3339 time"),
          }
      }

    rule spec_at_tail(x: u8) -> TimeSpecifier
      = [':'] m:minute() _ t:("tomorrow" _)? {?
          Hour::from_u8(x).map(|hour| {
              TimeSpecifier::At(AtTimeSpecifier::HourMinute { hour, minute: m, is_tomorrow: t.is_some() })
          }).map_err(|_| "hour")
      }
      / _ ['分'] _ {?
          Minute::from_u8(x).map(|m| {
              TimeSpecifier::At(AtTimeSpecifier::with_minute(m, None))
          }).map_err(|_| "minute")
      }
      / _ ['時'] _ m:spec_minute()? {?
          Hour::from_u8(x).map(|h| {
              TimeSpecifier::At(AtTimeSpecifier::with_hour(h, m, false))
          }).map_err(|_| "hour")
      }

    rule spec_at_half() -> TimeSpecifier
      = ['半'] _ { TimeSpecifier::At(AtTimeSpecifier::Minute(Minute::from_u8(30).unwrap())) }

    rule spec_at() -> TimeSpecifier
      = x:number() spec:spec_at_tail(x) { spec }
      / spec_at_tomorrow()
      / spec_at_rfc3339()
      / spec_at_half()

    rule spec_after() -> TimeSpecifier
      = x:number() _ spec:(
          minute_suffix() _ { AfterTimeSpecifier::with_minute(x, None) }
          / second_suffix() _ { AfterTimeSpecifier::Second(x) }
          / hour_suffix() _ m:(m:number() _ minute_suffix() _ { m })? { AfterTimeSpecifier::with_hour(x, m) }
      ) { TimeSpecifier::After(spec) }

    rule spec_after_suffix(spec: AfterTimeSpecifier) -> TimeRangeSpecifier
      = s:$("後まで" / ['後'] / "以内") {
          let spec = TimeSpecifier::After(spec);
          match s {
              "以内" | "後まで" => TimeRangeSpecifier::By(spec),
              "後" => TimeRangeSpecifier::At(spec),
              _ => unreachable!(),
          }
      }

    pub rule time_range() -> TimeRangeSpecifier
      = x:number() spec:(
          _ second_suffix() _ spec:spec_after_suffix((AfterTimeSpecifier::Second(x))) { spec }
          / _ minute_suffix() _ spec:spec_after_suffix((AfterTimeSpecifier::Minute(x))) { spec }
          / _ hour_suffix() _ m:(m:number() _ minute_suffix() _ { m })? spec:spec_after_suffix((AfterTimeSpecifier::with_hour(x, m))) { spec }
          / spec:spec_at_tail(x) s:"まで"? {
              if s.is_some() {
                  TimeRangeSpecifier::By(spec)
              } else {
                  TimeRangeSpecifier::At(spec)
              }
          }
        ) { spec }
      / spec:(spec_at_tomorrow() / spec_at_rfc3339() / spec_at_half()) s:"まで"? {
          if s.is_some() {
              TimeRangeSpecifier::By(spec)
          } else {
              TimeRangeSpecifier::At(spec)
          }
      }
      / ("now" / "今すぐ") { TimeRangeSpecifier::At(TimeSpecifier::Now) }
      / "at" _ spec:spec_at() { TimeRangeSpecifier::At(spec) }
      / "by" _ spec:spec_at() { TimeRangeSpecifier::By(spec) }
      / "after" _ spec:spec_after() { TimeRangeSpecifier::At(spec) }
      / "within" _ spec:spec_after() { TimeRangeSpecifier::By(spec) }

    rule spec_kaisanee() -> KaisaneeSpecifier
       = k:kaisanee() _ (['を'] _)? { k }

    pub rule command() -> Command
      = "help" { Command::Help }
      / "require-permission" _ b:boolean() { Command::RequirePermission(b) }
      / "timezone" _ tz:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '+' | '-' | '/' ]+) {?
          match tz.parse() {
              Ok(tz) => Ok(Command::TimeZone(tz)),
              Err(_) => Err("timezone")
          }
      }
      / "show-setting" { Command::ShowSetting }
      / kaisanee1:spec_kaisanee()? time_range:time_range() _ (['に'] _)? kaisanee2:spec_kaisanee()? "解散"? {?
          match (kaisanee1, kaisanee2) {
              (Some(kaisanee), None) | (None, Some(kaisanee)) => Ok(Command::Kaisan { kaisanee, time_range }),
              (None, None) => Ok(Command::Kaisan { kaisanee: KaisaneeSpecifier::default(), time_range }),
              (Some(_), Some(_)) => Err("kaisanee specified twice"),
          }
      }
  }
}

#[cfg(test)]
mod tests {
    use super::{parser, Command, TimeRangeSpecifier};
    use crate::model::{
        kaisanee::KaisaneeSpecifier,
        time::{AfterTimeSpecifier, AtTimeSpecifier, Hour, Minute, TimeSpecifier},
    };

    use chrono_tz::Tz;
    use serenity::model::id::UserId;

    #[test]
    fn test_help_command() {
        assert_eq!(parser::command("help"), Ok(Command::Help));
    }

    #[test]
    fn test_setting_command() {
        assert_eq!(
            parser::command("timezone UTC"),
            Ok(Command::TimeZone(Tz::UTC))
        );
        assert_eq!(
            parser::command("timezone Etc/GMT+0"),
            Ok(Command::TimeZone(Tz::Etc__GMTPlus0))
        );
        assert!(parser::command("timezone NoSuchTZ").is_err());
        assert_eq!(
            parser::command("require-permission はい"),
            Ok(Command::RequirePermission(true))
        );
        assert_eq!(
            parser::command("require-permission no"),
            Ok(Command::RequirePermission(false))
        );
        assert_eq!(parser::command("show-setting"), Ok(Command::ShowSetting));
    }

    #[test]
    fn test_kaisan_command_ja() {
        assert_eq!(
            parser::command("明日の1時に"),
            Ok(Command::Kaisan {
                kaisanee: KaisaneeSpecifier::All,
                time_range: TimeRangeSpecifier::At(TimeSpecifier::At(AtTimeSpecifier::Hour {
                    hour: Hour::from_u8(1).unwrap(),
                    is_tomorrow: true,
                }))
            })
        );
        assert_eq!(
            parser::command("10分後 私"),
            Ok(Command::Kaisan {
                kaisanee: KaisaneeSpecifier::Me,
                time_range: TimeRangeSpecifier::At(TimeSpecifier::After(
                    AfterTimeSpecifier::Minute(10)
                ))
            })
        );
        assert_eq!(
            parser::command("10分に私を解散"),
            Ok(Command::Kaisan {
                kaisanee: KaisaneeSpecifier::Me,
                time_range: TimeRangeSpecifier::At(TimeSpecifier::At(AtTimeSpecifier::Minute(
                    Minute::from_u8(10).unwrap()
                ))),
            })
        );
        assert_eq!(
            parser::command("全員を一分後"),
            Ok(Command::Kaisan {
                kaisanee: KaisaneeSpecifier::All,
                time_range: TimeRangeSpecifier::At(TimeSpecifier::After(
                    AfterTimeSpecifier::Minute(1)
                ))
            })
        );
    }

    #[test]
    fn test_kaisan_command_en() {
        assert_eq!(
            parser::command("me 10:10"),
            Ok(Command::Kaisan {
                kaisanee: KaisaneeSpecifier::Me,
                time_range: TimeRangeSpecifier::At(TimeSpecifier::At(
                    AtTimeSpecifier::HourMinute {
                        hour: Hour::from_u8(10).unwrap(),
                        minute: Minute::from_u8(10).unwrap(),
                        is_tomorrow: false,
                    }
                ))
            })
        );
        assert_eq!(
            parser::command("10:10 tomorrow"),
            Ok(Command::Kaisan {
                kaisanee: KaisaneeSpecifier::All,
                time_range: TimeRangeSpecifier::At(TimeSpecifier::At(
                    AtTimeSpecifier::HourMinute {
                        hour: Hour::from_u8(10).unwrap(),
                        minute: Minute::from_u8(10).unwrap(),
                        is_tomorrow: true,
                    }
                ))
            })
        );
    }

    #[test]
    fn test_kaisanee_ja() {
        assert_eq!(parser::kaisanee("全員"), Ok(KaisaneeSpecifier::All));
        assert_eq!(parser::kaisanee("わたし"), Ok(KaisaneeSpecifier::Me));
    }

    #[test]
    fn test_kaisanee_en() {
        assert_eq!(parser::kaisanee("All"), Ok(KaisaneeSpecifier::All));
        assert_eq!(parser::kaisanee("me"), Ok(KaisaneeSpecifier::Me));
        assert_eq!(
            parser::kaisanee("<@!12345> <@!45678><@!99999>"),
            Ok(KaisaneeSpecifier::Users(vec![
                UserId(12345),
                UserId(45678),
                UserId(99999)
            ]))
        );
    }

    #[test]
    fn test_now_ja() {
        assert_eq!(
            parser::time_range("今すぐ"),
            Ok(TimeRangeSpecifier::At(TimeSpecifier::Now))
        );
    }

    #[test]
    fn test_now_en() {
        assert_eq!(
            parser::time_range("now"),
            Ok(TimeRangeSpecifier::At(TimeSpecifier::Now))
        );
    }

    #[test]
    fn test_at_ja() {
        assert_eq!(
            parser::time_range("二十分"),
            Ok(TimeRangeSpecifier::At(TimeSpecifier::At(
                AtTimeSpecifier::Minute(Minute::from_u8(20).unwrap())
            )))
        );
        assert_eq!(
            parser::time_range("0:15"),
            Ok(TimeRangeSpecifier::At(TimeSpecifier::At(
                AtTimeSpecifier::HourMinute {
                    hour: Hour::from_u8(0).unwrap(),
                    minute: Minute::from_u8(15).unwrap(),
                    is_tomorrow: false,
                }
            )))
        );
        assert!(parser::time_range("90分").is_err());
        assert_eq!(
            parser::time_range("十時"),
            Ok(TimeRangeSpecifier::At(TimeSpecifier::At(
                AtTimeSpecifier::Hour {
                    hour: Hour::from_u8(10).unwrap(),
                    is_tomorrow: false,
                }
            )))
        );
        assert_eq!(
            parser::time_range("1時半"),
            Ok(TimeRangeSpecifier::At(TimeSpecifier::At(
                AtTimeSpecifier::HourMinute {
                    hour: Hour::from_u8(1).unwrap(),
                    minute: Minute::from_u8(30).unwrap(),
                    is_tomorrow: false,
                }
            )))
        );
        assert!(parser::time_range("30時").is_err());
        assert_eq!(
            parser::time_range("明日の一時"),
            Ok(TimeRangeSpecifier::At(TimeSpecifier::At(
                AtTimeSpecifier::Hour {
                    hour: Hour::from_u8(1).unwrap(),
                    is_tomorrow: true
                }
            )))
        );
        assert_eq!(
            parser::time_range("明日の10時15分"),
            Ok(TimeRangeSpecifier::At(TimeSpecifier::At(
                AtTimeSpecifier::HourMinute {
                    hour: Hour::from_u8(10).unwrap(),
                    minute: Minute::from_u8(15).unwrap(),
                    is_tomorrow: true,
                }
            )))
        );
        assert!(parser::time_range("明日の15分").is_err());
    }

    #[test]
    fn test_after_ja() {
        assert_eq!(
            parser::time_range("90分後"),
            Ok(TimeRangeSpecifier::At(TimeSpecifier::After(
                AfterTimeSpecifier::Minute(90)
            )))
        );
        assert_eq!(
            parser::time_range("一時間後"),
            Ok(TimeRangeSpecifier::At(TimeSpecifier::After(
                AfterTimeSpecifier::Hour(1)
            )))
        );
        assert_eq!(
            parser::time_range("1時間30分後"),
            Ok(TimeRangeSpecifier::At(TimeSpecifier::After(
                AfterTimeSpecifier::HourMinute(1, 30)
            )))
        );
        assert_eq!(
            parser::time_range("三秒後"),
            Ok(TimeRangeSpecifier::At(TimeSpecifier::After(
                AfterTimeSpecifier::Second(3)
            )))
        );
    }

    #[test]
    fn test_by_ja() {
        assert_eq!(
            parser::time_range("12:12まで"),
            Ok(TimeRangeSpecifier::By(TimeSpecifier::At(
                AtTimeSpecifier::HourMinute {
                    hour: Hour::from_u8(12).unwrap(),
                    minute: Minute::from_u8(12).unwrap(),
                    is_tomorrow: false
                }
            )))
        );
        assert_eq!(
            parser::time_range("四十五分まで"),
            Ok(TimeRangeSpecifier::By(TimeSpecifier::At(
                AtTimeSpecifier::Minute(Minute::from_u8(45).unwrap())
            )),)
        );
        assert_eq!(
            parser::time_range("十二時まで"),
            Ok(TimeRangeSpecifier::By(TimeSpecifier::At(
                AtTimeSpecifier::Hour {
                    hour: Hour::from_u8(12).unwrap(),
                    is_tomorrow: false
                }
            )))
        );
        assert_eq!(
            parser::time_range("明日の1時まで"),
            Ok(TimeRangeSpecifier::By(TimeSpecifier::At(
                AtTimeSpecifier::Hour {
                    hour: Hour::from_u8(1).unwrap(),
                    is_tomorrow: true
                }
            )))
        );
        assert_eq!(
            parser::time_range("明日の三時二十二分まで"),
            Ok(TimeRangeSpecifier::By(TimeSpecifier::At(
                AtTimeSpecifier::HourMinute {
                    hour: Hour::from_u8(3).unwrap(),
                    minute: Minute::from_u8(22).unwrap(),
                    is_tomorrow: true
                }
            )))
        );
    }

    #[test]
    fn test_within_ja() {
        assert_eq!(
            parser::time_range("九十分後まで"),
            Ok(TimeRangeSpecifier::By(TimeSpecifier::After(
                AfterTimeSpecifier::Minute(90)
            )))
        );
        assert_eq!(
            parser::time_range("11時間後まで"),
            Ok(TimeRangeSpecifier::By(TimeSpecifier::After(
                AfterTimeSpecifier::Hour(11)
            )))
        );
        assert_eq!(
            parser::time_range("90分以内"),
            Ok(TimeRangeSpecifier::By(TimeSpecifier::After(
                AfterTimeSpecifier::Minute(90)
            )))
        );
        assert_eq!(
            parser::time_range("五十秒以内"),
            Ok(TimeRangeSpecifier::By(TimeSpecifier::After(
                AfterTimeSpecifier::Second(50)
            )))
        );
    }

    #[test]
    fn test_at_en() {
        assert_eq!(
            parser::time_range("at 12:00"),
            Ok(TimeRangeSpecifier::At(TimeSpecifier::At(
                AtTimeSpecifier::HourMinute {
                    hour: Hour::from_u8(12).unwrap(),
                    minute: Minute::from_u8(00).unwrap(),
                    is_tomorrow: false
                }
            )))
        );
        assert!(parser::time_range("at 30:00").is_err());
        assert_eq!(
            parser::time_range("at 10:15 tomorrow"),
            Ok(TimeRangeSpecifier::At(TimeSpecifier::At(
                AtTimeSpecifier::HourMinute {
                    hour: Hour::from_u8(10).unwrap(),
                    minute: Minute::from_u8(15).unwrap(),
                    is_tomorrow: true
                }
            )))
        );
    }

    #[test]
    fn test_after_en() {
        assert_eq!(
            parser::time_range("after 90min"),
            Ok(TimeRangeSpecifier::At(TimeSpecifier::After(
                AfterTimeSpecifier::Minute(90)
            )))
        );
        assert_eq!(
            parser::time_range("after 1h"),
            Ok(TimeRangeSpecifier::At(TimeSpecifier::After(
                AfterTimeSpecifier::Hour(1)
            )))
        );
        assert_eq!(
            parser::time_range("after 1h30m"),
            Ok(TimeRangeSpecifier::At(TimeSpecifier::After(
                AfterTimeSpecifier::HourMinute(1, 30)
            )))
        );
        assert_eq!(
            parser::time_range("after 2s"),
            Ok(TimeRangeSpecifier::At(TimeSpecifier::After(
                AfterTimeSpecifier::Second(2)
            )))
        );
        assert_eq!(
            parser::time_range("after 2 seconds"),
            Ok(TimeRangeSpecifier::At(TimeSpecifier::After(
                AfterTimeSpecifier::Second(2)
            )))
        );
    }

    #[test]
    fn test_by_en() {
        assert_eq!(
            parser::time_range("by 12:12"),
            Ok(TimeRangeSpecifier::By(TimeSpecifier::At(
                AtTimeSpecifier::HourMinute {
                    hour: Hour::from_u8(12).unwrap(),
                    minute: Minute::from_u8(12).unwrap(),
                    is_tomorrow: false
                }
            )))
        );
        assert_eq!(
            parser::time_range("by 23:25 tomorrow"),
            Ok(TimeRangeSpecifier::By(TimeSpecifier::At(
                AtTimeSpecifier::HourMinute {
                    hour: Hour::from_u8(23).unwrap(),
                    minute: Minute::from_u8(25).unwrap(),
                    is_tomorrow: true
                }
            )))
        );
    }

    #[test]
    fn test_within_en() {
        assert_eq!(
            parser::time_range("within 90m"),
            Ok(TimeRangeSpecifier::By(TimeSpecifier::After(
                AfterTimeSpecifier::Minute(90)
            )))
        );
        assert_eq!(
            parser::time_range("within 11hr"),
            Ok(TimeRangeSpecifier::By(TimeSpecifier::After(
                AfterTimeSpecifier::Hour(11)
            )))
        );
        assert_eq!(
            parser::time_range("within 90 minute"),
            Ok(TimeRangeSpecifier::By(TimeSpecifier::After(
                AfterTimeSpecifier::Minute(90)
            )))
        );
        assert_eq!(
            parser::time_range("within 30sec"),
            Ok(TimeRangeSpecifier::By(TimeSpecifier::After(
                AfterTimeSpecifier::Second(30)
            )))
        );
    }
}
