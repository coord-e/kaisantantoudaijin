use std::fmt::{self, Display};

use crate::error::Error;
use crate::model::kaisanee::KaisaneeSpecifier;

use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use serenity::model::{id::UserId, misc::Mentionable};

#[derive(Debug)]
pub enum Message {
    Help,
    ScheduledAt(KaisaneeSpecifier, DateTime<Tz>),
    ScheduledBy(KaisaneeSpecifier, DateTime<Tz>),
    Kaisan(Vec<UserId>),
    Setting {
        requires_permission: bool,
        timezone: Tz,
    },
    HandleError(Error),
    KaisanError(Error),
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
            Message::ScheduledAt(kaisanee, time) => {
                let now = Utc::now().with_timezone(&time.timezone());
                fmt_datetime_when(f, *time, now)?;
                write!(f, "に{}を解散します", kaisanee)
            }
            Message::ScheduledBy(kaisanee, time) => {
                let now = Utc::now().with_timezone(&time.timezone());
                fmt_datetime_when(f, *time, now)?;
                write!(f, "までに{}を解散します", kaisanee)
            }
            Message::Kaisan(ids) => {
                for id in ids {
                    write!(f, "{} ", id.mention())?;
                }
                f.write_str("解散！")
            }
            Message::Setting {
                requires_permission,
                timezone,
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
                Ok(())
            }
            Message::HandleError(e) => fmt_error(f, e),
            Message::KaisanError(e) => {
                f.write_str("解散できませんでした: ")?;
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
        _ => f.write_str("ダメそう"),
    }
}

fn fmt_datetime_when<T: TimeZone>(
    f: &mut fmt::Formatter,
    time: DateTime<T>,
    now: DateTime<T>,
) -> fmt::Result {
    if time.date() != now.date() {
        write!(f, "{}/{} ", time.date().month(), time.date().day())?;
    }
    if time.hour() != now.hour() {
        write!(f, "{}時", time.hour())?;
    }
    write!(f, "{}分（", time.minute())?;

    let duration = time - now;
    if duration.num_hours() != 0 {
        write!(f, "{}時間", duration.num_hours())?;
    }
    if duration.num_minutes() != 0 || (duration.num_hours() == 0 && duration.num_days() == 0) {
        write!(f, "{}分", duration.num_minutes() % 60)?;
    }
    write!(f, "後）")
}
