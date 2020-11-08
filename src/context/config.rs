use chrono::FixedOffset;

pub trait ConfigContext {
    fn timezone(&self) -> FixedOffset;
}
