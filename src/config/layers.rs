use std::fmt;
use std::hash::{Hash, Hasher};

use pyo3::prelude::*;
use serde::ser::SerializeStruct;
use serde::{de, Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Default)]
#[pyclass(module = "tach.extension")]
pub struct LayerConfig {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub closed: bool,
}

impl PartialEq for LayerConfig {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.closed == other.closed
    }
}

impl Eq for LayerConfig {}

impl Hash for LayerConfig {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.closed.hash(state);
    }
}

impl Serialize for LayerConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if !self.closed {
            serializer.serialize_str(&self.name)
        } else {
            let mut state = serializer.serialize_struct("LayerConfig", 2)?;
            state.serialize_field("name", &self.name)?;
            state.serialize_field("closed", &self.closed)?;
            state.end()
        }
    }
}

struct LayerConfigVisitor;

impl<'de> de::Visitor<'de> for LayerConfigVisitor {
    type Value = LayerConfig;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("string or map")
    }

    fn visit_str<E>(self, value: &str) -> Result<LayerConfig, E>
    where
        E: de::Error,
    {
        Ok(LayerConfig::new(value, false))
    }

    fn visit_map<M>(self, mut map: M) -> Result<LayerConfig, M::Error>
    where
        M: de::MapAccess<'de>,
    {
        let mut name: Option<String> = None;
        let mut closed = false;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "name" => {
                    name = {
                        if name.is_some() {
                            return Err(de::Error::duplicate_field("name"));
                        }
                        Some(map.next_value()?)
                    }
                }
                "closed" => {
                    if closed {
                        return Err(de::Error::duplicate_field("closed"));
                    }
                    closed = map.next_value()?;
                }
                _ => {
                    return Err(de::Error::unknown_field(&key, &["name", "closed"]));
                }
            }
        }

        let name = name.ok_or_else(|| de::Error::missing_field("name"))?;

        Ok(LayerConfig::new(&name, closed))
    }
}

impl<'de> Deserialize<'de> for LayerConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(LayerConfigVisitor)
    }
}

impl LayerConfig {
    pub fn new(name: &str, closed: bool) -> Self {
        Self {
            name: name.to_string(),
            closed,
        }
    }
}
