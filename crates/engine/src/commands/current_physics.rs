//! Original normalized motion functions and bounded, undoable keyframe baking.
//! No third-party presets, expressions or curve-editor implementations are used.

use std::collections::BTreeMap;
use std::sync::{Arc, Weak};

use effectcraft_keyframe::{Interp, Keyframe, Value as KeyValue};
use effectcraft_project::{ItemId, LayerId, Project, Property, Uid};
use effectcraft_time::{TICKS_PER_SECOND, Tick};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{CommandSpec, bad, has_keys};
use crate::{EngineError, KeyRef, Result, Session, cmd, query};

const APPLY: &str = "keys.currentPhysics";
const FILE: &str = "current-physics-curves.json";
const MAX_NEW_KEYS: usize = 32_768;
const MAX_BASE_KEYS: usize = 65_536;
const MAX_INTERVAL_STEPS: usize = 8_192;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PhysicsMode {
    #[default]
    Spring,
    Elastic,
    Bounce,
}

/// Frequency is oscillations per selected interval, damping is normalized decay,
/// and amplitude blends away from a straight ramp. Bounce stays between the endpoints.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Physics {
    pub mode: PhysicsMode,
    pub frequency: f64,
    pub damping: f64,
    pub amplitude: f64,
}

impl Default for Physics {
    fn default() -> Self {
        Self { mode: PhysicsMode::Spring, frequency: 3.0, damping: 6.0, amplitude: 1.0 }
    }
}

impl Physics {
    pub fn validate(self) -> std::result::Result<Self, String> {
        if !self.frequency.is_finite() || !(0.5..=12.0).contains(&self.frequency) {
            return Err("Frequency must be between 0.5 and 12 oscillations per interval.".into());
        }
        if !self.damping.is_finite() || !(0.1..=20.0).contains(&self.damping) {
            return Err("Damping must be between 0.1 and 20.".into());
        }
        if !self.amplitude.is_finite() || !(0.0..=3.0).contains(&self.amplitude) {
            return Err("Amplitude must be between 0 and 3.".into());
        }
        Ok(self)
    }

    /// Finite normalized preview sample. Invalid persisted settings fall back to a ramp;
    /// the editing command rejects them before making any project changes.
    pub fn sample(self, time: f64) -> f64 {
        if !time.is_finite() || time <= 0.0 {
            return 0.0;
        }
        if time >= 1.0 {
            return 1.0;
        }
        if self.validate().is_err() {
            return time;
        }
        let omega = std::f64::consts::TAU * self.frequency;
        let response = |t: f64| {
            let wave = match self.mode {
                // Unit-step response of a damped harmonic oscillator, with zero initial velocity.
                PhysicsMode::Spring => (omega * t).cos() + self.damping / omega * (omega * t).sin(),
                // A decaying cosine has a different initial velocity and sharper recoil.
                PhysicsMode::Elastic => (omega * t).cos(),
                // Absolute cosine reflects each impact back inside the target boundary.
                PhysicsMode::Bounce => (omega * t).cos().abs(),
            };
            1.0 - (-self.damping * t).exp() * wave
        };
        // Endpoint normalization retains exact source boundary values, even at low damping.
        let normal = response(1.0);
        if !normal.is_finite() || normal.abs() < 1e-9 {
            return time;
        }
        let value = time + (response(time) / normal - time) * self.amplitude;
        if !value.is_finite() {
            return time;
        }
        if self.mode == PhysicsMode::Bounce { value.clamp(0.0, 1.0) } else { value }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicsPreset {
    pub name: String,
    pub physics: Physics,
}

fn builtin_presets() -> Vec<PhysicsPreset> {
    [
        ("Spring", Physics::default()),
        ("Elastic", Physics { mode: PhysicsMode::Elastic, frequency: 4.0, damping: 7.0, amplitude: 1.0 }),
        ("Bounce", Physics { mode: PhysicsMode::Bounce, frequency: 3.0, damping: 3.0, amplitude: 1.0 }),
    ]
    .into_iter()
    .map(|(name, physics)| PhysicsPreset { name: name.into(), physics })
    .collect()
}

fn user_presets(s: &Session) -> Result<Vec<PhysicsPreset>> {
    let Some(store) = &s.config else { return Ok(Vec::new()) };
    let text = store.read_bounded(FILE, 256 * 1024).map_err(|e| bad("current.physicsPresets", format!("Cannot read motion curves: {e}")))?;
    let Some(text) = text else { return Ok(Vec::new()) };
    let curves: Vec<PhysicsPreset> = serde_json::from_str(&text).map_err(|e| bad("current.physicsPresets", format!("Invalid motion library: {e}")))?;
    if curves.len() > 128 || curves.iter().any(|p| p.name.trim().is_empty() || p.name.chars().count() > 80 || p.physics.validate().is_err()) {
        return Err(bad("current.physicsPresets", "The motion library contains invalid curves or exceeds 128 entries."));
    }
    Ok(curves)
}

fn presets(s: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!({"builtins": builtin_presets(), "user": user_presets(s)?}))
}

fn write_presets(s: &Session, curves: &[PhysicsPreset]) -> Result<()> {
    let store = s.config.as_ref().ok_or_else(|| bad("current.savePhysicsPreset", "This session has no settings store."))?;
    let text = serde_json::to_string_pretty(curves).map_err(|e| bad("current.savePhysicsPreset", e.to_string()))?;
    store.write(FILE, &text).map_err(|e| bad("current.savePhysicsPreset", format!("Cannot save motion curves: {e}")))
}

fn save_preset(s: &mut Session, p: &Value) -> Result<Value> {
    let name = p
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|n| !n.is_empty() && n.chars().count() <= 80)
        .ok_or_else(|| bad("current.savePhysicsPreset", "Name the motion curve using 1–80 characters."))?;
    let physics: Physics =
        serde_json::from_value(p.get("physics").cloned().ok_or_else(|| bad("current.savePhysicsPreset", "Pass the motion settings in `physics`."))?)
            .map_err(|e| bad("current.savePhysicsPreset", e.to_string()))?;
    let physics = physics.validate().map_err(|e| bad("current.savePhysicsPreset", e))?;
    let mut user = user_presets(s)?;
    if let Some(existing) = user.iter_mut().find(|p| p.name == name) {
        existing.physics = physics;
    } else {
        if user.len() >= 128 {
            return Err(bad("current.savePhysicsPreset", "The library is full. Delete a saved motion curve first."));
        }
        user.push(PhysicsPreset { name: name.into(), physics });
    }
    write_presets(s, &user)?;
    Ok(json!({"saved": name, "user": user}))
}

