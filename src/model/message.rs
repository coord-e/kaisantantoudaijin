use std::fmt::{self, Display};

use crate::error::Error;
use crate::model::kaisanee::KaisaneeSpecifier;

use chrono::{DateTime, FixedOffset};
use serenity::model::{id::UserId, misc::Mentionable};

pub enum Message {
    Help,
    ScheduledAt(KaisaneeSpecifier, DateTime<FixedOffset>),
    ScheduledBy(KaisaneeSpecifier, DateTime<FixedOffset>),
    Kaisan(Vec<UserId>),
    HandleError(Error),
    KaisanError(Error),
}

impl Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Message::Help => f.write_str(
                "メンションか `!kaisan` でコマンドが実行できます。

**コマンド例**
- `@解散担当大臣 1時間30分後`
- `!kaisan me after 10min`
- `明日の一時 @解散担当大臣`
- `!kaisan @someone at 10:30`
",
            ),
            Message::ScheduledAt(kaisanee, time) => {
                // let now = Utc::now().with_timezone(&time.timezone());
                write!(f, "{}に{:?}を解散します", time, kaisanee)
            }
            Message::ScheduledBy(kaisanee, time) => {
                // let now = Utc::now().with_timezone(&time.timezone());
                write!(f, "{}までに{:?}を解散します", time, kaisanee)
            }
            Message::Kaisan(ids) => {
                for id in ids {
                    write!(f, "{} ", id.mention())?;
                }
                f.write_str("解散！")
            }
            Message::HandleError(e) => e.fmt(f),
            Message::KaisanError(e) => write!(f, "解散できませんでした: {}", e),
        }
    }
}
