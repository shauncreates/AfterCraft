//! Current: normalized cubic-Bezier easing translated into native keyframe metadata.
//!
//! The curve is original editor data, not a proprietary preset or expression. Only intervals
//! whose two adjacent keys are selected change; values, times and motion paths are preserved.

use std::collections::{BTreeMap, BTreeSet};

use effectcraft_keyframe::{Ease, Interp, Value as KeyValue};
use effectcraft_project::{LayerId, Property, Uid};
use effectcraft_time::Tick;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{CommandSpec, bad, has_keys};
use crate::{EngineError, Result, Session, cmd, query};

pub use super::current_library::{Graph, GraphConfig, GraphList, GraphLists, GraphMode, default_graphs};
#[doc(hidden)]
pub use super::current_physics::Gesture as PhysicsGesture;
pub use super::current_physics::{Physics, PhysicsMode, PhysicsPreset};
pub use super::current_standard::{Direction, Standard, StandardFamily};

const APPLY: &str = "keys.currentApply";
const LIBRARY_FILE: &str = "current-curves.json";

/// An original Current easing preset. User curves are stored in AfterCraft's own settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub curve: [f64; 4],
}

pub fn builtin_presets() -> Vec<Preset> {
    [
        ("Linear", [1.0 / 3.0, 1.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0]),
        ("Drift", [0.3, 0.0, 0.7, 1.0]),
        ("Accelerate", [0.6, 0.0, 0.8, 0.4]),
        ("Settle", [0.2, 0.6, 0.4, 1.0]),
        ("Snap", [0.78, 0.0, 0.22, 1.0]),
        ("Anticipate", [0.35, -0.45, 0.65, 1.0]),
        ("Overshoot", [0.35, 0.0, 0.65, 1.45]),
    ]
    .into_iter()
    .map(|(name, curve)| Preset { name: name.into(), curve })
    .collect()
}

fn user_presets(s: &Session) -> Result<Vec<Preset>> {
    let Some(store) = &s.config else { return Ok(Vec::new()) };
    let text = store.read_bounded(LIBRARY_FILE, 256 * 1024).map_err(|e| bad("current.presets", format!("Cannot read Current curves: {e}")))?;
    let Some(text) = text else { return Ok(Vec::new()) };
    let curves: Vec<Preset> = serde_json::from_str(&text).map_err(|e| bad("current.presets", format!("Invalid Current library: {e}")))?;
    if curves.len() > 128 || curves.iter().any(|p| p.name.is_empty() || p.name.chars().count() > 80 || Curve(p.curve).validate().is_err()) {
        return Err(bad("current.presets", "Current library contains invalid curves or exceeds 128 presets."));
    }
    Ok(curves)
}

fn presets(s: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!({"builtins": builtin_presets(), "user": user_presets(s)?}))
}

fn write_presets(s: &Session, presets: &[Preset]) -> Result<()> {
    let store = s.config.as_ref().ok_or_else(|| bad("current.savePreset", "This session has no settings store for saved curves."))?;
    let text = serde_json::to_string_pretty(presets).map_err(|e| bad("current.savePreset", format!("Cannot encode curves: {e}")))?;
    store.write(LIBRARY_FILE, &text).map_err(|e| bad("current.savePreset", format!("Cannot save Current curves: {e}")))
}

fn save_preset(s: &mut Session, p: &Value) -> Result<Value> {
    let name = p
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|n| !n.is_empty() && n.chars().count() <= 80)
        .ok_or_else(|| bad("current.savePreset", "Name the curve using 1–80 characters."))?;
    let curve = parse_curve(p)?.0;
    let mut user = user_presets(s)?;
    if let Some(existing) = user.iter_mut().find(|p| p.name == name) {
        existing.curve = curve;
    } else {
        if user.len() >= 128 {
            return Err(bad("current.savePreset", "The library is full. Delete a saved curve before adding another."));
        }
        user.push(Preset { name: name.into(), curve });
    }
    write_presets(s, &user)?;
    Ok(json!({"saved": name, "user": user}))
}

