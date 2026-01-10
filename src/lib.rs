macro_rules! say {
    ($dst:expr, $fmt:literal, $($arg:expr),*) => { write!($dst, $fmt, $( crate::say::SayExt::display_say($arg) ),*) }
}

macro_rules! sayln {
    ($dst:expr, $fmt:literal, $($arg:expr),*) => { writeln!($dst, $fmt, $( crate::say::SayExt::display_say($arg) ),*) }
}

pub mod context;
pub mod database;
pub mod error;
pub mod model;
pub mod say;
pub mod use_case;

#[cfg(test)]
mod test;
