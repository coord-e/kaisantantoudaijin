use serenity::model::id::UserId;

pub trait BotContext {
    fn bot_id(&self) -> UserId;
}