fn delete_preset(s: &mut Session, p: &Value) -> Result<Value> {
    let name = p.get("name").and_then(Value::as_str).ok_or_else(|| bad("current.deletePreset", "Pass the saved curve's `name`."))?;
    let mut user = user_presets(s)?;
    let before = user.len();
    user.retain(|p| p.name != name);
    if user.len() == before {
        return Err(bad("current.deletePreset", "Choose one of your saved curves to delete."));
    }
    write_presets(s, &user)?;
    Ok(json!({"deleted": name, "user": user}))
}

/// Cubic handles in normalized time/value space. Endpoints are (0,0) and (1,1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Curve(pub [f64; 4]);

impl Curve {
    pub const DEFAULT: Self = Self([0.3, 0.0, 0.7, 1.0]);

    pub fn validate(self) -> std::result::Result<Self, String> {
        let [x1, y1, x2, y2] = self.0;
        if !self.0.iter().all(|n| n.is_finite()) || !(0.0..=1.0).contains(&x1) || !(0.0..=1.0).contains(&x2) {
            return Err("Use four finite curve values; time handles must be between 0 and 1.".into());
        }
        if !(-9.0..=10.0).contains(&y1) || !(-9.0..=10.0).contains(&y2) {
            return Err("Value handles must be between -9 and 10.".into());
        }
        if (x1 == 0.0 && y1 != 0.0) || (x2 == 1.0 && y2 != 1.0) {
            return Err("A vertical endpoint handle needs infinite velocity. Move its time handle slightly inward.".into());
        }
        Ok(self)
    }

    /// Keep library data exact; approximate only an otherwise infinite endpoint
    /// velocity while preparing a native keyframe edit.
    pub fn for_apply(self) -> std::result::Result<(Self, Vec<String>), String> {
        let mut curve = self;
        let start = curve.0[0] == 0.0 && curve.0[1] != 0.0;
        let end = curve.0[2] == 1.0 && curve.0[3] != 1.0;
        if start {
            curve.0[0] = 0.001;
        }
        if end {
            curve.0[2] = 0.999;
        }
        curve.validate()?;
        let notices = if start || end {
            vec!["Vertical time handles were adjusted to 0.001 / 0.999 for finite keyframe velocity. The saved graph is unchanged.".into()]
        } else {
            Vec::new()
        };
        Ok((curve, notices))
    }

    fn eases(self, average: f64) -> Option<(Ease, Ease)> {
        let [x1, y1, x2, y2] = self.0;
        let out = Ease { speed: if x1 == 0.0 { 0.0 } else { average * y1 / x1 }, influence: x1 };
        let incoming = Ease { speed: if x2 == 1.0 { 0.0 } else { average * (1.0 - y2) / (1.0 - x2) }, influence: 1.0 - x2 };
        (out.speed.is_finite() && incoming.speed.is_finite()).then_some((out, incoming))
    }
}

pub(super) fn selected_segments(s: &Session) -> Result<Vec<(LayerId, Uid, usize)>> {
    let comp = s.active_comp().ok_or(EngineError::NoComp)?;
    if s.state.selected_keys.len() > 100_000 {
        return Err(bad(APPLY, "Select at most 100,000 keyframes per operation."));
    }
    let mut groups: BTreeMap<(LayerId, Uid), BTreeSet<Tick>> = BTreeMap::new();
    for key in &s.state.selected_keys {
        groups.entry((key.layer, key.prop)).or_default().insert(key.time);
    }
    let mut segments = Vec::new();
    for ((lid, uid), selected) in groups {
        let Some(layer) = comp.layer(lid) else { continue };
        if layer.switches.locked {
            return Err(bad(APPLY, "Unlock the selected layer before applying easing."));
        }
        let Some(prop) = layer.props.find(uid) else { continue };
        if prop.hold_only || prop.static_only {
            continue;
        }
        for (i, pair) in prop.keys.windows(2).enumerate() {
            let (Some(a), Some(b)) = (pair.first(), pair.get(1)) else { continue };
            if selected.contains(&a.time) && selected.contains(&b.time) && a.value.interpolates() && b.value.interpolates() && b.time > a.time {
                segments.push((lid, uid, i));
            }
        }
    }
    if segments.is_empty() {
        return Err(bad(APPLY, "Select two adjacent keyframes of an animatable property."));
    }
    Ok(segments)
}

