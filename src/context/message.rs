use crate::error::Result;

use serenity::model::{channel::ReactionType, id::UserId};

#[async_trait::async_trait]
pub trait MessageContext {
    fn author_id(&self) -> UserId;
    async fn react(&self, reaction: impl Into<ReactionType> + 'async_trait + Send) -> Result<()>;
}
