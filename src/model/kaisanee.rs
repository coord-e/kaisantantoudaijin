use serenity::model::id::UserId;

#[derive(Debug, Clone)]
pub enum KaisaneeSpecifier {
    Me,
    All,
    Users(Vec<UserId>),
}

impl Default for KaisaneeSpecifier {
    fn default() -> Self {
        KaisaneeSpecifier::All
    }
}