/// Average speeds in layer time. Spatial properties use path length rather than chord length.
fn averages(prop: &Property, i: usize) -> Option<Vec<f64>> {
    let next = i.checked_add(1)?;
    let (a, b) = (prop.keys.get(i)?, prop.keys.get(next)?);
    let seconds = b.time.seconds() - a.time.seconds();
    if !seconds.is_finite() || seconds <= 0.0 {
        return None;
    }
    let v = if prop.spatial && matches!(a.value, KeyValue::Vec2(_) | KeyValue::Vec3(_)) {
        vec![effectcraft_keyframe::spatial_segment_length(&prop.keys, i) / seconds]
    } else {
        let (from, to) = (a.value.components(), b.value.components());
        if from.is_empty() {
            vec![1.0 / seconds] // Path/gradient interpolation uses overall progress.
        } else {
            from.iter().zip(to.iter()).map(|(a, b)| (b - a) / seconds).collect()
        }
    };
    v.iter().all(|n| n.is_finite()).then_some(v)
}

fn curve_data(p: &Value) -> Result<Curve> {
    let values = p.get("curve").and_then(Value::as_array).filter(|v| v.len() == 4).ok_or_else(|| bad(APPLY, "Pass `curve: [x1,y1,x2,y2]`."))?;
    let mut curve = [0.0; 4];
    for (target, value) in curve.iter_mut().zip(values) {
        *target = value.as_f64().ok_or_else(|| bad(APPLY, "Curve values must be numbers."))?;
    }
    Ok(Curve(curve))
}
fn parse_curve(p: &Value) -> Result<Curve> {
    curve_data(p)?.validate().map_err(|e| bad(APPLY, e))
}

fn apply(s: &mut Session, p: &Value) -> Result<Value> {
    let original = curve_data(p)?;
    let (curve, notices) = original.for_apply().map_err(|e| bad(APPLY, e))?;
    let segments = selected_segments(s)?;
    let cid = s.active_comp_id().ok_or(EngineError::NoComp)?;
    let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?;
    let mut plan = Vec::with_capacity(segments.len());
    let mut freeze = BTreeMap::new();
    for (lid, uid, i) in segments {
        let prop = comp.layer(lid).and_then(|l| l.props.find(uid)).ok_or_else(|| bad(APPLY, "Selected property is missing."))?;
        if prop.spatial && (curve.0[1] < 0.0 || curve.0[1] > 1.0 || curve.0[3] < 0.0 || curve.0[3] > 1.0) {
            return Err(bad(APPLY, "Separate Position dimensions before applying overshoot; a motion path has bounded endpoints."));
        }
        let speeds = averages(prop, i).ok_or_else(|| bad(APPLY, "The selected interval contains invalid times or values."))?;
        let sides = speeds
            .into_iter()
            .map(|speed| curve.eases(speed))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| bad(APPLY, "Curve velocity exceeds the supported range."))?;
        let (out, incoming): (Vec<_>, Vec<_>) = sides.into_iter().unzip();
        for index in [Some(i), i.checked_add(1)].into_iter().flatten() {
            if prop.keys.get(index).is_some_and(|key| key.auto_bezier) {
                let dims = if prop.spatial { 1 } else { prop.value.dims().max(1) };
                let side =
                    |out| (0..dims).map(|d| effectcraft_keyframe::side_ease(&prop.keys, index, d, prop.spatial, out).unwrap_or_default()).collect::<Vec<_>>();
                freeze.insert((lid, uid, index), (side(false), side(true)));
            }
        }
        plan.push((lid, uid, i, out, incoming));
    }
    let count = plan.len();
    s.edit("Current: Apply Easing", None, |project, _| {
        let comp = project.comp_mut(cid).ok_or(EngineError::NoComp)?;
        // Disabling auto tangents must not change the neighbouring, unselected interval.
        for ((lid, uid, i), (incoming, outgoing)) in freeze {
            if let Some(key) = comp.layer_mut(lid).and_then(|l| l.props.find_mut(uid)).and_then(|pr| pr.keys.get_mut(i)) {
                key.in_ease = incoming;
                key.out_ease = outgoing;
                key.auto_bezier = false;
            }
        }
        for (lid, uid, i, outgoing, incoming) in plan {
            let prop = comp.layer_mut(lid).and_then(|l| l.props.find_mut(uid)).ok_or_else(|| bad(APPLY, "Selected property is missing."))?;
            let key = prop.keys.get_mut(i).ok_or_else(|| bad(APPLY, "Selected keyframe is missing."))?;
            key.out_interp = Interp::Bezier;
            key.out_ease = outgoing;
            key.auto_bezier = false;
            key.continuous = false;
            let key = i.checked_add(1).and_then(|j| prop.keys.get_mut(j)).ok_or_else(|| bad(APPLY, "Adjacent keyframe is missing."))?;
            key.in_interp = Interp::Bezier;
            key.in_ease = incoming;
            key.auto_bezier = false;
            key.continuous = false;
        }
        Ok(())
    })?;
    Ok(json!({"segments": count, "curve": original.0,"appliedCurve":curve.0,"notices":notices}))
}

