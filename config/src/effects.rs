use std::{
    collections::HashMap,
    io::{Read, Write},
    num::NonZeroUsize,
    sync::Mutex,
};

use color_eyre::{Result, eyre::Context};
use nom::{IResult, Parser, branch::alt, bytes::streaming::tag, sequence::preceded};
use ratatui_core::style::Color;
use serde::{Deserialize, de::Visitor};
use tachyonfx::{Effect, dsl::EffectDsl};

#[derive(Debug, Deserialize)]
struct ParseEffect {
    effect: String,
    fps: u8,
}

#[derive(Debug)]
struct ParseWidgetEffect {
    start: Option<Option<ParseEffect>>,
    main: Option<Option<ParseEffect>>,
    exit: Option<Option<ParseEffect>>,
}

impl<'de> Deserialize<'de> for ParseWidgetEffect {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        enum Field {
            Start,
            Main,
            Exit,
        }
        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct FieldVisitor;
                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("`start` or `exit`")
                    }

                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        match v {
                            "start" => Ok(Field::Start),
                            "exit" => Ok(Field::Exit),
                            "main" => Ok(Field::Main),
                            _ => Err(serde::de::Error::unknown_field(v, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }
        struct EffVisitor;
        impl<'de> Visitor<'de> for EffVisitor {
            type Value = ParseWidgetEffect;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct Effect")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut start = None;
                let mut main = None;
                let mut exit = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Start => {
                            if start.is_some() {
                                return Err(serde::de::Error::duplicate_field("start"));
                            }
                            let val: Option<ParseEffect> = map.next_value()?;
                            start = Some(val);
                        }
                        Field::Main => {
                            if main.is_some() {
                                return Err(serde::de::Error::duplicate_field("main"));
                            }
                            let val: Option<ParseEffect> = map.next_value()?;
                            main = Some(val);
                        }
                        Field::Exit => {
                            if exit.is_some() {
                                return Err(serde::de::Error::duplicate_field("exit"));
                            }
                            let val: Option<ParseEffect> = map.next_value()?;
                            exit = Some(val);
                        }
                    }
                }
                Ok(ParseWidgetEffect { start, main, exit })
            }
        }
        const FIELDS: &[&str] = &["start", "main", "exit"];
        deserializer.deserialize_struct("Effect", FIELDS, EffVisitor)
    }
}

#[derive(Deserialize, Debug)]
struct ParseEffects {
    start: Option<ParseEffect>,
    exit: Option<ParseEffect>,
    main: Option<ParseEffect>,
    #[serde(flatten)]
    views: HashMap<String, ParseWidgetEffect>,
}

#[derive(Debug, Clone)]
pub struct EffectInfo {
    pub effect: Effect,
    pub fps: u8,
}

#[derive(Debug)]
struct EffectSet {
    start: Option<Option<EffectInfo>>,
    main: Option<Option<EffectInfo>>,
    exit: Option<Option<EffectInfo>>,
}

#[derive(Debug)]
struct EffectSets {
    start: Option<EffectInfo>,
    exit: Option<EffectInfo>,
    main: Option<EffectInfo>,
    views: HashMap<String, EffectSet>,
}

#[derive(Debug)]
pub struct EffectStore {
    inner: Mutex<EffectSets>,
}

#[cfg(test)]
#[test]
fn check_default_effects() -> Result<()> {
    EffectStore::parse(include_str!("../effects.toml"))?;
    Ok(())
}

impl EffectStore {
    pub fn parse(effect_config: &str) -> Result<Self> {
        let colors = parse_colors()?;
        let parse: ParseEffects = toml::from_str(effect_config).context("parsing effect toml")?;

        fn to_set(input: ParseWidgetEffect, colors: TermColors) -> Result<EffectSet> {
            fn map(
                input: Option<Option<ParseEffect>>,
                colors: TermColors,
            ) -> Result<Option<Option<EffectInfo>>> {
                Ok(match input {
                    None => None,
                    Some(None) => Some(None),
                    Some(Some(e)) => Some(Some(compile_effect(colors, e)?)),
                })
            }
            Ok(EffectSet {
                start: map(input.start, colors).context("compiling start")?,
                main: map(input.main, colors).context("compiling main")?,
                exit: map(input.exit, colors).context("compiling exit")?,
            })
        }

        let store = EffectSets {
            start: parse
                .start
                .map(|e| compile_effect(colors, e).context("compiling general start"))
                .transpose()?,
            main: parse
                .main
                .map(|e| compile_effect(colors, e).context("compiling general main"))
                .transpose()?,
            exit: parse
                .exit
                .map(|e| compile_effect(colors, e).context("compiling general exit"))
                .transpose()?,
            views: parse
                .views
                .into_iter()
                .map(|(k, v)| -> Result<_> {
                    let val = to_set(v, colors).with_context(|| format!("for {k}"))?;
                    Ok((k, val))
                })
                .collect::<Result<HashMap<_, _>>>()?,
        };
        Ok(Self {
            inner: Mutex::new(store),
        })
    }

    pub fn start(&self, widget: &str) -> Option<EffectInfo> {
        let inner = self.inner.lock().expect("should never panic");
        inner
            .views
            .get(widget)
            .as_ref()
            .and_then(|s| s.start.as_ref())
            .unwrap_or(&inner.start)
            .clone()
    }

    pub fn main(&self, widget: &str) -> Option<EffectInfo> {
        let inner = self.inner.lock().expect("should never panic");
        inner
            .views
            .get(widget)
            .as_ref()
            .and_then(|s| s.main.as_ref())
            .unwrap_or(&inner.main)
            .clone()
    }

