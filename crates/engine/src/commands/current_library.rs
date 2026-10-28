//! Validated portable graph lists. Library changes never edit project history.
use super::current::{Curve, Physics, PhysicsMode, PhysicsPreset, Preset, builtin_presets};
use super::current_standard::{Direction, Standard, StandardFamily};
use super::{CommandSpec, bad};
use crate::{Result, Session, cmd, query};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;
#[cfg(not(target_arch = "wasm32"))]
use std::io::Read;
use std::io::Write;

const FILE: &str = "current-graph-lists.json";
const MAX_BYTES: usize = 2 * 1024 * 1024;
const FORMAT: &str = "current.graph-lists";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GraphMode {
    #[default]
    Bezier,
    Spring,
    Elastic,
    Bounce,
    Standard,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GraphConfig {
    pub mode: GraphMode,
    pub curve: [f64; 4],
    pub frequency: f64,
    pub damping: f64,
    pub amplitude: f64,
    pub density: u8,
    pub family: StandardFamily,
    pub direction: Direction,
}
impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            mode: GraphMode::Bezier,
            curve: Curve::DEFAULT.0,
            frequency: 3.0,
            damping: 6.0,
            amplitude: 1.0,
            density: 1,
            family: StandardFamily::Quad,
            direction: Direction::InOut,
        }
    }
}
impl GraphConfig {
    pub fn physics(&self) -> Physics {
        Physics {
            mode: match self.mode {
                GraphMode::Elastic => PhysicsMode::Elastic,
                GraphMode::Bounce => PhysicsMode::Bounce,
                _ => PhysicsMode::Spring,
            },
            frequency: self.frequency,
            damping: self.damping,
            amplitude: self.amplitude,
        }
    }
    pub fn validate(&self) -> std::result::Result<(), String> {
        let [x1, y1, x2, y2] = self.curve;
        if !self.curve.iter().all(|n| n.is_finite())
            || !(0.0..=1.0).contains(&x1)
            || !(0.0..=1.0).contains(&x2)
            || !(-9.0..=10.0).contains(&y1)
            || !(-9.0..=10.0).contains(&y2)
        {
            return Err("Bezier graph handles must be finite, with X in 0–1 and Y in -9–10.".into());
        }
        self.physics().validate()?;
        if !(1..=8).contains(&self.density) {
            return Err("Graph density must be between 1 and 8.".into());
        }
        Ok(())
    }
    pub fn sample(&self, time: f64) -> f64 {
        if !time.is_finite() || time <= 0.0 {
            return 0.0;
        }
        if time >= 1.0 {
            return 1.0;
        }
        if self.validate().is_err() {
            return time;
        }
        match self.mode {
            GraphMode::Bezier => super::current_standard::bezier_sample(self.curve, time),
            GraphMode::Standard => Standard { family: self.family, direction: self.direction }.sample(time),
            _ => self.physics().sample(time),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Graph {
    pub name: String,
    pub config: GraphConfig,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphList {
    pub name: String,
    pub graphs: Vec<Graph>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphLists {
    pub format: String,
    pub version: u32,
    pub lists: Vec<GraphList>,
}
impl Default for GraphLists {
    fn default() -> Self {
        Self { format: FORMAT.into(), version: 1, lists: Vec::new() }
    }
}

fn valid_name(name: &str) -> bool {
    !name.trim().is_empty() && name == name.trim() && name.chars().count() <= 80 && !name.chars().any(char::is_control)
}
fn same_name(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}
impl GraphLists {
    pub fn validate(&self) -> std::result::Result<(), String> {
        if self.format != FORMAT || self.version != 1 {
            return Err("Choose a Current graph-lists JSON document with version 1.".into());
        }
        if self.lists.len() > 32 {
            return Err("A Current library supports at most 32 custom lists.".into());
        }
        let mut names = BTreeSet::new();
        let mut total = 0usize;
        for list in &self.lists {
            if !valid_name(&list.name) || same_name(&list.name, "Defaults") || !names.insert(list.name.to_lowercase()) {
                return Err("List names must be unique, 1–80 characters, and cannot be Defaults.".into());
            }
            if list.graphs.len() > 128 {
                return Err("Each graph list supports at most 128 graphs.".into());
            }
            total = total.checked_add(list.graphs.len()).filter(|n| *n <= 4096).ok_or("A Current library supports at most 4,096 graphs.")?;
            let mut graphs = BTreeSet::new();
            for graph in &list.graphs {
                if !valid_name(&graph.name) || !graphs.insert(graph.name.to_lowercase()) {
                    return Err("Graph names must be unique within their list and contain 1–80 characters.".into());
                }
                graph.config.validate()?;
            }
        }
        Ok(())
    }
    pub fn parse(text: &str) -> std::result::Result<Self, String> {
        if text.len() > MAX_BYTES {
            return Err("Current graph JSON exceeds 2 MiB.".into());
        }
        let document: Self = serde_json::from_str(text).map_err(|e| format!("Invalid Current graph JSON: {e}"))?;
        document.validate()?;
        Ok(document)
    }
    pub fn to_json(&self) -> std::result::Result<String, String> {
        self.validate()?;
        encode(self)
    }
}

struct Buffer(Vec<u8>);
impl Write for Buffer {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        if self.0.len().checked_add(data.len()).is_none_or(|n| n > MAX_BYTES) {
            return Err(std::io::Error::other("Current graph JSON exceeds 2 MiB"));
        }
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn encode(value: &impl Serialize) -> std::result::Result<String, String> {
    let mut writer = Buffer(Vec::new());
    serde_json::to_writer_pretty(&mut writer, value).map_err(|e| e.to_string())?;
    String::from_utf8(writer.0).map_err(|e| e.to_string())
}

pub fn default_graphs() -> GraphList {
    let mut graphs: Vec<_> = builtin_presets()
        .into_iter()
        .map(|p| Graph { name: if p.name == "Linear" { "Linear Bezier".into() } else { p.name }, config: GraphConfig { curve: p.curve, ..Default::default() } })
        .collect();
    graphs.push(Graph { name: "CSS Ease".into(), config: GraphConfig { curve: [0.25, 0.1, 0.25, 1.0], ..Default::default() } });
    for family in StandardFamily::ALL {
        for direction in Direction::ALL {
            let suffix = match direction {
                Direction::In => " In",
                Direction::Out => " Out",
                Direction::InOut => "",
            };
            graphs.push(Graph {
                name: format!("{}{suffix}", family.label()),
                config: GraphConfig { mode: GraphMode::Standard, family, direction, ..Default::default() },
            });
        }
    }
    for mode in [GraphMode::Spring, GraphMode::Elastic, GraphMode::Bounce] {
        let name = match mode {
            GraphMode::Elastic => "Elastic",
            GraphMode::Bounce => "Bounce",
            _ => "Spring",
        };
        graphs.push(Graph {
            name: name.into(),
            config: GraphConfig {
                mode,
                frequency: if mode == GraphMode::Elastic { 4.0 } else { 3.0 },
                damping: if mode == GraphMode::Elastic {
                    7.0
                } else if mode == GraphMode::Bounce {
                    3.0
                } else {
                    6.0
                },
                ..Default::default()
            },
        });
    }
    GraphList { name: "Defaults".into(), graphs }
}

fn write(s: &Session, doc: &GraphLists) -> Result<()> {
    let text = doc.to_json().map_err(|e| bad("current.graphLists", e))?;
    let store = s.config.as_ref().ok_or_else(|| bad("current.graphLists", "This session has no settings store for graph lists."))?;
    store.write(FILE, &text).map_err(|e| bad("current.graphLists", format!("Cannot save graph lists: {e}")))
}
fn load(s: &Session) -> Result<GraphLists> {
    let Some(store) = &s.config else { return Ok(GraphLists::default()) };
    let text = store.read_bounded(FILE, MAX_BYTES).map_err(|e| bad("current.graphLists", format!("Cannot read graph lists: {e}")))?;
    if let Some(text) = text {
        return GraphLists::parse(&text).map_err(|e| bad("current.graphLists", e));
    }
    let mut doc = GraphLists::default();
    let mut found = false;
    for (file, name, physics) in [("current-curves.json", "Migrated Bezier", false), ("current-physics-curves.json", "Migrated Motion", true)] {
        let Some(text) = store.read_bounded(file, 256 * 1024).map_err(|e| bad("current.graphLists", format!("Cannot migrate old curves: {e}")))? else {
            continue;
        };
        found = true;
        let mut graphs: Vec<Graph> = if physics {
            let presets: Vec<PhysicsPreset> = serde_json::from_str(&text).map_err(|e| bad("current.graphLists", format!("Invalid old motion library: {e}")))?;
            presets
                .into_iter()
                .map(|p| Graph {
                    name: p.name,
                    config: GraphConfig {
                        mode: match p.physics.mode {
                            PhysicsMode::Spring => GraphMode::Spring,
                            PhysicsMode::Elastic => GraphMode::Elastic,
                            PhysicsMode::Bounce => GraphMode::Bounce,
                        },
                        frequency: p.physics.frequency,
                        damping: p.physics.damping,
                        amplitude: p.physics.amplitude,
                        ..Default::default()
                    },
                })
                .collect()
        } else {
            let presets: Vec<Preset> = serde_json::from_str(&text).map_err(|e| bad("current.graphLists", format!("Invalid old Bezier library: {e}")))?;
            presets.into_iter().map(|p| Graph { name: p.name, config: GraphConfig { curve: p.curve, ..Default::default() } }).collect()
        };
        let mut seen = BTreeSet::new();
        for graph in &mut graphs {
            let base = graph.name.chars().filter(|c| !c.is_control()).take(80).collect::<String>().trim().to_string();
            let base = if base.is_empty() { "Migrated graph".to_string() } else { base };
            graph.name = base.clone();
            for n in 2..=130 {
                if seen.insert(graph.name.to_lowercase()) {
                    break;
                }
                let suffix = format!(" ({n})");
                graph.name = base.chars().take(80usize.saturating_sub(suffix.chars().count())).collect::<String>() + &suffix;
            }
        }
        doc.lists.push(GraphList { name: name.into(), graphs });
    }
    if found {
        write(s, &doc)?;
    }
    Ok(doc)
}

fn name<'a>(p: &'a Value, field: &str) -> Result<&'a str> {
    p.get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|n| valid_name(n))
        .ok_or_else(|| bad("current.graphLists", format!("Pass `{field}` using 1–80 characters.")))
}
fn list_index(doc: &GraphLists, name: &str) -> Result<usize> {
    if same_name(name, "Defaults") {
        return Err(bad("current.graphLists", "Defaults is read-only. Create a custom list and save a copy there."));
    }
    doc.lists.iter().position(|l| same_name(&l.name, name)).ok_or_else(|| bad("current.graphLists", "Choose an existing custom graph list."))
}
fn reply(doc: &GraphLists) -> Value {
    json!({"format":FORMAT,"version":1,"lists":doc.lists,"defaults":default_graphs()})
}
fn lists(s: &mut Session, _: &Value) -> Result<Value> {
    Ok(reply(&load(s)?))
}
fn create(s: &mut Session, p: &Value) -> Result<Value> {
    let name = name(p, "name")?;
    let mut doc = load(s)?;
    doc.lists.push(GraphList { name: name.into(), graphs: Vec::new() });
    write(s, &doc)?;
    Ok(reply(&doc))
}
fn rename(s: &mut Session, p: &Value) -> Result<Value> {
    let mut doc = load(s)?;
    let i = list_index(&doc, name(p, "list")?)?;
    let list = doc.lists.get_mut(i).ok_or_else(|| bad("current.graphLists", "The list is missing."))?;
    list.name = name(p, "name")?.into();
    write(s, &doc)?;
    Ok(reply(&doc))
}
fn delete_list(s: &mut Session, p: &Value) -> Result<Value> {
    let mut doc = load(s)?;
    let i = list_index(&doc, name(p, "list")?)?;
    doc.lists.remove(i);
    write(s, &doc)?;
    Ok(reply(&doc))
}
fn save_graph(s: &mut Session, p: &Value) -> Result<Value> {
    let mut doc = load(s)?;
    let i = list_index(&doc, name(p, "list")?)?;
    let graph = Graph {
        name: name(p, "name")?.into(),
        config: serde_json::from_value(p.get("config").cloned().ok_or_else(|| bad("current.saveGraph", "Pass the graph configuration."))?)
            .map_err(|e| bad("current.saveGraph", e.to_string()))?,
    };
    let list = doc.lists.get_mut(i).ok_or_else(|| bad("current.saveGraph", "The list is missing."))?;
    if let Some(old) = list.graphs.iter_mut().find(|g| same_name(&g.name, &graph.name)) {
        *old = graph;
    } else {
        list.graphs.push(graph);
    }
    write(s, &doc)?;
    Ok(reply(&doc))
}
fn delete_graph(s: &mut Session, p: &Value) -> Result<Value> {
    let mut doc = load(s)?;
    let i = list_index(&doc, name(p, "list")?)?;
    let graph = name(p, "name")?;
    let list = doc.lists.get_mut(i).ok_or_else(|| bad("current.deleteGraph", "The list is missing."))?;
    let before = list.graphs.len();
    list.graphs.retain(|g| !same_name(&g.name, graph));
    if list.graphs.len() == before {
        return Err(bad("current.deleteGraph", "Choose a saved graph in this list."));
    }
    write(s, &doc)?;
    Ok(reply(&doc))
}

fn import(s: &mut Session, p: &Value) -> Result<Value> {
    if ["text", "document", "path"].iter().filter(|k| p.get(**k).is_some()).count() != 1 {
        return Err(bad("current.importGraphLists", "Pass exactly one of text, document or path."));
    }
    let text = if let Some(text) = p.get("text") {
        text.as_str().ok_or_else(|| bad("current.importGraphLists", "Text must be a JSON string."))?.to_string()
    } else if let Some(document) = p.get("document") {
        encode(document).map_err(|e| bad("current.importGraphLists", e))?
    } else {
        let path = p
            .get("path")
            .and_then(Value::as_str)
            .filter(|p| !p.is_empty() && p.len() <= 32768)
            .ok_or_else(|| bad("current.importGraphLists", "Choose a graph JSON path."))?;
        read_path(path).map_err(|e| bad("current.importGraphLists", e))?
    };
    let mut imported =
        super::current_flow::decode(&text, p.get("name").and_then(Value::as_str).unwrap_or("Imported Flow")).map_err(|e| bad("current.importGraphLists", e))?;
    if let Some(path) = p.get("path").and_then(Value::as_str) {
        if imported.lists.len() != 1 {
            return Err(bad("current.importGraphLists", "Choose a file containing one graph list."));
        }
        let name = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| valid_name(s))
            .ok_or_else(|| bad("current.importGraphLists", "Use a file name of 1 to 80 characters."))?;
        imported.lists[0].name = name.to_string();
    }
    let mut doc = load(s)?;
    let mut added = Vec::new();
    for mut list in imported.lists {
        let base = list.name.clone();
        for n in 2..=34 {
            if !doc.lists.iter().any(|l| same_name(&l.name, &list.name)) {
                break;
            }
            let suffix = format!(" ({n})");
            list.name = base.chars().take(80usize.saturating_sub(suffix.chars().count())).collect::<String>() + &suffix;
        }
        added.push(list.name.clone());
        doc.lists.push(list);
    }
    write(s, &doc)?;
    let mut result = reply(&doc);
    result["imported"] = json!(added);
    Ok(result)
}
fn export(s: &mut Session, p: &Value) -> Result<Value> {
    let flow = p.get("format").and_then(Value::as_str) == Some("flow");
    if p.get("format").is_some_and(|f| !matches!(f.as_str(), Some("flow" | "current"))) {
        return Err(bad("current.exportGraphLists", "Export format must be current or flow."));
    }
    if flow && !p.get("list").is_some_and(Value::is_string) {
        return Err(bad("current.exportGraphLists", "Choose one graph list for Flow export."));
    }
    let mut doc = if let Some(list) = p.get("list").and_then(Value::as_str) {
        if same_name(list, "Defaults") {
            let mut defaults = default_graphs();
            defaults.name = "Defaults Copy".into();
            GraphLists { lists: vec![defaults], ..Default::default() }
        } else {
            let doc = load(s)?;
            let i = list_index(&doc, list)?;
            GraphLists { lists: vec![doc.lists.get(i).cloned().ok_or_else(|| bad("current.exportGraphLists", "The list is missing."))?], ..Default::default() }
        }
    } else {
        load(s)?
    };
    doc.format = FORMAT.into();
    let (text, skipped) = if flow {
        super::current_flow::encode(&doc).map_err(|e| bad("current.exportGraphLists", e))?
    } else {
        (doc.to_json().map_err(|e| bad("current.exportGraphLists", e))?, Vec::new())
    };
    if let Some(path) = p.get("path") {
        let path = path.as_str().filter(|p| !p.is_empty()).ok_or_else(|| bad("current.exportGraphLists", "Choose an export path."))?;
        write_path(s, path, text.as_bytes()).map_err(|e| bad("current.exportGraphLists", e))?;
    }
    let total: usize = doc.lists.iter().map(|l| l.graphs.len()).sum();
    Ok(
        json!({"document":doc,"json":text,"format":if flow {"flow"} else {"current"},"exportedCount":total.saturating_sub(skipped.len()),"skippedCount":skipped.len(),"skipped":skipped}),
    )
}
#[cfg(not(target_arch = "wasm32"))]
fn read_path(path: &str) -> std::result::Result<String, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("Cannot open graph JSON: {e}"))?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_BYTES {
        return Err("Current graph JSON exceeds 2 MiB.".into());
    }
    String::from_utf8(bytes).map_err(|e| format!("Graph JSON must be UTF-8: {e}"))
}
#[cfg(target_arch = "wasm32")]
fn read_path(_: &str) -> std::result::Result<String, String> {
    Err("Import graph JSON using the text or document parameter in this host.".into())
}
fn write_path(s: &Session, path: &str, bytes: &[u8]) -> std::result::Result<(), String> {
    let standard = crate::config::StdFiles;
    let files = s.config.as_ref().map(|c| c.files()).unwrap_or(&standard);
    files.write(std::path::Path::new(path), bytes).map_err(|e| format!("Cannot export graph JSON: {e}"))
}
pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        query!("current.graphLists", "Current: Graph Lists", "{}", lists),
        cmd!("current.createGraphList", "Current: Create List", [], None, "{name}", super::always, create),
        cmd!("current.renameGraphList", "Current: Rename List", [], None, "{list,name}", super::always, rename),
        cmd!("current.deleteGraphList", "Current: Delete List", [], None, "{list}", super::always, delete_list),
        cmd!("current.saveGraph", "Current: Save Graph", [], None, "{list,name,config}", super::always, save_graph),
        cmd!("current.deleteGraph", "Current: Delete Graph", [], None, "{list,name}", super::always, delete_graph),
        cmd!("current.importGraphLists", "Current: Import Graph Lists", [], None, "{document?|text?|path?}", super::always, import),
        query!("current.exportGraphLists", "Current: Export Graph Lists", "{list?,path?}", export),
    ]
}

#[cfg(test)]
#[path = "current_library_tests.rs"]
mod tests;
