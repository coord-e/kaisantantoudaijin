use std::collections::HashSet;

use crate::error::Error;
use crate::model::{kaisanee::KaisaneeSpecifier, reminder::Reminder, time::TimeSpecifier};
use crate::say::{fmt, IntoIteratorSayExt, Say};

use chrono::{DateTime, Datelike, Timelike};
use chrono_tz::Tz;
use serenity::model::id::UserId;

#[derive(Clone, Debug)]
pub enum Message {
    Help,
    Scheduled {
        calculated_time: CalculatedDateTime,
        kaisanee: KaisaneeSpecifier,
    },
    Kaisan(Vec<UserId>),
    Remind(Vec<UserId>, Reminder),
    Setting {
        requires_permission: bool,
        timezone: Tz,
        reminders: HashSet<Reminder>,
        reminds_random_kaisan: bool,
    },
    HandleError(Error),
    KaisanError(Error),
    RemindError(Error),
}

const HELP_MESSAGE: &str = "メンションか `!kaisan` でコマンドが実行できます。

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
・`!kaisan add-reminder N`: 解散の `N` 分前にリマインドを設定
・`!kaisan remove-reminder N`: 解散の `N` 分前のリマインドを削除
・`!kaisan remind-random BOOLEAN`: 解散時刻がランダムな場合にもリマインダを使うかどうか設定
";

impl Say for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Message::Help => f.write_str(HELP_MESSAGE),
            Message::Scheduled {
                calculated_time,
                kaisanee,
            } => say!(f, "{}に{}を解散します", calculated_time, kaisanee),
            Message::Kaisan(ids) => say!(f, "{} 解散！", ids.say_mentions_ref()),
            Message::Remind(ids, reminder) => say!(
                f,
                "{} あと{}で解散です",
                ids.say_mentions_ref(),
                reminder.before_duration()
            ),
            Message::Setting {
                requires_permission,
                timezone,
                reminders,
                reminds_random_kaisan,
            } => {
                sayln!(
                    f,
                    "他人を解散させるのに権限を必要とする: {}",
                    requires_permission
                )?;
                sayln!(f, "タイムゾーン: {}", timezone)?;
                sayln!(
                    f,
                    "リマインダ: {}",
                    reminders
                        .say_joined("、")
                        .with_alternative("設定されていません")
                )?;
                sayln!(
                    f,
                    "解散時刻がランダムな場合にもリマインダを使う: {}",
                    reminds_random_kaisan
                )?;

                Ok(())
            }
            Message::HandleError(e) => Say::fmt(e, f),
            Message::KaisanError(e) => say!(f, "解散できませんでした: {}", e),
            Message::RemindError(e) => say!(f, "リマインドできませんでした: {}", e),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CalculatedDateTime {
    pub time: DateTime<Tz>,
    pub now: DateTime<Tz>,
    pub spec: TimeSpecifier,
    pub is_random: bool,
}

impl Say for CalculatedDateTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let CalculatedDateTime {
            spec,
            time,
            now,
            is_random,
        } = *self;

        if spec.is_interested_in_time() {
            if time.date_naive() != now.date_naive() {
                write!(
                    f,
                    "{}/{} ",
                    time.date_naive().month(),
                    time.date_naive().day()
                )?;
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
            f.write_str("、")?;
        }

        if spec.is_interested_in_duration() {
            say!(f, "{}後", time - now)?;
        }

        if is_random {
            f.write_str("まで")?;
        }

        Ok(())
    }
}