    pub fn exit(&self, widget: &str) -> Option<EffectInfo> {
        let inner = self.inner.lock().expect("should never panic");
        inner
            .views
            .get(widget)
            .as_ref()
            .and_then(|s| s.exit.as_ref())
            .unwrap_or(&inner.exit)
            .clone()
    }
}

fn compile_effect(colors: TermColors, e: ParseEffect) -> Result<EffectInfo> {
    let effect = EffectDsl::new()
        .compiler()
        .bind("fg", colors.fg)
        .bind("bg", colors.bg)
        .compile(&e.effect)?;
    Ok(EffectInfo { effect, fps: e.fps })
}

struct RawModeGuard;
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        crossterm::terminal::disable_raw_mode().expect("disabling raw mode failed")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TermColors {
    pub fg: Color,
    pub bg: Color,
}

fn parse_hex_digit(i: &[u8]) -> IResult<&[u8], u8> {
    let res = match i.first().copied() {
        Some(v @ b'0'..=b'9') => v - b'0',
        Some(v @ b'a'..=b'f') => v - b'a' + 10,
        Some(v @ b'A'..=b'F') => v - b'A' + 10,
        Some(_) => {
            return IResult::Err(nom::Err::Error(nom::error::Error::new(
                i,
                nom::error::ErrorKind::HexDigit,
            )));
        }
        None => {
            return IResult::Err(nom::Err::Incomplete(nom::Needed::Size(
                NonZeroUsize::new(1).unwrap(),
            )));
        }
    };
    Ok((&i[1..], res))
}

fn parse_hex<const N: usize>(mut i: &[u8]) -> IResult<&[u8], [u8; N]> {
    let mut res = [0u8; N];
    for r in res.iter_mut().take(N) {
        let (ri, rv) = parse_hex_digit(i)?;
        i = ri;
        *r = rv;
    }
    Ok((i, res))
}

fn parse_single_color(i: &[u8]) -> IResult<&[u8], u8> {
    fn map_1(i: [u8; 1]) -> u8 {
        let i = i[0];
        i | (i << 4)
    }
    fn map_2(i: [u8; 2]) -> u8 {
        let i1 = i[0];
        let i2 = i[1];
        (i1 << 4) | i2
    }
    fn map_4(i: [u8; 4]) -> u8 {
        (i[3]) | ((i[2]) << 4)
    }
    let c4 = nom::combinator::map(parse_hex::<4>, map_4);
    let c2 = nom::combinator::map(parse_hex::<2>, map_2);
    let c1 = nom::combinator::map(parse_hex::<1>, map_1);
    alt((c4, c2, c1)).parse(i)
}

fn parse_color(i: &[u8]) -> IResult<&[u8], Color> {
    let (i, _) = tag("rgb:")(i)?;
    let (i, r) = parse_single_color(i)?;
    let (i, _) = tag("/")(i)?;
    let (i, g) = parse_single_color(i)?;
    let (i, _) = tag("/")(i)?;
    let (i, b) = parse_single_color(i)?;
    Ok((i, Color::Rgb(r, g, b)))
}

fn parse_response(i: &[u8]) -> IResult<&[u8], TermColors> {
    let mut terminator = alt((tag("\x1b\\"), tag("\x07")));
    let mut separator = alt((
        tag(";"),
        preceded(alt((tag("\x1b\\"), tag("\x07"))), tag("\x1b]11;")),
    ));
    let (i, _) = tag("\x1b]10;")(i)?;
    let (i, fg) = parse_color(i)?;
    let (i, _) = separator.parse(i)?;
    let (i, bg) = parse_color(i)?;
    let (i, _) = terminator.parse(i)?;
    Ok((i, TermColors { fg, bg }))
}

const DEFAULT_COLORS: TermColors = TermColors {
    fg: Color::White,
    bg: Color::Black,
};

/**
This function queries the terminal for the current foreground and background colors.
It reads and writes to the terminal so running it concurrently might fail.
TODO: currently only supports unix
 */
pub fn parse_colors() -> Result<TermColors> {
    #[cfg(not(unix))]
    {
        return Ok(DEFAULT_COLORS);
    }
    #[cfg(unix)]
    {
        use crossterm::tty::IsTty;

        if (!cfg!(test)) && std::io::stdin().is_tty() {
            let _guard = if crossterm::terminal::is_raw_mode_enabled()
                .context("querying raw mode failed")?
            {
                None
            } else {
                crossterm::terminal::enable_raw_mode().context("enabling raw mode failed")?;
                Some(RawModeGuard)
            };
            std::io::stdout()
                .write_all(b"\x1b]10;?;?\x1b\\")
                .context("print escape sequence")?;
            std::io::stdout().flush().context("flushing stdout")?;
            let mut stdin = std::io::stdin().lock();
            let mut read = 0usize;
            let mut out = [0u8; 50];
            loop {
                use std::os::fd::AsFd;

                use nix::poll::{PollFd, PollFlags};

                if 1 == nix::poll::poll(&mut [PollFd::new(stdin.as_fd(), PollFlags::POLLIN)], 100u8)
                    .context("polling stdin")?
                {
                    read += stdin
                        .read(&mut out[read..])
                        .context("failed reading stdin")?;
                    match parse_response(&out[..read]) {
                        Ok((_, v)) => break Ok(v),
                        Err(nom::Err::Incomplete(_)) => continue,
                        Err(nom::Err::Error(e) | nom::Err::Failure(e)) => {
                            break Err(color_eyre::Report::msg(format!("{e:?}"))
                                .wrap_err("error parsing terminal colors"));
                        }
                    }
                } else {
                    tracing::warn!("querying terminal colors received timeout");
                    break Ok(DEFAULT_COLORS);
                }
            }
        } else {
            Ok(DEFAULT_COLORS)
        }
    }
}
