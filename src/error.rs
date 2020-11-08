use crate::model::command::ParseCommandError;

use chrono::{DateTime, Utc};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("could not access to the target guild")]
    InaccessibleGuild,
    #[error("the user is not in voice channel")]
    NotInVoiceChannel,
    #[error("unable to parse command")]
    InvalidCommand(#[from] ParseCommandError),
    #[error("unreachable time {specified} has specified at {at}")]
    UnreachableTime {
        specified: DateTime<Utc>,
        at: DateTime<Utc>,
    },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;