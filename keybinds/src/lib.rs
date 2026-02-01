pub mod parse_config;

use color_eyre::Result;
use crossterm::event::{EventStream, KeyCode};
use futures_util::Stream;
use std::{
    collections::BTreeMap,
    fmt::{Debug, Display},
    pin::Pin,
    sync::Arc,
    task::Poll,
};

pub use futures_util::StreamExt;

///reexport for proc macro
#[doc(hidden)]
pub use color_eyre::eyre as __eyre;

pub use keybinds_derive::{Command, keybind_config};

pub trait Command: Clone + Copy + Debug {
    fn to_name(self) -> &'static str;
    fn from_name(name: &str) -> Option<Self>;
    fn all() -> &'static [&'static str];
}

#[derive(PartialEq, Eq, Clone)]
pub struct Key {
    pub inner: KeyCode,
    pub control: bool,
    pub alt: bool,
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn score_key(k: &Key) -> u8 {
    let mut v = 0;
    if k.control {
        v += 2;
    }
    if k.alt {
        v += 1;
    }
    v
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.inner
            .to_string()
            .cmp(&other.inner.to_string())
            .then_with(|| score_key(self).cmp(&score_key(other)))
    }
}

impl Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}
impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.control {
            f.write_str("C-")?;
        }
        if self.alt {
            f.write_str("A")?;
        }
        Display::fmt(&self.inner, f)
    }
}

pub type BindingMap<T> = Arc<BTreeMap<Key, KeyBinding<T>>>;

#[derive(Debug, Clone)]
pub enum KeyBinding<T: Command> {
    Command(T),
    Group { map: BindingMap<T>, name: String },
    Invalid(String),
}

#[derive(Debug, Clone)]
pub enum Text {
    Char(char),
    Str(String),
}

#[derive(Debug, Clone)]
pub enum KeybindEvent<T: Command> {
    Render,
    Command(T),
    Text(Text),
}

pub struct KeybindEvents {
    events: EventStream,
    finished: bool,
}

impl KeybindEvents {
    pub fn new() -> Result<Self> {
        Ok(Self {
            events: EventStream::new(),
            finished: false,
        })
    }
}

impl Stream for KeybindEvents {
    type Item = std::result::Result<crossterm::event::Event, std::io::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.finished {
            Poll::Ready(None)
        } else {
            match Pin::new(&mut this.events).poll_next(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(None) => {
                    this.finished = true;
                    Poll::Ready(None)
                }
                Poll::Ready(Some(v)) => Poll::Ready(Some(v)),
            }
        }
    }
}

#[doc(hidden)]
pub mod __macro_support {
    use crate::{BindingMap, Command};

    pub fn collect_all_names(names: &[&[&'static str]]) -> &'static [&'static str] {
        let mut out: Vec<_> = names
            .iter()
            .flat_map(|s| s.iter().map(|s| s as &'static str))
            .collect();
        out.sort_unstable();
        out.leak()
    }
    pub fn commands_unique(names: &[&str], ty: &str) {
        let mut iter = names.iter();
        if let Some(mut last) = iter.next() {
            for current in iter {
                assert_ne!(last, current, "name of commands in {ty} is not unique");
                last = current;
            }
        }
    }

    pub trait BindingMapExt {
        type T;
    }
    impl<T: Command> BindingMapExt for BindingMap<T> {
        type T = T;
    }
}
