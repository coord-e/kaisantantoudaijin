use std::fmt::{self, Display};
use std::str::FromStr;

use chrono::{DateTime, FixedOffset};
use serenity::model::id::UserId;

#[derive(Debug, Clone)]
pub enum KaisaneeSpecifier {
    Me,
    All,
    Users(Vec<UserId>),
}

impl Default for KaisaneeSpecifier {
    fn default() -> Self {
        KaisaneeSpecifier::All
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy)]
pub struct Hour(u8);

impl Hour {
    // implemented to use in parser
    fn from_u8(x: u8) -> Result<Hour, &'static str> {
        if x < 24 {
            Ok(Hour(x))
        } else {
            Err("invalid hour")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy)]
pub struct Minute(u8);

impl Minute {
    // implemented to use in parser
    fn from_u8(x: u8) -> Result<Minute, &'static str> {
        if x < 60 {
            Ok(Minute(x))
        } else {
            Err("invalid minute")
        }
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
    rule _() = [' ']*

    rule me()
      = "me" / "Me"
      / "私" / "わたし"
      / "俺" / "おれ" / "オレ"
      / "僕" / "ぼく" / "ボク"

    rule all()
      = "all" / "All" / "全員" / "皆" / "みんな"

    rule user() -> UserId
      = "<@!" n:$(['0'..='9']+) ">" { UserId(n.parse().unwrap()) }

    rule users() -> Vec<UserId>
      = l:user() ** _ {? if l.is_empty() { Err("invalid empty users") } else { Ok(l) } }

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
              Err("invalid kanji number")
          }
      }

    rule kanji_number() -> u8
      = kanji_number_tail(1)
      / x:kanji_number_digit() y:kanji_number_tail(x)? { y.unwrap_or(x) }

    rule number() -> u8
      = x:$(['0'..='9']*<1,3>) {? x.parse().map_err(|_| "0~255") }
      / kanji_number()

    rule minute() -> Minute
      = t:$(['0'..='9']*<1,2>) {? Minute::from_u8(t.parse().unwrap()) }
      / n:kanji_number() {? Minute::from_u8(n) }

    rule hour() -> Hour
      = t:$(['0'..='9']*<1,2>) {? Hour::from_u8(t.parse().unwrap()) }
      / n:kanji_number() {? Hour::from_u8(n) }

    rule spec_minute() -> Minute
      = ['半'] _ { Minute(30) }
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
              Err(_) => Err("invalid rfc3339 time"),
          }
      }

    rule spec_at_tail(x: u8) -> TimeSpecifier
      = [':'] m:minute() _ t:("tomorrow" _)? {?
          Hour::from_u8(x).map(|h| {
              TimeSpecifier::At {
                  time: HourMinuteSpecifier::Both(h, m),
                  is_tomorrow: t.is_some(),
              }
          })
      }
      / _ ['分'] _ {?
          Minute::from_u8(x).map(|m| {
              TimeSpecifier::At {
                  time: HourMinuteSpecifier::with_minute(m, None),
                  is_tomorrow: false
              }
          })
      }
      / _ ['時'] _ m:spec_minute()? {?
          Hour::from_u8(x).map(|h| {
              TimeSpecifier::At {
                  time: HourMinuteSpecifier::with_hour(h, m),
                  is_tomorrow: false
              }
          })
      }

    rule spec_at_half() -> TimeSpecifier
      = ['半'] _ { TimeSpecifier::At { time: HourMinuteSpecifier::Minute(Minute(30)), is_tomorrow: false } }

    rule spec_at() -> TimeSpecifier
      = x:number() spec:spec_at_tail(x) { spec }
      / spec_at_tomorrow()
      / spec_at_rfc3339()
      / spec_at_half()

    rule spec_after() -> TimeSpecifier
      = x:number() s:(
          minute_suffix() _ { HourMinuteSpecifier::with_minute(x, None) }
          / hour_suffix() _ m:(m:number() _ minute_suffix() _ { m })? { HourMinuteSpecifier::with_hour(x, m) }
      ) { TimeSpecifier::After(s) }

    pub rule time_range() -> TimeRangeSpecifier
      = x:number() spec:(
          _ minute_suffix() _ s1:$(['後'] / "以内") s2:"まで"? {?
              let spec = TimeSpecifier::After(HourMinuteSpecifier::with_minute(x, None));
              match (s1, s2.is_some()) {
                  ("以内", false) | ("後", true) => Ok(TimeRangeSpecifier::By(spec)),
                  ("後", false) => Ok(TimeRangeSpecifier::At(spec)),
                  _ => Err("invalid range spec"),
              }
          }
          / _ hour_suffix() _ m:(m:number() _ minute_suffix() _ { m })? s1:$(['後'] / "以内") s2:"まで"? {?
              let spec = TimeSpecifier::After(HourMinuteSpecifier::with_hour(x, m));
              match (s1, s2.is_some()) {
                  ("以内", false) | ("後", true) => Ok(TimeRangeSpecifier::By(spec)),
                  ("後", false) => Ok(TimeRangeSpecifier::At(spec)),
                  _ => Err("invalid range spec"),
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
      / kaisanee1:spec_kaisanee()? time_range:time_range() _ (['に'] _)? kaisanee2:spec_kaisanee()? "解散"? {?
          match (kaisanee1, kaisanee2) {
              (Some(kaisanee), None) | (None, Some(kaisanee)) => Ok(Command::Kaisan { kaisanee, time_range }),
              (None, None) => Ok(Command::Kaisan { kaisanee: KaisaneeSpecifier::default(), time_range }),
              (Some(_), Some(_)) => Err("kaisanee specified twice"),
          }
      }
  }
}

// #[cfg(test)]
// mod tests {
//     use super::{parser, Command};
// }