fn delete_preset(s: &mut Session, p: &Value) -> Result<Value> {
    let name = p.get("name").and_then(Value::as_str).ok_or_else(|| bad("current.deletePhysicsPreset", "Choose a saved motion curve by `name`."))?;
    let mut user = user_presets(s)?;
    let before = user.len();
    user.retain(|p| p.name != name);
    if user.len() == before {
        return Err(bad("current.deletePhysicsPreset", "The saved motion curve was not found."));
    }
    write_presets(s, &user)?;
    Ok(json!({"deleted": name, "user": user}))
}

/// Only a command-owned token can replace a previous bake. The source keys are bounded,
/// and the weak project pointer cannot retain a whole closed document in memory.
#[derive(Clone)]
pub struct Gesture {
    token: u64,
    comp: ItemId,
    expected: Weak<Project>,
    selection: Vec<KeyRef>,
    source: BTreeMap<(LayerId, Uid), Property>,
    segments: Vec<(LayerId, Uid, usize)>,
    merge: String,
}

fn new_gesture(s: &Session) -> Result<Gesture> {
    let segments = super::current::selected_segments(s)?;
    if segments.len() > MAX_NEW_KEYS {
        return Err(bad(APPLY, "Select fewer intervals; one bake is limited to 32,768 new keys."));
    }
    let comp = s.active_comp().ok_or(EngineError::NoComp)?;
    let mut source = BTreeMap::new();
    let mut base_keys = 0usize;
    for (lid, uid, _) in &segments {
        if source.contains_key(&(*lid, *uid)) {
            continue;
        }
        let prop = comp.layer(*lid).and_then(|l| l.props.find(*uid)).ok_or_else(|| bad(APPLY, "The selected property is missing."))?;
        if prop.spatial {
            return Err(bad(APPLY, "Separate Position dimensions before baking motion; spatial motion paths are not replaced."));
        }
        if prop.has_expression() {
            return Err(bad(APPLY, "Disable the property's expression before baking motion, so it can display its keyframes."));
        }
        if !matches!(prop.value, KeyValue::Scalar(_) | KeyValue::Vec2(_) | KeyValue::Vec3(_)) {
            return Err(bad(
                APPLY,
                "Motion baking supports scalar and non-spatial numeric vector properties. Shapes, colors and discrete values need another workflow.",
            ));
        }
        base_keys = base_keys.checked_add(prop.keys.len()).ok_or_else(|| bad(APPLY, "The selected keyframe count is too large."))?;
        if base_keys > MAX_BASE_KEYS {
            return Err(bad(APPLY, "Select properties containing at most 65,536 existing keyframes per bake."));
        }
        source.insert((*lid, *uid), prop.clone());
    }
    let token = s.revision.checked_add(1).ok_or_else(|| bad(APPLY, "Restart this editing session before baking more motion."))?;
    Ok(Gesture {
        token,
        comp: s.active_comp_id().ok_or(EngineError::NoComp)?,
        expected: Arc::downgrade(&s.project),
        selection: s.state.selected_keys.clone(),
        source,
        segments,
        merge: format!("current.physics.{token}"),
    })
}

