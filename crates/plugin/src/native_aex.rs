//! Experimental single-frame AEX adapter. Native ABI code stays in the external worker.
use crate::{EffectPlugin, PluginFrame, PluginManifest, PluginParams, register_plugin};
use effectcraft_effects::plugin::PluginParamKind;
use serde::Deserialize;
use serde_json::json;
use std::{
    fs,
    io::Read,
    os::windows::process::CommandExt,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Descriptor {
    experimental_aex: bool,
    worker: PathBuf,
    plugin: PathBuf,
    #[serde(default)]
    match_name: Option<String>,
    #[serde(default)]
    dll_dirs: Vec<PathBuf>,
    #[serde(default = "default_api")]
    host_api: String,
    #[serde(default)]
    render_mode: RenderMode,
    manifest: PluginManifest,
    /// Native 1-based parameter index for each manifest parameter.
    parameter_indices: Vec<usize>,
}
#[derive(Default, serde::Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RenderMode {
    #[default]
    Auto,
    Legacy,
    Smart,
}
fn default_api() -> String {
    "third_party".into()
}
struct NativeEffect {
    config: Descriptor,
    source: String,
    serial: Mutex<()>,
}

pub fn load(bytes: &[u8], source: &str) -> Result<&'static effectcraft_effects::EffectSpec, String> {
    if bytes.len() > 1024 * 1024 {
        return Err("AEX descriptor exceeds 1 MiB".into());
    }
    let mut config: Descriptor = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    config.worker = crate::bundled_worker(config.worker, "effectcraft-aex-probe.exe");
    if !config.manifest.name.ends_with(" [Adobe]") {
        config.manifest.name.push_str(" [Adobe]");
    }
    config.validate()?;
    register_plugin(Arc::new(NativeEffect { config, source: source.into(), serial: Mutex::new(()) }))
}
impl Descriptor {
    fn validate(&self) -> Result<(), String> {
        if !self.experimental_aex {
            return Err("AEX descriptor must explicitly opt in with experimental_aex: true; native plugins execute with your OS access and Boris output is not verified".into());
        }
        self.manifest.validate()?;
        for path in [&self.worker, &self.plugin] {
            if !path.is_absolute() || !path.is_file() {
                return Err("AEX worker and plugin must be existing absolute file paths".into());
            }
        }
        if self.plugin.extension().is_none_or(|e| !e.eq_ignore_ascii_case("aex")) {
            return Err("Native plugin must have an .aex extension".into());
        }
        if self.dll_dirs.len() > 16 || self.dll_dirs.iter().any(|p| !p.is_absolute() || !p.is_dir()) {
            return Err("Invalid AEX DLL directories".into());
        }
        if !matches!(self.host_api.as_str(), "third_party" | "after_effects") {
            return Err("Unknown AEX host API profile".into());
        }
        let mut seen = std::collections::HashSet::new();
        if self.parameter_indices.len() != self.manifest.params.len() || self.parameter_indices.iter().any(|i| *i == 0 || *i > 1024 || !seen.insert(*i)) {
            return Err("AEX parameter indices must be unique, in 1..1024, and match the manifest parameters".into());
        }
        if self.manifest.params.iter().any(|p| {
            !matches!(
                p.kind,
                PluginParamKind::Slider { .. }
                    | PluginParamKind::Checkbox { .. }
                    | PluginParamKind::Popup { .. }
                    | PluginParamKind::Color { .. }
                    | PluginParamKind::Point { .. }
                    | PluginParamKind::Angle { .. }
            )
        }) {
            return Err("Unsupported AEX parameter kind".into());
        }
        Ok(())
    }
}
impl EffectPlugin for NativeEffect {
    fn supports_float(&self) -> bool {
        false
    }
    fn manifest(&self) -> &PluginManifest {
        &self.config.manifest
    }
    fn source(&self) -> String {
        self.source.clone()
    }
    fn render(&self, frame: &mut PluginFrame, params: &PluginParams, time: f64) -> Result<(), String> {
        self.render_at_rate(frame, params, time, 25.0, None)
    }
    fn render_with_context(&self, frame: &mut PluginFrame, params: &PluginParams, ctx: &effectcraft_effects::EffectCtx) -> Result<(), String> {
        self.render_at_rate(frame, params, ctx.time, if ctx.env.frame_rate == 0.0 { 25.0 } else { ctx.env.frame_rate }, Some(ctx.layer_size))
    }
}
fn frame_rate(fps: f64) -> Result<serde_json::Value, String> {
    if !fps.is_finite() || !(1.0..=240.0).contains(&fps) {
        return Err("Native frame rate must be within 1..240 fps".into());
    }
    for num in [24000u32, 30000, 60000, 120000] {
        if (fps - f64::from(num) / 1001.0).abs() < 0.000000001 {
            return Ok(json!({"num":num,"den":1001}));
        }
    }
    let (mut prev_num, mut num, mut prev_den, mut den) = (0u64, 1u64, 1u64, 0u64);
    let mut x = fps;
    for _ in 0..24 {
        let whole = x.floor();
        if whole > 240000.0 {
            break;
        }
        let a = whole as u64;
        let Some(next_num) = a.checked_mul(num).and_then(|n| n.checked_add(prev_num)) else {
            break;
        };
        let Some(next_den) = a.checked_mul(den).and_then(|n| n.checked_add(prev_den)) else {
            break;
        };
        if next_num > 240000 || next_den > 100000 || next_den == 0 {
            break;
        }
        if (next_num as f64 / next_den as f64 - fps).abs() < 0.000000001 {
            return Ok(json!({"num":next_num,"den":next_den}));
        }
        (prev_num, num, prev_den, den) = (num, next_num, den, next_den);
        let fraction = x - whole;
        if fraction == 0.0 {
            break;
        }
        x = 1.0 / fraction;
    }
    Err("Frame rate cannot be represented by the native clock without changing it".into())
}
impl NativeEffect {
    fn render_at_rate(&self, frame: &mut PluginFrame, params: &PluginParams, time: f64, fps: f64, layer_size: Option<[f64; 2]>) -> Result<(), String> {
        let rate = frame_rate(fps)?;
        let _serial = self.serial.lock().map_err(|_| "AEX render lock unavailable")?;
        let downsample = preview_ratio(frame.scale)?;
        let logical = layer_size.unwrap_or([f64::from(frame.width) / frame.scale, f64::from(frame.height) / frame.scale]);
        if logical.iter().any(|v| !v.is_finite() || !(1.0..=16384.0).contains(v)) {
            return Err("AEX logical layer dimensions must be within 1..16384".into());
        }
        let layer_size = logical.map(|v| v.round() as u32);
        let count = frame.width.checked_mul(frame.height).filter(|n| *n > 0 && *n <= 16 * 1024 * 1024).ok_or("AEX frame exceeds 16 million pixels")?;
        if frame.width > 16384 || frame.height > 16384 || frame.pixels.len() != count as usize {
            return Err("Invalid AEX frame dimensions".into());
        }
        if !time.is_finite() || !(0.0..=36000.0).contains(&time) {
            return Err("AEX time outside supported range".into());
        }
        if params.flat.len() != self.config.manifest.flat_len() {
            return Err("AEX parameter count mismatch".into());
        }
        let mut image = image::RgbaImage::new(frame.width, frame.height);
        for (src, dst) in frame.pixels.iter().zip(image.pixels_mut()) {
            if src.iter().any(|c| !c.is_finite() || *c < 0.0 || *c > 1.0) {
                return Err("Experimental AEX bridge supports SDR 8-bit frames only".into());
            }
            let alpha = src.get(3).copied().ok_or("Invalid pixel")?;
            if src.iter().take(3).any(|c| *c > alpha + f32::EPSILON) {
                return Err("Experimental AEX bridge cannot preserve overbright premultiplied colors".into());
            }
            for (channel, value) in src.iter().enumerate() {
                let straight = if channel == 3 {
                    *value
                } else if alpha > 0.0 {
                    *value / alpha
                } else {
                    0.0
                };
                let dest = dst.0.get_mut(channel).ok_or("Invalid PNG channel")?;
                *dest = (straight.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
        let dir = tempfile::Builder::new().prefix("effectcraft-aex-").tempdir().map_err(|e| e.to_string())?;
        let input = dir.path().join("input.png");
        let output = dir.path().join("output.png");
        image.save(&input).map_err(|e| e.to_string())?;
        let mut overrides = serde_json::Map::new();
        let mut at = 0usize;
        for (index, param) in self.config.parameter_indices.iter().zip(&self.config.manifest.params) {
            let end = at.checked_add(param.kind.width()).ok_or("AEX parameter offset overflow")?;
            let values = params.flat.get(at..end).ok_or("AEX parameter span missing")?;
            at = end;
            if values.iter().any(|v| !v.is_finite()) {
                return Err("AEX parameter is not finite".into());
            }
            let scalar = values.first().copied().ok_or("AEX scalar missing")?;
            let value = match param.kind {
                PluginParamKind::Popup { .. } => json!(scalar + 1.0),
                PluginParamKind::Point { .. } | PluginParamKind::Color { .. } => json!(values),
                _ => json!(scalar),
            };
            overrides.insert(index.to_string(), value);
        }
        let job = dir.path().join("job.json");
        fs::write(
            &job,
            serde_json::to_vec(&json!({"plugin":self.config.plugin,"dll_dirs":self.config.dll_dirs,
            "match_name":self.config.match_name,"host_api":self.config.host_api,"render_mode":self.config.render_mode,"input":input,"output":output,"time_seconds":time,"frame_rate":rate,"downsample":downsample,"layer_size":layer_size,"parameters":overrides}))
            .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let log = dir.path().join("report.json");
        let log_file = fs::File::create(&log).map_err(|e| e.to_string())?;
        let mut child = Command::new(&self.config.worker)
            .arg("render")
            .arg(&job)
            .creation_flags(0x08000000)
            .stdin(Stdio::null())
            .stdout(log_file)
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Cannot start AEX host: {e}"))?;
        let start = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if start.elapsed() < Duration::from_secs(70) => std::thread::sleep(Duration::from_millis(10)),
                result => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(match result {
                        Err(e) => format!("AEX worker wait failed: {e}"),
                        _ => "AEX worker timed out".into(),
                    });
                }
            }
        };
        let mut bytes = Vec::new();
        fs::File::open(&log).map_err(|e| e.to_string())?.take(8 * 1024 * 1024 + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err("AEX report exceeds size limit".into());
        }
        let report: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| format!("Invalid AEX host report: {e}"))?;
        if !status.success() || report["protocol"] != 1 || report["status"] != "frame_written" || report["frameWritten"] != true {
            return Err(format!("AEX rendering failed: {}", report["status"]));
        }
        if let Some(message) = report["events"].as_array().and_then(|events| events.iter().find(|e| e["errorMessage"] == true)) {
            return Err(format!("AEX plugin reported: {}", message["message"]));
        }
        let reader = image::ImageReader::open(&output).map_err(|e| e.to_string())?.with_guessed_format().map_err(|e| e.to_string())?;
        if reader.format() != Some(image::ImageFormat::Png) || reader.into_dimensions().map_err(|e| e.to_string())? != (frame.width, frame.height) {
            return Err("AEX output format or dimensions changed".into());
        }
        let mut reader = image::ImageReader::open(&output).map_err(|e| e.to_string())?.with_guessed_format().map_err(|e| e.to_string())?;
        let mut limits = image::Limits::default();
        limits.max_alloc = Some(256 * 1024 * 1024);
        reader.limits(limits);
        let rendered = reader.decode().map_err(|e| e.to_string())?.to_rgba8();
        for (dst, src) in frame.pixels.iter_mut().zip(rendered.pixels()) {
            let [r, g, b, a] = src.0;
            let alpha = f32::from(a) / 255.0;
            *dst = [f32::from(r) / 255.0 * alpha, f32::from(g) / 255.0 * alpha, f32::from(b) / 255.0 * alpha, alpha];
        }
        Ok(())
    }
}

