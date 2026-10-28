//! Bounded data interchange observed in Flow 1.5.2. No vendor code or preset values.
use super::current_library::{Graph, GraphConfig, GraphList, GraphLists, GraphMode};
use serde::{
    Deserialize, Deserializer,
    de::{self, MapAccess, Visitor},
};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fmt;

const MAX_BYTES: usize = 2 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(untagged)]
enum Controls {
    Numbers([f64; 4]),
    Text(String),
}
impl Controls {
    fn values(self) -> Result<[f64; 4], String> {
        match self {
            Self::Numbers(v) => Ok(v),
            Self::Text(text) => {
                if text.len() > 256 {
                    return Err("A Flow curve must contain four numeric coordinates.".into());
                }
                let mut values = [0.0; 4];
                let mut parts = text.split(',');
                for value in &mut values {
                    *value = parts
                        .next()
                        .ok_or("A Flow curve needs four coordinates.")?
                        .trim()
                        .parse::<f64>()
                        .map_err(|_| "Flow curve coordinates must be finite numbers.")?;
                }
                if parts.next().is_some() {
                    return Err("A Flow curve needs exactly four coordinates.".into());
                }
                Ok(values)
            }
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    name: String,
    #[serde(default, rename = "_id")]
    _id: Option<Value>,
    #[serde(default)]
    value: Option<Controls>,
    #[serde(default)]
    curve: Option<Controls>,
}
struct UniqueMap(Vec<(String, Controls)>);
impl<'de> Deserialize<'de> for UniqueMap {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct MapVisitor;
        impl<'de> Visitor<'de> for MapVisitor {
            type Value = UniqueMap;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a Flow name-to-curve object")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                let mut names = BTreeSet::new();
                let mut values = Vec::new();
                while let Some((name, value)) = map.next_entry::<String, Controls>()? {
                    if values.len() >= 128 || !names.insert(name.clone()) {
                        return Err(de::Error::custom("Flow library exceeds 128 curves or contains a duplicate JSON key"));
                    }
                    values.push((name, value));
                }
                Ok(UniqueMap(values))
            }
        }
        d.deserialize_map(MapVisitor)
    }
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Input {
    Current(GraphLists),
    Records(Vec<Record>),
    Map(UniqueMap),
}

pub(super) fn decode(text: &str, list_name: &str) -> Result<GraphLists, String> {
    if text.len() > MAX_BYTES {
        return Err("Graph library exceeds 2 MiB.".into());
    }
    let input: Input = serde_json::from_str(text).map_err(|e| format!("Choose Current graph-lists JSON or a Flow Bezier library: {e}"))?;
    let records = match input {
        Input::Current(doc) => {
            doc.validate()?;
            return Ok(doc);
        }
        Input::Records(records) => {
            if records.len() > 128 {
                return Err("A Flow import supports at most 128 curves in one list.".into());
            }
            let mut values = Vec::new();
            for record in records {
                let controls = match (record.value, record.curve) {
                    (Some(v), None) | (None, Some(v)) => v,
                    _ => return Err("Each Flow record needs exactly one value or curve field.".into()),
                };
                values.push((record.name, controls));
            }
            values
        }
        Input::Map(values) => values.0,
    };
    let mut graphs: Vec<Graph> = Vec::new();
    for (name, controls) in records {
        let base = name.trim();
        if base.is_empty() || base.chars().count() > 80 || base.chars().any(char::is_control) {
            return Err("Flow curve names must contain 1–80 printable characters.".into());
        }
        let mut name = base.to_string();
        for n in 2..=130 {
            if !graphs.iter().any(|g| g.name.to_lowercase() == name.to_lowercase()) {
                break;
            }
            let suffix = format!(" ({n})");
            name = base.chars().take(80usize.saturating_sub(suffix.chars().count())).collect::<String>() + &suffix;
        }
        let config = GraphConfig { curve: controls.values()?, ..Default::default() };
        config.validate()?;
        graphs.push(Graph { name, config });
    }
    let doc = GraphLists { lists: vec![GraphList { name: list_name.trim().into(), graphs }], ..Default::default() };
    doc.validate()?;
    Ok(doc)
}
pub(super) fn encode(doc: &GraphLists) -> Result<(String, Vec<String>), String> {
    doc.validate()?;
    if doc.lists.len() != 1 {
        return Err("Choose one graph list for Flow export.".into());
    }
    let list = doc.lists.first().ok_or("Choose a graph list for Flow export.")?;
    let skipped: Vec<_> = list.graphs.iter().filter(|g| g.config.mode != GraphMode::Bezier).map(|g| g.name.clone()).collect();
    let records: Vec<_> = list.graphs.iter().filter(|g| g.config.mode == GraphMode::Bezier).map(|g| json!({"name":g.name,"value":g.config.curve})).collect();
    if records.is_empty() {
        return Err("This list has no Bezier graphs for Flow export. Choose another list or use Current JSON to preserve every graph mode.".into());
    }
    let text = serde_json::to_string_pretty(&records).map_err(|e| e.to_string())?;
    if text.len() > MAX_BYTES {
        return Err("Flow library exceeds 2 MiB.".into());
    }
    Ok((text, skipped))
}
