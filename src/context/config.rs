use chrono_tz::Tz;

pub trait ConfigContext {
    fn timezone(&self) -> Tz;
    fn requires_permission(&self) -> bool;
}