fn apply(s: &mut Session, p: &Value) -> Result<Value> {
    let physics: Physics = serde_json::from_value(p.clone()).map_err(|e| bad(APPLY, format!("Invalid motion settings: {e}")))?;
    let physics = physics.validate().map_err(|e| bad(APPLY, e))?;
    let mut result = bake(s, p, |time| physics.sample(time), "Current: Apply Motion")?;
    result["physics"] = json!(physics);
    Ok(result)
}

pub(super) fn bake(s: &mut Session, p: &Value, sample: impl Fn(f64) -> f64, label: &str) -> Result<Value> {
    let density = match p.get("density") {
        None => 2,
        Some(v) => v.as_u64().filter(|n| (1..=8).contains(n)).ok_or_else(|| bad(APPLY, "Sample density must be an integer between 1 and 8."))?,
    };
    let mut gesture = if let Some(token) = p.get("token") {
        let token = token.as_u64().ok_or_else(|| bad(APPLY, "The replacement token must be an unsigned integer."))?;
        let old = s
            .current_physics
            .as_ref()
            .filter(|g| g.token == token)
            .ok_or_else(|| bad(APPLY, "Motion preview expired. Select adjacent source keys and apply again."))?;
        let current = old.expected.upgrade().is_some_and(|project| Arc::ptr_eq(&project, &s.project));
        if !current
            || s.active_comp_id() != Some(old.comp)
            || s.state.selected_keys != old.selection
            || s.history.merge_key.as_deref() != Some(old.merge.as_str())
        {
            return Err(bad(APPLY, "Motion preview expired after a project, selection or history change. Select adjacent source keys and apply again."));
        }
        old.clone()
    } else {
        new_gesture(s)?
    };
    let comp = s.project.comp(gesture.comp).ok_or(EngineError::NoComp)?;
    let fps = comp.frame_rate.as_f64();
    if !fps.is_finite() || !(0.001..=1000.0).contains(&fps) {
        return Err(bad(APPLY, "The composition frame rate is outside the supported motion-baking range."));
    }
    let mut plan: BTreeMap<_, _> = gesture.source.iter().map(|(id, prop)| (*id, prop.keys.clone())).collect();
    let mut generated = 0usize;
    for &(lid, uid, index) in &gesture.segments {
        let layer = comp.layer(lid).ok_or_else(|| bad(APPLY, "The selected layer is missing."))?;
        if layer.switches.locked {
            return Err(bad(APPLY, "Unlock the selected layer before baking motion."));
        }
        let stretch = layer.stretch.abs() / 100.0;
        if !stretch.is_finite() || !(0.00001..=1000.0).contains(&stretch) {
            return Err(bad(APPLY, "The layer time stretch is outside the supported motion-baking range."));
        }
        let prop = gesture.source.get(&(lid, uid)).ok_or_else(|| bad(APPLY, "The source property is missing."))?;
        let next = index.checked_add(1).ok_or_else(|| bad(APPLY, "The interval index is invalid."))?;
        let a = prop.keys.get(index).ok_or_else(|| bad(APPLY, "The first source key is missing."))?;
        let b = prop.keys.get(next).ok_or_else(|| bad(APPLY, "The last source key is missing."))?;
        if std::mem::discriminant(&a.value) != std::mem::discriminant(&b.value)
            || a.value.components().iter().chain(b.value.components().iter()).any(|x| !x.is_finite())
        {
            return Err(bad(APPLY, "Source keys must have matching finite numeric values."));
        }
        let ticks = i128::from(b.time.0) - i128::from(a.time.0);
        if ticks < 2 {
            return Err(bad(APPLY, "The source interval is too short to insert an intermediate keyframe."));
        }
        let count = (ticks as f64 / TICKS_PER_SECOND as f64 * stretch * fps * density as f64).ceil().max(2.0);
        if !count.is_finite() || count > MAX_INTERVAL_STEPS as f64 {
            return Err(bad(APPLY, "This interval needs more than 8,192 samples. Reduce density or use a shorter interval."));
        }
        let steps = (count as usize).min(usize::try_from(ticks).unwrap_or(usize::MAX));
        generated = generated
            .checked_add(steps.saturating_sub(1))
            .filter(|n| *n <= MAX_NEW_KEYS)
            .ok_or_else(|| bad(APPLY, "One motion bake is limited to 32,768 new keys. Select fewer intervals or reduce density."))?;
        let keys = plan.get_mut(&(lid, uid)).ok_or_else(|| bad(APPLY, "The output property is missing."))?;
        // Freeze both sides before disabling auto tangents. Untouched neighbouring intervals
        // retain their original evaluated shape when the first/last keys were auto-Bezier.
        for j in [index, next] {
            if let Some(key) = keys.get_mut(j) {
                if key.auto_bezier {
                    key.in_ease = (0..key.value.dims()).map(|d| effectcraft_keyframe::side_ease(&prop.keys, j, d, false, false).unwrap_or_default()).collect();
                    key.out_ease = (0..key.value.dims()).map(|d| effectcraft_keyframe::side_ease(&prop.keys, j, d, false, true).unwrap_or_default()).collect();
                }
                key.auto_bezier = false;
                key.continuous = false;
            }
        }
        if let Some(key) = keys.get_mut(index) {
            key.out_interp = Interp::Linear;
        }
        if let Some(key) = keys.get_mut(next) {
            key.in_interp = Interp::Linear;
        }
        for i in 1..steps {
            let relative = ticks
                .checked_mul(i as i128)
                .and_then(|n| n.checked_div(steps as i128))
                .ok_or_else(|| bad(APPLY, "A generated sample time exceeds the supported range."))?;
            let absolute = i128::from(a.time.0).checked_add(relative).ok_or_else(|| bad(APPLY, "A generated sample time exceeds the supported range."))?;
            let time = Tick(i64::try_from(absolute).map_err(|_| bad(APPLY, "A generated sample time exceeds the supported range."))?);
            let progress = sample(relative as f64 / ticks as f64);
            let value = a.value.lerp(&b.value, progress);
            if value.components().iter().any(|v| !v.is_finite()) {
                return Err(bad(APPLY, "The motion amplitude exceeds the property's finite numeric range."));
            }
            keys.push(Keyframe::new(time, value));
        }
    }
    for keys in plan.values_mut() {
        keys.sort_by_key(|k| k.time);
    }
    let count = gesture.segments.len();
    let cid = gesture.comp;
    s.edit(label, Some(&gesture.merge), |project, _| {
        let comp = project.comp_mut(cid).ok_or(EngineError::NoComp)?;
        for ((lid, uid), keys) in plan {
            let prop = comp.layer_mut(lid).and_then(|l| l.props.find_mut(uid)).ok_or_else(|| bad(APPLY, "The output property is missing."))?;
            prop.keys = keys;
        }
        Ok(())
    })?;
    gesture.expected = Arc::downgrade(&s.project);
    let token = gesture.token;
    s.current_physics = Some(gesture);
    Ok(json!({"segments": count, "generatedKeys": generated, "token": token, "density": density}))
}

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!("keys.currentPhysics", "Current: Apply Motion", [], None, "{mode,frequency,damping,amplitude,density?,token?}", has_keys, apply),
        query!("current.physicsPresets", "Current: Motion Library", "{}", presets),
        cmd!(
            "current.savePhysicsPreset",
            "Current: Save Motion Curve",
            [],
            None,
            "{name,physics:{mode,frequency,damping,amplitude}}",
            super::always,
            save_preset
        ),
        cmd!("current.deletePhysicsPreset", "Current: Delete Motion Curve", [], None, "{name}", super::always, delete_preset),
    ]
}

#[cfg(test)]
#[path = "current_physics_tests.rs"]
mod tests;
