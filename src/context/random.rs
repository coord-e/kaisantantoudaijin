#[async_trait::async_trait]
pub trait RandomContext {
    async fn random_range(&self, from: i64, to: i64) -> i64;
}
