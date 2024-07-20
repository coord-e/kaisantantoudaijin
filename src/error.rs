use std::sync::Arc;

use crate::model::{command::ParseCommandError, reminder::Reminder, time::TimeSpecifier};
use crate::say::{fmt, Say};

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serenity::model::permissions::Permissions;
use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error("could not access to the target guild")]
    InaccessibleGuild,
    #[error("the user is not in voice channel")]
    NotInVoiceChannel,
    #[error("unable to parse command")]
    InvalidCommand(#[from] ParseCommandError),
    #[error("you don't have {0} permission")]
    InsufficientPermission(Permissions),
    #[error("unreachable time {specified} has specified at {at}")]
    UnreachableTime {
        specified: DateTime<Utc>,
        at: DateTime<Utc>,
    },
    #[error("invalid time {specifier:?} at {at} in {timezone}")]
    InvalidTime {
        specifier: TimeSpecifier,
        at: DateTime<Utc>,
        timezone: Tz,
    },
    #[error("no such reminder for {}", .0.before_duration())]
    NoSuchReminder(Reminder),
    #[error("reminder for {} already exists", .0.before_duration())]
    DuplicatedReminders(Reminder),
    #[error(transparent)]
    Other(Arc<anyhow::Error>),
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Error {
        Error::Other(Arc::new(err))
    }
}

impl Say for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NotInVoiceChannel => f.write_str("ボイスチャンネルに入った状態で使ってほしい"),
            Error::InvalidCommand(_) => f.write_str("コマンドがわからない"),
            Error::UnreachableTime { .. } => f.write_str("過去を変えることはできない"),
            Error::InvalidTime { .. } => f.write_str("そんな時刻はない"),
            Error::InsufficientPermission(p) => write!(f, "{} の権限が必要です", p),
            Error::NoSuchReminder(_) => f.write_str("そんなリマインダはない"),
            Error::DuplicatedReminders(_) => f.write_str("それはすでにある"),
            _ => f.write_str("ダメそう"),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