fn read(s: &mut Session, _: &Value) -> Result<Value> {
    let selected = selected_segments(s)?;
    let (lid, uid, i) = selected.first().copied().ok_or_else(|| bad("keys.currentRead", "Select adjacent keyframes."))?;
    let prop =
        s.active_comp().and_then(|c| c.layer(lid)).and_then(|l| l.props.find(uid)).ok_or_else(|| bad("keys.currentRead", "Selected property is missing."))?;
    let average = averages(prop, i)
        .and_then(|v| v.into_iter().find(|n| n.abs() > 1e-12))
        .ok_or_else(|| bad("keys.currentRead", "A constant interval has no normalized easing curve."))?;
    // Prefer the first nonconstant dimension. Different dimension curves cannot collapse to one.
    let speeds = averages(prop, i).ok_or_else(|| bad("keys.currentRead", "Invalid interval."))?;
    let dim = speeds.iter().position(|n| (*n - average).abs() <= f64::EPSILON).unwrap_or(0);
    let out = effectcraft_keyframe::side_ease(&prop.keys, i, dim, prop.spatial, true)
        .ok_or_else(|| bad("keys.currentRead", "Hold interpolation has no easing curve."))?;
    let incoming = i
        .checked_add(1)
        .and_then(|j| effectcraft_keyframe::side_ease(&prop.keys, j, dim, prop.spatial, false))
        .ok_or_else(|| bad("keys.currentRead", "Hold interpolation has no easing curve."))?;
    let curve = Curve([out.influence, out.speed / average * out.influence, 1.0 - incoming.influence, 1.0 - incoming.speed / average * incoming.influence]);
    curve.validate().map_err(|e| bad("keys.currentRead", e))?;
    Ok(json!({"curve": curve.0, "dimension": dim, "segments": selected.len()}))
}

pub fn specs() -> Vec<CommandSpec> {
    let mut specs = vec![
        cmd!("keys.currentApply", "Current: Apply Easing", [], None, "{curve: [x1,y1,x2,y2]}", has_keys, apply),
        query!("keys.currentRead", "Current: Read Easing", "{}", read),
        query!("current.presets", "Current: Curve Library", "{}", presets),
        cmd!("current.savePreset", "Current: Save Curve", [], None, "{name, curve: [x1,y1,x2,y2]}", super::always, save_preset),
        cmd!("current.deletePreset", "Current: Delete Saved Curve", [], None, "{name}", super::always, delete_preset),
    ];
    specs.extend(super::current_physics::specs());
    specs.extend(super::current_standard::specs());
    specs.extend(super::current_library::specs());
    specs
}

#[cfg(test)]
#[path = "current_tests.rs"]
mod tests;
