use std::collections::HashSet;
use std::fmt::{self, Display};

use crate::error::Error;
use crate::model::{
    command::TimeRangeSpecifier, kaisanee::KaisaneeSpecifier, reminder::Reminder,
    time::TimeSpecifier,
};

use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike};
use chrono_tz::Tz;
use serenity::model::{id::UserId, misc::Mentionable};

#[derive(Debug)]
pub enum Message {
    Help,
    Scheduled {
        spec: TimeRangeSpecifier,
        kaisanee: KaisaneeSpecifier,
        time: DateTime<Tz>,
        now: DateTime<Tz>,
    },
    Kaisan(Vec<UserId>),
    Remind(Vec<UserId>, Reminder),
    Setting {
        requires_permission: bool,
        timezone: Tz,
        reminders: HashSet<Reminder>,
    },
    HandleError(Error),
    KaisanError(Error),
    RemindError(Error),
}

impl Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Message::Help => f.write_str(
                "メンションか `!kaisan` でコマンドが実行できます。

・`!kaisan help`: ヘルプ

**解散コマンド** 省略された場合、`TARGET` は全員になります
・`!kaisan [TARGET] at TIME`: `TARGET` を `TIME` に解散する
・`!kaisan [TARGET] after DURATION`: `TARGET` を `DURATION` 後に解散する
・`!kaisan [TARGET] by TIME`: `TARGET` を `TIME` までのランダムな時間に解散する
・`!kaisan [TARGET] within DURATION`: `TARGET` を `DURATION` 後までのランダムな時間に解散する
・その他さまざまな糖衣構文

*解散コマンド例*
・`@解散担当大臣 1時間30分後`
・`!kaisan me after 10min`
・`明日の一時 @解散担当大臣`
・`!kaisan @someone at 10:30`

**設定コマンド** 設定には Manage Guild 権限が必要です
・`!kaisan show-setting`: 設定表示
・`!kaisan timezone TIMEZONE`: タイムゾーンを設定
・`!kaisan require-permission BOOLEAN`: 他人を解散するのに Move Members 権限を必要とするか設定
",
            ),
            Message::Scheduled {
                spec: TimeRangeSpecifier::Now,
                kaisanee,
                ..
            } => write!(f, "今すぐに{}を解散します", kaisanee),
            Message::Scheduled {
                spec: TimeRangeSpecifier::At(spec),
                kaisanee,
                time,
                now,
            } => {
                fmt_datetime_when(f, *spec, *time, *now)?;
                write!(f, "に{}を解散します", kaisanee)
            }
            Message::Scheduled {
                spec: TimeRangeSpecifier::By(spec),
                kaisanee,
                time,
                now,
            } => {
                fmt_datetime_when(f, *spec, *time, *now)?;
                write!(f, "までに{}を解散します", kaisanee)
            }
            Message::Kaisan(ids) => {
                for id in ids {
                    write!(f, "{} ", id.mention())?;
                }
                f.write_str("解散！")
            }
            Message::Remind(ids, reminder) => {
                for id in ids {
                    write!(f, "{} ", id.mention())?;
                }
                write!(f, "あと")?;
                fmt_duration(f, reminder.before_duration())?;
                write!(f, "で解散です")
            }
            Message::Setting {
                requires_permission,
                timezone,
                reminders,
            } => {
                writeln!(
                    f,
                    "他人を解散させるのに権限を必要とする: {}",
                    if *requires_permission {
                        "はい"
                    } else {
                        "いいえ"
                    }
                )?;
                writeln!(f, "タイムゾーン: {}", timezone.name())?;

                f.write_str("リマインダ: ")?;
                let mut reminders = reminders.iter();
                if let Some(head) = reminders.next() {
                    fmt_duration(f, head.before_duration())?;
                    write!(f, "前")?;
                    for m in reminders {
                        write!(f, "、")?;
                        fmt_duration(f, m.before_duration())?;
                        write!(f, "前")?;
                    }
                } else {
                    f.write_str("設定されていません")?;
                }

                Ok(())
            }
            Message::HandleError(e) => fmt_error(f, e),
            Message::KaisanError(e) => {
                f.write_str("解散できませんでした: ")?;
                fmt_error(f, e)
            }
            Message::RemindError(e) => {
                f.write_str("リマインドできませんでした: ")?;
                fmt_error(f, e)
            }
        }
    }
}

fn fmt_error(f: &mut fmt::Formatter, e: &Error) -> fmt::Result {
    match e {
        Error::NotInVoiceChannel => f.write_str("ボイスチャンネルに入った状態で使ってほしい"),
        Error::InvalidCommand(_) => f.write_str("コマンドがわからない"),
        Error::UnreachableTime { .. } => f.write_str("過去を変えることはできない"),
        Error::InsufficientPermission(p) => write!(f, "{} の権限が必要です", p),
        Error::NoSuchReminder(_) => f.write_str("そんなリマインダはない"),
        Error::DuplicatedReminders(_) => f.write_str("それはすでにある"),
        _ => f.write_str("ダメそう"),
    }
}

fn fmt_datetime_when<T: TimeZone>(
    f: &mut fmt::Formatter,
    spec: TimeSpecifier,
    time: DateTime<T>,
    now: DateTime<T>,
) -> fmt::Result {
    if spec.is_interested_in_time() {
        if time.date() != now.date() {
            write!(f, "{}/{} ", time.date().month(), time.date().day())?;
        }
        if time.hour() != now.hour() {
            write!(f, "{}時", time.hour())?;
            if time.minute() != 0 {
                write!(f, "{}分", time.minute())?;
            }
        } else {
            write!(f, "{}分", time.minute())?;
        }
    }

    if spec.is_interested_in_time() && spec.is_interested_in_duration() {
        write!(f, "、")?;
    }

    if spec.is_interested_in_duration() {
        fmt_duration(f, time - now)?;
        write!(f, "後")?;
    }

    Ok(())
}

fn fmt_duration(f: &mut fmt::Formatter, duration: Duration) -> fmt::Result {
    if duration.num_hours() != 0 {
        write!(f, "{}時間", duration.num_hours())?;
    }
    if duration.num_minutes() != 0 || duration.num_hours() == 0 {
        write!(f, "{}分", duration.num_minutes() % 60)?;
    }
    Ok(())
}
