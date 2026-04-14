use std::{collections::HashMap, sync::Mutex};

use color_eyre::{Result, eyre::Context};
use serde::{Deserialize, de::Visitor};
use tachyonfx::{Effect, dsl::EffectDsl};

#[derive(Debug)]
struct ParseEffect {
    start: Option<Option<String>>,
    exit: Option<Option<String>>,
}

impl<'de> Deserialize<'de> for ParseEffect {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        enum Field {
            Start,
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
                            _ => Err(serde::de::Error::unknown_field(v, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }
        struct EffVisitor;
        impl<'de> Visitor<'de> for EffVisitor {
            type Value = ParseEffect;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct Effect")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut start = None;
                let mut exit = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Start => {
                            if start.is_some() {
                                return Err(serde::de::Error::duplicate_field("start"));
                            }
                            let val: Option<String> = map.next_value()?;
                            start = Some(val);
                        }
                        Field::Exit => {
                            if exit.is_some() {
                                return Err(serde::de::Error::duplicate_field("exit"));
                            }
                            let val: Option<String> = map.next_value()?;
                            exit = Some(val);
                        }
                    }
                }
                Ok(ParseEffect { start, exit })
            }
        }
        const FIELDS: &[&str] = &["start", "exit"];
        deserializer.deserialize_struct("Effect", FIELDS, EffVisitor)
    }
}

#[derive(Deserialize, Debug)]
struct ParseEffects {
    start: Option<String>,
    exit: Option<String>,
    #[serde(flatten)]
    views: HashMap<String, ParseEffect>,
}

struct EffectSet {
    start: Option<Option<Effect>>,
    exit: Option<Option<Effect>>,
}

struct EffectSets {
    start: Option<Effect>,
    exit: Option<Effect>,
    views: HashMap<String, EffectSet>,
}

pub struct EffectStore {
    inner: Mutex<EffectSets>,
}

impl EffectStore {
    pub fn parse(effect_config: &str) -> Result<Self> {
        let parse: ParseEffects = toml::from_str(effect_config).context("parsing effect toml")?;

        fn to_set(input: ParseEffect) -> Result<EffectSet> {
            fn map(input: Option<Option<String>>) -> Result<Option<Option<Effect>>> {
                Ok(match input {
                    None => None,
                    Some(None) => Some(None),
                    Some(Some(e)) => Some(Some(EffectDsl::new().compiler().compile(&e)?)),
                })
            }
            Ok(EffectSet {
                start: map(input.start).context("compiling start")?,
                exit: map(input.exit).context("compiling exit")?,
            })
        }

        let store = EffectSets {
            start: parse
                .start
                .map(|e| {
                    EffectDsl::new()
                        .compiler()
                        .compile(&e)
                        .context("compiling general start")
                })
                .transpose()?,
            exit: parse
                .exit
                .map(|e| {
                    EffectDsl::new()
                        .compiler()
                        .compile(&e)
                        .context("compiling general exit")
                })
                .transpose()?,
            views: parse
                .views
                .into_iter()
                .map(|(k, v)| -> Result<_> {
                    let val = to_set(v).with_context(|| format!("for {k}"))?;
                    Ok((k, val))
                })
                .collect::<Result<HashMap<_, _>>>()?,
        };
        Ok(Self {
            inner: Mutex::new(store),
        })
    }

    pub fn start(&self, widget: &str) -> Option<Effect> {
        let inner = self.inner.lock().expect("should never panic");
        inner
            .views
            .get(widget)
            .as_ref()
            .and_then(|s| s.start.as_ref())
            .unwrap_or(&inner.start)
            .clone()
    }

    pub fn exit(&self, widget: &str) -> Option<Effect> {
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
