use std::sync::Arc;

use crate::model::{command::ParseCommandError, reminder::Reminder};

use chrono::{DateTime, Utc};
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

pub type Result<T> = std::result::Result<T, Error>;
