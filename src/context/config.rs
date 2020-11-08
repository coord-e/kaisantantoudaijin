use chrono_tz::Tz;

pub trait ConfigContext {
    fn timezone(&self) -> Tz;
}
