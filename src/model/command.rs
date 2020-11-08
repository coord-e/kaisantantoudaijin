use std::error::Error;
use std::fmt::{self, Display};
use std::str::FromStr;

use chrono::DateTime;
use chrono_tz::Tz;
use serenity::model::id::UserId;

use crate::model::{
    kaisanee::KaisaneeSpecifier,
    time::{Hour, HourMinuteSpecifier, Minute, TimeSpecifier},
};

#[derive(Debug, Clone, Copy)]
pub enum TimeRangeSpecifier {
    By(TimeSpecifier),
    At(TimeSpecifier),
}

#[derive(Debug, Clone)]
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
          [':'] m:minute() _ { HourMinuteSpecifier::Both(h, m) }
          / _ ['時'] _ m:spec_minute()? { HourMinuteSpecifier::with_hour(h, m) }
      ) { TimeSpecifier::At { time: s, is_tomorrow: true } }

    rule spec_at_rfc3339() -> TimeSpecifier
      = "rfc3339" _ t:$(['T' | 'Z' | '+' | '-' | '.' | ':' | '0'..='9']+) _ {?
          match DateTime::parse_from_rfc3339(t) {
              Ok(t) => Ok(TimeSpecifier::Exactly(t)),
              Err(_) => Err("rfc3339 time"),
          }
      }

    rule spec_at_tail(x: u8) -> TimeSpecifier
      = [':'] m:minute() _ t:("tomorrow" _)? {?
          Hour::from_u8(x).map(|h| {
              TimeSpecifier::At {
                  time: HourMinuteSpecifier::Both(h, m),
                  is_tomorrow: t.is_some(),
              }
          }).map_err(|_| "hour")
      }
      / _ ['分'] _ {?
          Minute::from_u8(x).map(|m| {
              TimeSpecifier::At {
                  time: HourMinuteSpecifier::with_minute(m, None),
                  is_tomorrow: false
              }
          }).map_err(|_| "minute")
      }
      / _ ['時'] _ m:spec_minute()? {?
          Hour::from_u8(x).map(|h| {
              TimeSpecifier::At {
                  time: HourMinuteSpecifier::with_hour(h, m),
                  is_tomorrow: false
              }
          }).map_err(|_| "hour")
      }

    rule spec_at_half() -> TimeSpecifier
      = ['半'] _ { TimeSpecifier::At { time: HourMinuteSpecifier::Minute(Minute::from_u8(30).unwrap()), is_tomorrow: false } }

    rule spec_at() -> TimeSpecifier
      = x:number() spec:spec_at_tail(x) { spec }
      / spec_at_tomorrow()
      / spec_at_rfc3339()
      / spec_at_half()

    rule spec_after() -> TimeSpecifier
      = x:number() _ s:(
          minute_suffix() _ { HourMinuteSpecifier::with_minute(x, None) }
          / hour_suffix() _ m:(m:number() _ minute_suffix() _ { m })? { HourMinuteSpecifier::with_hour(x, m) }
      ) { TimeSpecifier::After(s) }

    pub rule time_range() -> TimeRangeSpecifier
      = x:number() spec:(
          _ minute_suffix() _ s:$("後まで" / ['後'] / "以内") {
              let spec = TimeSpecifier::After(HourMinuteSpecifier::with_minute(x, None));
              match s {
                  "以内" | "後まで" => TimeRangeSpecifier::By(spec),
                  "後" => TimeRangeSpecifier::At(spec),
                  _ => unreachable!(),
              }
          }
          / _ hour_suffix() _ m:(m:number() _ minute_suffix() _ { m })? s:$("後まで" / ['後'] / "以内") {
              let spec = TimeSpecifier::After(HourMinuteSpecifier::with_hour(x, m));
              match s {
                  "以内" | "後まで" => TimeRangeSpecifier::By(spec),
                  "後" => TimeRangeSpecifier::At(spec),
                  _ => unreachable!(),
              }
          }
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