fn preview_ratio(scale: f64) -> Result<serde_json::Value, String> {
    if !scale.is_finite() || !(1.0 / 16384.0..=1.0).contains(&scale) {
        return Err("AEX preview scale must be within 1/16384..1".into());
    }
    for den in 1..=65536u32 {
        let num = (scale * f64::from(den)).round() as u32;
        if num > 0 && (f64::from(num) / f64::from(den) - scale).abs() < 1e-10 {
            return Ok(json!({"num":num,"den":den}));
        }
    }
    Err("AEX preview scale has no bounded rational representation".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use effectcraft_effects::{Buf, EffectCtx, EffectEnv, Image};
    use effectcraft_project::build::Ids;
    #[test]
    fn rates_preserve_ntsc_and_custom_rationals() {
        assert_eq!(frame_rate(30000.0 / 1001.0).unwrap(), json!({"num":30000,"den":1001}));
        assert_eq!(frame_rate(100.0 / 3.0).unwrap(), json!({"num":100,"den":3}));
        assert_eq!(frame_rate(25.0).unwrap(), json!({"num":25,"den":1}));
        assert!(frame_rate(f64::NAN).is_err());
    }
    #[test]
    fn preview_ratios_are_bounded_and_preserve_standard_resolutions() {
        for den in [1u32, 2, 3, 4, 5, 8, 16] {
            assert_eq!(preview_ratio(1.0 / f64::from(den)).unwrap(), json!({"num":1,"den":den}));
        }
        for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY, 2.0] {
            assert!(preview_ratio(invalid).is_err());
        }
    }
    #[test]
    fn edition_names_resolve_without_changing_saved_ids() {
        struct LabelEffect(PluginManifest);
        impl EffectPlugin for LabelEffect {
            fn manifest(&self) -> &PluginManifest {
                &self.0
            }
            fn render(&self, _: &mut PluginFrame, _: &PluginParams, _: f64) -> Result<(), String> {
                Ok(())
            }
        }
        let make = |id: &str, name: &str| {
            register_plugin(Arc::new(LabelEffect(serde_json::from_value(json!({"api":1,"id":id,"name":name,"category":"Edition Test"})).unwrap()))).unwrap()
        };
        let adobe = make("org.test.edition.adobe", "Edition Collision [Adobe]");
        let ofx = make("ofx.org.test.edition", "Edition Collision [OFX]");
        let previous = effectcraft_effects::plugin::adobe_plugins_preferred();
        effectcraft_effects::plugin::prefer_adobe_plugins(false);
        assert_eq!(effectcraft_effects::lookup("Edition Collision").unwrap().id, ofx.id);
        effectcraft_effects::plugin::prefer_adobe_plugins(true);
        assert_eq!(effectcraft_effects::lookup("Edition Collision").unwrap().id, adobe.id);
        effectcraft_effects::plugin::prefer_adobe_plugins(previous);
        assert_eq!(effectcraft_effects::lookup("Edition Collision [Adobe]").unwrap().id, adobe.id);
        assert_eq!(effectcraft_effects::lookup(adobe.id).unwrap().id, adobe.id);
    }
    #[test]
    #[ignore = "requires original native fixture and worker"]
    fn downsampled_smart_and_legacy_frames_keep_layer_size_and_point_coordinates() {
        let plugin = PathBuf::from(std::env::var("EFFECTCRAFT_AEX_TEST_PLUGIN").unwrap());
        for mode in ["smart", "legacy"] {
            let id = format!("org.test.preview.{mode}");
            let mut config = descriptor(&id, plugin.clone());
            config["render_mode"] = json!(mode);
            config["parameter_indices"] = json!([7, 4]);
            config["manifest"]["params"] = json!([
                {"id":"preview","name":"Encode Preview","type":"checkbox","default":true},
                {"id":"point","name":"Point","type":"point","default":[0.25,0.5]}]);
            let spec = load(&serde_json::to_vec(&config).unwrap(), "preview.ecaex.json").unwrap();
            for den in [1u32, 2, 3, 4, 5, 8] {
                let scale = 1.0 / f64::from(den);
                let mut next = 1;
                let group = effectcraft_effects::instantiate(spec, &mut Ids(&mut next), spec.name, [48.0, 24.0]);
                let params = effectcraft_effects::flatten_params(&group, &mut |p| p.value.clone());
                let ctx = EffectCtx { params: &params, time: 0.0, layer_size: [48.0, 24.0], seed: 1, adjustment: false, env: EffectEnv::default() };
                let buf = Buf { img: Image::filled(48u32.div_ceil(den), 24u32.div_ceil(den), [0.0, 0.0, 0.0, 1.0]), offset: [0.0; 2], scale };
                let result = (spec.render)(&ctx, buf).img;
                let pixel = result.get(0, 0);
                let expected = [48.0 / 255.0, (den * 16 + 1) as f32 / 255.0, (24.0 / f64::from(den)).floor() as f32 / 255.0, 1.0];
                for (actual, expected) in pixel.into_iter().zip(expected) {
                    assert!((actual - expected).abs() < 1.0 / 255.0, "{mode} 1/{den}: {pixel:?}");
                }
            }
        }
    }
    #[test]
    fn render_modes_are_explicit_and_unknown_modes_are_rejected() {
        for mode in ["auto", "smart", "legacy"] {
            let value = json!(mode);
            let parsed: RenderMode = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(parsed).unwrap(), value);
        }
        assert!(serde_json::from_value::<RenderMode>(json!("gpu")).is_err());
    }
    #[test]
    fn catalogs_beyond_sixty_four_effects_route_to_the_correct_plugin() {
        struct ColorEffect {
            manifest: PluginManifest,
            red: f32,
        }
        impl EffectPlugin for ColorEffect {
            fn manifest(&self) -> &PluginManifest {
                &self.manifest
            }
            fn render(&self, frame: &mut PluginFrame, _: &PluginParams, _: f64) -> Result<(), String> {
                for p in frame.pixels.iter_mut() {
                    *p = [self.red, 0.0, 0.0, 1.0];
                }
                Ok(())
            }
        }
        let mut last = None;
        for i in 0..80 {
            let manifest =
                serde_json::from_value(json!({"api":1,"id":format!("org.test.capacity.effect{i}"),"name":format!("Capacity {i}"),"category":"Capacity Test"}))
                    .unwrap();
            last = Some(register_plugin(Arc::new(ColorEffect { manifest, red: i as f32 / 100.0 })).unwrap());
        }
        let pixel = render(last.unwrap(), 0.0).get(1, 1);
        assert!((pixel[0] - 0.79).abs() < 0.000001, "{pixel:?}");
    }
    fn descriptor(id: &str, plugin: PathBuf) -> serde_json::Value {
        json!({"experimental_aex":true,"worker":std::env::var("EFFECTCRAFT_AEX_TEST_WORKER").unwrap(),
            "plugin":plugin,"parameter_indices":[1],"manifest":{"api":1,"id":id,
            "name":"Experimental Native Gain","category":"Experimental AEX","params":[
                {"id":"gain","name":"Gain","type":"slider","default":2,"min":0,"max":4}]}})
    }
    fn render(spec: &effectcraft_effects::EffectSpec, gain: f64) -> Image {
        let mut next = 1;
        let group = effectcraft_effects::instantiate(spec, &mut Ids(&mut next), spec.name, [4.0, 2.0]);
        let mut params = effectcraft_effects::flatten_params(&group, &mut |p| p.value.clone());
        params.values.insert("gain".into(), effectcraft_keyframe::Value::Scalar(gain));
        let ctx = EffectCtx { params: &params, time: 0.0, layer_size: [4.0, 2.0], seed: 1, adjustment: false, env: EffectEnv::default() };
        let buf = Buf { img: Image::filled(4, 2, [40.0 / 255.0, 80.0 / 255.0, 120.0 / 255.0, 1.0]), offset: [0.0; 2], scale: 1.0 };
        (spec.render)(&ctx, buf).img
    }
    #[test]
    #[ignore = "Requires native-host fixture paths in EFFECTCRAFT_AEX_TEST_WORKER and EFFECTCRAFT_AEX_TEST_PLUGIN"]
    fn native_aex_registers_renders_and_changes_parameters() {
        let plugin = PathBuf::from(std::env::var("EFFECTCRAFT_AEX_TEST_PLUGIN").unwrap());
        for mode in ["auto", "smart", "legacy"] {
            let id = format!("org.test.native_gain_{mode}");
            let mut config = descriptor(&id, plugin.clone());
            config["render_mode"] = json!(mode);
            let bytes = serde_json::to_vec(&config).unwrap();
            let loaded = crate::loader(&bytes, "gain.ecaex.json").unwrap();
            assert_eq!(loaded["experimentalNative"], true);
            let spec = effectcraft_effects::lookup(&id).unwrap();
            assert!(!spec.float && !spec.gpu);
            for gain in [2.0, 0.5] {
                let pixel = render(spec, gain).get(1, 1);
                for (actual, expected) in pixel.into_iter().zip([40.0 * gain / 255.0, 80.0 * gain / 255.0, 120.0 * gain / 255.0, 1.0]) {
                    assert!((f64::from(actual) - expected).abs() < 1.0 / 255.0, "{mode}: {pixel:?}");
                }
            }
        }
    }
    #[test]
    #[ignore = "Requires native-host worker in EFFECTCRAFT_AEX_TEST_WORKER"]
    fn failed_native_load_preserves_effectcraft_frame() {
        let dir = tempfile::tempdir().unwrap();
        let plugin = dir.path().join("invalid.aex");
        fs::write(&plugin, b"invalid original test DLL").unwrap();
        let mut config = descriptor("org.test.native_invalid", plugin);
        config["experimental_aex"] = json!(false);
        assert!(load(&serde_json::to_vec(&config).unwrap(), "invalid.ecaex.json").is_err());
        config["experimental_aex"] = json!(true);
        let spec = load(&serde_json::to_vec(&config).unwrap(), "invalid.ecaex.json").unwrap();
        assert_eq!(render(spec, 2.0).get(1, 1), [40.0 / 255.0, 80.0 / 255.0, 120.0 / 255.0, 1.0]);
    }
}
