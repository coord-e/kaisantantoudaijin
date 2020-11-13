use crate::say::{fmt, IntoIteratorSayExt, Say};

use serenity::model::id::UserId;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum KaisaneeSpecifier {
    Me,
    All,
    Users(Vec<UserId>),
}

impl KaisaneeSpecifier {
    pub fn may_include_others(&self, user_id: UserId) -> bool {
        match self {
            KaisaneeSpecifier::Me => false,
            KaisaneeSpecifier::All => true,
            KaisaneeSpecifier::Users(users) => users != &[user_id],
        }
    }
}

impl Default for KaisaneeSpecifier {
    fn default() -> Self {
        KaisaneeSpecifier::All
    }
}

impl Say for KaisaneeSpecifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            KaisaneeSpecifier::Me => f.write_str("あなた"),
            KaisaneeSpecifier::All => f.write_str("全員"),
            KaisaneeSpecifier::Users(ids) => ids.say_mentions_ref().fmt(f),
        }
    }
}
