pub use std::fmt;
use std::fmt::Display;
use std::marker::PhantomData;
use std::sync::Arc;

use chrono::Duration;
use chrono_tz::Tz;
use serenity::model::mention::Mentionable;

pub trait Say {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result;
}

impl<T: Say + ?Sized> Say for &T {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        T::fmt(self, f)
    }
}

impl<T: Say + ?Sized> Say for &mut T {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        T::fmt(self, f)
    }
}

impl<T: Say + ?Sized> Say for Box<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        T::fmt(self, f)
    }
}

impl<T: Say + ?Sized> Say for Arc<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        T::fmt(self, f)
    }
}

impl Say for String {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self)
    }
}

impl Say for str {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self)
    }
}

impl Say for Duration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.num_hours() != 0 {
            write!(f, "{}時間", self.num_hours())?;
        }
        if self.num_minutes() != 0 || self.num_hours() == 0 {
            write!(f, "{}分", self.num_minutes() % 60)?;
        }
        Ok(())
    }
}

impl Say for Tz {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl Say for bool {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if *self {
            f.write_str("はい")
        } else {
            f.write_str("いいえ")
        }
    }
}

pub trait SayExt: Sized {
    fn display_say(self) -> DisplaySay<Self> {
        DisplaySay(self)
    }
}

impl<T: Say> SayExt for T {}

pub struct DisplaySay<T>(T);

impl<T: Say> Display for DisplaySay<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Say::fmt(&self.0, f)
    }
}

impl<T: Say> Say for DisplaySay<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Say::fmt(&self.0, f)
    }
}

pub struct SayDisplay<T>(T);

impl<T: Display> Say for SayDisplay<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl<T: Display> Display for SayDisplay<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

pub trait DisplayExt: Sized {
    fn say_display(self) -> SayDisplay<Self> {
        SayDisplay(self)
    }
}

impl<T: Display> DisplayExt for T {}

pub struct SayJoined<'a, 'b, T, U> {
    iter: T,
    separator: &'a str,
    alternative: Option<&'b str>,
    _marker: PhantomData<U>,
}

impl<'a, T, U> SayJoined<'a, '_, T, U> {
    pub fn with_alternative<'b>(self, alternative: &'b str) -> SayJoined<'a, 'b, T, U> {
        SayJoined {
            alternative: Some(alternative),
            iter: self.iter,
            separator: self.separator,
            _marker: PhantomData,
        }
    }
}

impl<T, U> Say for SayJoined<'_, '_, T, U>
where
    T: Iterator<Item = U> + Clone,
    U: Say,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut iter = self.iter.clone();
        if let Some(head) = iter.next() {
            Say::fmt(&head, f)?;
            for x in iter {
                f.write_str(self.separator)?;
                Say::fmt(&x, f)?;
            }
        } else if let Some(alt) = self.alternative {
            f.write_str(alt)?;
        }
        Ok(())
    }
}

type SayMentionsRef<'a, 'b, T, U> =
    SayJoined<'static, 'b, std::iter::Map<T, fn(&'a U) -> String>, String>;

pub trait IntoIteratorSayExt: IntoIterator + Sized {
    fn say_mentions_ref<'a, 'b, T>(self) -> SayMentionsRef<'a, 'b, Self::IntoIter, T>
    where
        Self: IntoIterator<Item = &'a T>,
        T: Mentionable + 'a,
    {
        fn f<T>(x: &T) -> String
        where
            T: Mentionable,
        {
            x.mention().to_string()
        }
        self.into_iter().map(f as fn(&'a T) -> String).say_unwords()
    }

    fn say_unwords<'b>(self) -> SayJoined<'static, 'b, Self::IntoIter, Self::Item> {
        self.say_joined(" ")
    }

    fn say_joined<'a, 'b>(
        self,
        separator: &'a str,
    ) -> SayJoined<'a, 'b, Self::IntoIter, Self::Item> {
        SayJoined {
            iter: self.into_iter(),
            separator,
            alternative: None,
            _marker: PhantomData,
        }
    }
}

impl<I: IntoIterator> IntoIteratorSayExt for I {}
