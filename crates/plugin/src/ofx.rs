//! Safe client for glumbis/ec-extension-ofx's public external-helper protocol v1.
//! Native OFX ABI code stays in the separately licensed helper. This adapter exposes API-1
//! controls; API-2 text/hidden/3D/layer controls keep their native defaults.
use crate::{EffectPlugin, PluginFrame, PluginManifest, PluginParams, register_plugin};
use effectcraft_effects::plugin::{PluginParam, PluginParamKind};
use effectcraft_effects::{Buf, EffectCtx};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{Arc, Mutex, OnceLock, mpsc},
    time::{Duration, Instant},
};

const MAX_LINE: u64 = 16 * 1024 * 1024;
const MAX_PIXELS: u32 = 16 * 1024 * 1024;
const MAX_DESCRIPTOR_BYTES: usize = 8 * 1024 * 1024;
const MAX_FRAME_STORE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_IDLE_HELPERS: usize = 2;

/// Enrich old registrations from data retained by an already-run helper. This never
/// starts a helper, opens a vendor binary or writes a profile/cache file.
pub(crate) fn enrich_profile(profile: &mut Value) -> Result<bool, String> {
    if profile["experimental_ofx"] != true {
        return Ok(false);
    }
    let needs_units = profile["parameterSemantics"].is_null();
    let needs_gpu = profile["gpu"].is_null();
    if !needs_units && !needs_gpu {
        return Ok(false);
    }
    let id = profile["remote"]["manifest"]["id"].as_str().ok_or("OFX profile has no native effect identity")?;
    let device = id.split('.').next().unwrap_or_default().to_ascii_lowercase();
    let reserved = matches!(device.as_str(), "con" | "prn" | "aux" | "nul")
        || device
            .strip_prefix("com")
            .or_else(|| device.strip_prefix("lpt"))
            .is_some_and(|n| n.len() == 1 && n.as_bytes().first().is_some_and(|b| (b'1'..=b'9').contains(b)));
    if id.is_empty()
        || id.len() > 256
        || matches!(id, "." | "..")
        || reserved
        || !id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        || profile["manifest"]["id"].as_str().and_then(|s| s.strip_prefix("ofx.")) != Some(id)
    {
        return Err("OFX profile identities cannot select a safe metadata file".into());
    }
    let state = profile["state_dir"].as_str().map(Path::new).ok_or("OFX profile has no metadata state folder")?;
    if !state.is_absolute() || state.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err("OFX metadata state folder must be an absolute path without parent traversal".into());
    }
    let state = state.canonicalize().map_err(|e| format!("OFX metadata state folder is unavailable: {e}"))?;
    let directory = state.join("ofx-descriptors").canonicalize().map_err(|e| format!("OFX cached descriptors are unavailable: {e}"))?;
    if directory == state || !directory.starts_with(&state) {
        return Err("OFX metadata directory resolves outside its state folder".into());
    }
    let path = directory.join(format!("{id}.json")).canonicalize().map_err(|e| format!("OFX cached descriptor is unavailable: {e}"))?;
    if path.parent() != Some(directory.as_path()) {
        return Err("OFX metadata file resolves outside its descriptor folder".into());
    }
    let file = fs::File::open(path).map_err(|e| format!("Cannot open OFX cached descriptor: {e}"))?;
    let info = file.metadata().map_err(|e| format!("Cannot inspect OFX cached descriptor: {e}"))?;
    if !info.is_file() || info.len() > MAX_DESCRIPTOR_BYTES as u64 {
        return Err("OFX cached descriptor must be a regular file no larger than 8 MiB".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_DESCRIPTOR_BYTES as u64 + 1).read_to_end(&mut bytes).map_err(|e| format!("Cannot read OFX cached descriptor: {e}"))?;
    if bytes.len() > MAX_DESCRIPTOR_BYTES {
        return Err("OFX cached descriptor exceeds 8 MiB".into());
    }
    let metadata: Value = serde_json::from_slice(&bytes).map_err(|e| format!("Invalid OFX cached descriptor: {e}"))?;
    if metadata["id"].as_str() != Some(id) {
        return Err("OFX cached descriptor belongs to another effect".into());
    }
    let units = if needs_units {
        validate_semantics(&metadata["parameterSemantics"], &profile["remote"]["manifest"]["params"])?;
        Some(metadata["parameterSemantics"].clone())
    } else {
        None
    };
    let gpu = if needs_gpu { Some(validated_gpu(&metadata)?) } else { None };
    // Validate every requested field first; a failed enrichment leaves the profile intact.
    let object = profile.as_object_mut().ok_or("OFX registration must be an object")?;
    if let Some(units) = units {
        object.insert("parameterSemantics".into(), units);
    }
    if let Some(gpu) = gpu {
        object.insert("gpu".into(), gpu);
    }
    Ok(true)
}

fn validate_semantics(units: &Value, params: &Value) -> Result<(), String> {
    let fields = units.as_object().filter(|v| v.len() <= 2048).ok_or("OFX parameter semantics must be a bounded object")?;
    let params = params.as_array().filter(|v| v.len() <= 2048).ok_or("OFX remote parameters must be a bounded array")?;
    let ids: std::collections::BTreeSet<_> = params.iter().filter_map(|p| p["id"].as_str()).collect();
    for (id, value) in fields {
        if id.len() > 256 || !ids.contains(id.as_str()) {
            return Err("OFX cached parameter semantics do not match the registered parameter layout".into());
        }
        let value = value.as_object().filter(|v| v.len() <= 10).ok_or("OFX parameter semantics entry must be a bounded object")?;
        for (key, data) in value {
            let valid = match key.as_str() {
                "ofxName" | "type" | "doubleType" | "coordinates" => data.is_null() || data.as_str().is_some_and(|s| s.len() <= 1024),
                "default" | "min" | "max" | "displayMin" | "displayMax" => {
                    data.as_array().is_some_and(|v| v.len() <= 4 && v.iter().all(|n| n.is_null() || n.as_f64().is_some_and(f64::is_finite)))
                }
                "choices" => data.as_array().is_some_and(|v| v.len() <= 4096 && v.iter().all(|s| s.as_str().is_some_and(|s| s.len() <= 1024))),
                _ => false,
            };
            if !valid {
                return Err("OFX cached parameter semantics contain an unsupported field or value shape".into());
            }
        }
    }
    Ok(())
}

fn validated_gpu(metadata: &Value) -> Result<Value, String> {
    for name in ["openGL", "cuda", "openCL"] {
        let value = &metadata[name];
        if !value.is_null() && !value.is_boolean() && !value.as_str().is_some_and(|s| s.len() <= 64) {
            return Err("OFX cached GPU declarations have an invalid value shape".into());
        }
    }
    let depths = &metadata["openGLDepths"];
    if !depths.is_null() && !depths.as_array().is_some_and(|v| v.len() <= 16 && v.iter().all(|s| s.as_str().is_some_and(|s| s.len() <= 128))) {
        return Err("OFX cached OpenGL depths have an invalid value shape".into());
    }
    let host = metadata["hostGpu"].as_object().filter(|v| v.len() <= 8).ok_or("OFX cached GPU host information is unavailable")?;
    if !metadata["hostGpu"]["available"].is_boolean() {
        return Err("OFX cached GPU host availability must be a boolean".into());
    }
    for (key, data) in host {
        let valid = match key.as_str() {
            "available" | "disabled" => data.is_boolean(),
            "backend" | "vendor" | "renderer" | "version" | "reason" => data.as_str().is_some_and(|s| s.len() <= 4096),
            _ => false,
        };
        if !valid {
            return Err("OFX cached GPU host information has an unsupported field or value shape".into());
        }
    }
    Ok(json!({"openGL":metadata["openGL"],"cuda":metadata["cuda"],"openCL":metadata["openCL"],"openGLDepths":depths,"host":metadata["hostGpu"]}))
}
static ACTIVE_HELPERS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
struct HelperPermit;
impl HelperPermit {
    fn acquire() -> Result<Self, String> {
        use std::sync::atomic::Ordering;
        let mut count = ACTIVE_HELPERS.load(Ordering::Acquire);
        loop {
            if count >= 16 {
                return Err("OFX helper limit reached (16); reduce concurrent native renders".into());
            }
            match ACTIVE_HELPERS.compare_exchange(count, count + 1, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => return Ok(Self),
                Err(current) => count = current,
            }
        }
    }
}
impl Drop for HelperPermit {
    fn drop(&mut self) {
        ACTIVE_HELPERS.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Descriptor {
    experimental_ofx: bool,
    worker: PathBuf,
    plugin: PathBuf,
    state_dir: PathBuf,
    manifest: PluginManifest,
    remote: Value,
    #[serde(default, rename = "parameterSemantics")]
    _parameter_semantics: Value,
    #[serde(default, rename = "gpu")]
    gpu: Value,
}
struct RemoteEffect {
    descriptor: Descriptor,
    source: String,
}
struct Connection {
    _permit: HelperPermit,
    child: Child,
    input: ChildStdin,
    replies: mpsc::Receiver<Result<Value, String>>,
    frames: tempfile::NamedTempFile,
    effects: Vec<Value>,
    errors: Vec<String>,
}
impl Drop for Connection {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Connection {
    fn start(worker: &Path, plugin: &Path, state: &Path) -> Result<Self, String> {
        if !worker.is_absolute() || !worker.is_file() || !plugin.is_absolute() || !plugin.exists() || !state.is_absolute() {
            return Err("OFX helper/plugin/state paths must be existing absolute locations".into());
        }
        fs::create_dir_all(state).map_err(|e| e.to_string())?;
        let permit = HelperPermit::acquire()?;
        let frames = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
        let mut child = Command::new(worker)
            .arg("--effectcraft-helper")
            .env("EFFECTCRAFT_HELPER_STATE_DIR", state)
            .creation_flags(0x08000000)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Cannot start OFX helper: {e}"))?;
        let Some(input) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("OFX helper stdin unavailable".into());
        };
        let Some(output) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("OFX helper stdout unavailable".into());
        };
        let (tx, replies) = mpsc::sync_channel(4);
        if let Err(e) = std::thread::Builder::new().name("ofx-replies".into()).spawn(move || {
            let mut reader = BufReader::new(output);
            loop {
                let mut line = Vec::new();
                let result = (&mut reader).take(MAX_LINE + 1).read_until(b'\n', &mut line);
                match result {
                    Ok(0) => {
                        let _ = tx.send(Err("OFX helper exited or crashed".into()));
                        break;
                    }
                    Ok(_) if line.len() as u64 <= MAX_LINE => {
                        let line = String::from_utf8_lossy(&line);
                        if let Some(at) = line.find("@ec-helper ") {
                            let value = line
                                .get(at + 11..)
                                .ok_or("invalid helper marker")
                                .and_then(|s| serde_json::from_str(s).map_err(|_| "invalid OFX helper JSON"))
                                .map_err(str::to_string);
                            if tx.send(value).is_err() {
                                break;
                            }
                        }
                    }
                    _ => {
                        let _ = tx.send(Err("OFX helper output exceeds its line limit or could not be read".into()));
                        break;
                    }
                }
            }
        }) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e.to_string());
        }
        let mut connection = Self { _permit: permit, child, input, replies, frames, effects: Vec::new(), errors: Vec::new() };
        connection.send(json!({"op":"hello","version":1,"frames":connection.frames.path()}))?;
        let hello = connection.receive(Instant::now() + Duration::from_secs(20))?;
        if hello["op"] != "hello" || hello["version"] != 1 {
            return Err("Unsupported OFX helper protocol".into());
        }
        connection.send(json!({"op":"load","path":plugin}))?;
        let reply = connection.receive(Instant::now() + Duration::from_secs(120))?;
        if reply["op"] != "effects" {
            return Err(format!("Cannot load OFX bundle: {}", reply["message"]));
        }
        connection.effects = reply["effects"].as_array().ok_or("OFX helper omitted effect metadata")?.clone();
        connection.errors = reply["errors"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect();
        Ok(connection)
    }
    fn send(&mut self, value: Value) -> Result<(), String> {
        let mut bytes = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_LINE {
            return Err("OFX request exceeds 16 MiB".into());
        }
        bytes.push(b'\n');
        self.input.write_all(&bytes).and_then(|_| self.input.flush()).map_err(|e| e.to_string())
    }
    fn receive(&self, deadline: Instant) -> Result<Value, String> {
        self.replies.recv_timeout(deadline.saturating_duration_since(Instant::now())).map_err(|_| "OFX helper timed out or disconnected".to_string())?
    }
    fn append_image(&mut self, buf: &Buf) -> Result<Value, String> {
        let pixels = buf.img.width.checked_mul(buf.img.height).filter(|n| *n > 0 && *n <= MAX_PIXELS).ok_or("OFX temporal source exceeds image limits")?;
        if buf.img.width > 16384
            || buf.img.height > 16384
            || buf.img.data.len() != pixels as usize
            || !buf.scale.is_finite()
            || buf.scale <= 0.0
            || buf.offset.iter().any(|v| !v.is_finite() || v.abs() > 1.0e7)
        {
            return Err("Invalid OFX temporal source geometry".into());
        }
        let file = self.frames.as_file_mut();
        let offset = file.seek(SeekFrom::End(0)).map_err(|e| e.to_string())?;
        if offset.checked_add(u64::from(pixels) * 16).is_none_or(|end| end > MAX_FRAME_STORE_BYTES) {
            return Err("OFX temporal source storage exceeds 512 MiB per render".into());
        }
        let mut writer = std::io::BufWriter::new(file);
        for pixel in &buf.img.data {
            for value in pixel {
                if !value.is_finite() {
                    return Err("OFX temporal source has non-finite pixels".into());
                }
                writer.write_all(&value.to_ne_bytes()).map_err(|e| e.to_string())?;
            }
        }
        writer.flush().map_err(|e| e.to_string())?;
        Ok(json!({"offset":offset,"width":buf.img.width,"height":buf.img.height,"origin":buf.offset,"scale":buf.scale}))
    }
}
/// Discover a binary or bundle through the helper; does not execute native code in AfterCraft.
pub(crate) fn discover(worker: &Path, plugin: &Path, state: &Path) -> Result<(Vec<Value>, Vec<String>), String> {
    let connection = Connection::start(worker, plugin, state)?;
    Ok((connection.effects.clone(), connection.errors.clone()))
}
pub(crate) fn descriptor(remote: &Value, worker: &Path, plugin: &Path, state: &Path) -> Result<Value, String> {
    let raw = &remote["manifest"];
    let mut params = Vec::new();
    for p in raw["params"].as_array().ok_or("OFX manifest has no parameters")? {
        if let Ok(mut parsed) = serde_json::from_value::<PluginParam>(p.clone()) {
            if let PluginParamKind::Slider { decimals, .. } = &mut parsed.kind
                && p["integer"] == true
            {
                *decimals = 0;
            }
            params.push(parsed);
        }
    }
    let manifest = PluginManifest {
        api: 1,
        id: format!("ofx.{}", raw["id"].as_str().ok_or("missing OFX effect id")?),
        name: raw["name"].as_str().unwrap_or("Unnamed OFX effect").into(),
        category: raw["category"].as_str().filter(|s| !s.is_empty()).unwrap_or("OpenFX").into(),
        version: raw["version"].as_str().unwrap_or_default().into(),
        author: raw["author"].as_str().unwrap_or_default().into(),
        description:
            "Experimental OFX helper with CPU frames, temporal inputs and automatic Windows OpenGL for compatible plug-ins. Custom UI and native CUDA/Metal host APIs are unavailable."
                .into(),
        params,
    };
    manifest.validate()?;
    let mut profile = json!({"experimental_ofx":true,"worker":worker,"plugin":plugin,"state_dir":state,"manifest":manifest,"remote":remote});
    // Missing optional cache data does not prevent ordinary CPU registration.
    let _ = enrich_profile(&mut profile);
    Ok(profile)
}
pub(crate) fn load(bytes: &[u8], source: &str) -> Result<(&'static effectcraft_effects::EffectSpec, Value), String> {
    if bytes.len() > MAX_DESCRIPTOR_BYTES {
        return Err("OFX descriptor exceeds 8 MiB".into());
    }
    let mut profile: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    // Deferred registrations stay quiet at startup. The importer reports missing cache
    // data once per plugin/state folder with explicit rescan advice.
    let _ = enrich_profile(&mut profile);
    let mut descriptor: Descriptor = serde_json::from_value(profile).map_err(|e| e.to_string())?;
    descriptor.worker = crate::bundled_worker(descriptor.worker, "effectcraft-helper-ofx.exe");
    if !descriptor.manifest.name.ends_with(" [OFX]") {
        descriptor.manifest.name.push_str(" [OFX]");
    }
    if !descriptor.experimental_ofx {
        return Err("Native OFX profiles require explicit experimental_ofx opt-in".into());
    }
    descriptor.manifest.validate()?;
    if !descriptor.worker.is_absolute()
        || !descriptor.worker.is_file()
        || !descriptor.plugin.is_absolute()
        || !descriptor.plugin.exists()
        || !descriptor.state_dir.is_absolute()
    {
        return Err("Invalid OFX helper/plugin/state paths".into());
    }
    let gpu = descriptor.gpu.clone();
    let spec = register_plugin(Arc::new(RemoteEffect { descriptor, source: source.into() }))?;
    Ok((spec, gpu))
}
struct PoolEntry {
    worker: PathBuf,
    plugin: PathBuf,
    state: PathBuf,
    connection: Connection,
}
fn pool() -> &'static Mutex<Vec<PoolEntry>> {
    static POOL: OnceLock<Mutex<Vec<PoolEntry>>> = OnceLock::new();
    POOL.get_or_init(|| Mutex::new(Vec::new()))
}
impl RemoteEffect {
    fn values(&self, params: &PluginParams, frame: &PluginFrame) -> Result<(Vec<f64>, Vec<String>), String> {
        if params.flat.len() != self.descriptor.manifest.flat_len() {
            return Err("OFX parameter count mismatch".into());
        }
        let mut exposed = std::collections::HashMap::new();
        let mut at = 0usize;
        for p in &self.descriptor.manifest.params {
            let end = at.checked_add(p.kind.width()).ok_or("OFX parameter span overflow")?;
            exposed.insert(p.id.as_str(), params.flat.get(at..end).ok_or("missing OFX control values")?);
            at = end;
        }
        let mut flat = Vec::new();
        let mut texts = Vec::new();
        for p in self.descriptor.remote["manifest"]["params"].as_array().ok_or("OFX metadata has no parameters")? {
            let id = p["id"].as_str().ok_or("missing OFX parameter id")?;
            if let Some(values) = exposed.get(id) {
                flat.extend_from_slice(values);
                continue;
            }
            let value = &p["default"];
            match p["type"].as_str().ok_or("missing OFX parameter type")? {
                "text" | "hidden" => texts.push(value.as_str().unwrap_or_default().to_string()),
                "layer" => flat.push(-1.0),
                "checkbox" => flat.push(if value.as_bool().unwrap_or(false) { 1.0 } else { 0.0 }),
                "slider" | "angle" | "popup" => flat.push(value.as_f64().unwrap_or(0.0)),
                "point" | "point3" | "color" => {
                    let kind = p["type"].as_str().unwrap_or_default();
                    let count = if kind == "point" {
                        2
                    } else if kind == "point3" {
                        3
                    } else {
                        4
                    };
                    let values = value.as_array().filter(|a| a.len() == count).ok_or("invalid OFX vector default")?;
                    for (i, v) in values.iter().enumerate() {
                        let mut v = v.as_f64().ok_or("invalid OFX vector component")?;
                        if kind == "point" {
                            v *= if i == 0 { f64::from(frame.width) } else { f64::from(frame.height) };
                        } else if kind == "point3" && i < 2 {
                            v *= frame.scale;
                        }
                        flat.push(v);
                    }
                }
                _ => return Err("Unsupported OFX helper parameter layout".into()),
            }
        }
        if flat.iter().any(|v| !v.is_finite()) {
            return Err("Non-finite OFX parameter".into());
        }
        Ok((flat, texts))
    }
    fn process(&self, frame: &mut PluginFrame, params: &PluginParams, time: f64, ctx: Option<&EffectCtx>) -> Result<(), String> {
        let fps = ctx.map_or(25.0, |c| if c.env.frame_rate > 0.0 { c.env.frame_rate } else { 25.0 });
        let instance = ctx.map_or(0, |c| u64::from(c.seed));
        let layer_size = ctx.map_or([f64::from(frame.width) / frame.scale, f64::from(frame.height) / frame.scale], |c| c.layer_size);
        let count = frame.width.checked_mul(frame.height).filter(|n| *n > 0 && *n <= MAX_PIXELS).ok_or("OFX frame exceeds 16 million pixels")?;
        if frame.width > 16384
            || frame.height > 16384
            || frame.pixels.len() != count as usize
            || !frame.scale.is_finite()
            || frame.scale <= 0.0
            || !time.is_finite()
            || !fps.is_finite()
            || fps <= 0.0
            || frame.origin.iter().any(|v| !v.is_finite() || v.abs() > 1.0e7)
        {
            return Err("Invalid OFX render geometry or clock".into());
        }
        let (flat, texts) = self.values(params, frame)?;
        let mut entries = pool().lock().map_err(|_| "OFX helper pool lock unavailable")?;
        let d = &self.descriptor;
        let entry = if let Some(i) = entries.iter().position(|e| e.worker == d.worker && e.plugin == d.plugin && e.state == d.state_dir) {
            entries.remove(i)
        } else {
            if entries.len() >= MAX_IDLE_HELPERS {
                entries.remove(0);
            }
            PoolEntry {
                worker: d.worker.clone(),
                plugin: d.plugin.clone(),
                state: d.state_dir.clone(),
                connection: Connection::start(&d.worker, &d.plugin, &d.state_dir)?,
            }
        };
        // Temporal Source checkout may render earlier OFX effects recursively. Lease the
        // connection, releasing the pool lock before any renderer callbacks can reenter us.
        drop(entries);
        let mut entry = entry;
        let result = (|| -> Result<(), String> {
            let connection = &mut entry.connection;
            let remote_id = d.remote["manifest"]["id"].as_str().ok_or("missing remote effect id")?;
            let current = connection.effects.iter().find(|e| e["manifest"]["id"] == remote_id).ok_or("OFX effect is no longer available; rescan its bundle")?;
            if current["manifest"] != d.remote["manifest"] {
                return Err("OFX metadata changed; rescan and restart AfterCraft".into());
            }
            let deadline = Instant::now() + Duration::from_secs(120);
            if d.remote["padding"] == true {
                connection.send(json!({"op":"padding","effect":remote_id,"instance":instance,"time":time,"scale":frame.scale,"layer_size":layer_size,"flat":flat,"texts":texts}))?;
                let reply = connection.receive(deadline)?;
                if reply["op"] != "padding" || reply["pixels"].as_u64() != Some(0) {
                    return Err("This OFX effect requires expanded output bounds or its padding query failed".into());
                }
            }
            let mut bytes = Vec::with_capacity(count as usize * 16);
            for pixel in frame.pixels.iter() {
                for value in pixel {
                    if !value.is_finite() {
                        return Err("Non-finite OFX source pixel".into());
                    }
                    bytes.extend_from_slice(&value.to_ne_bytes());
                }
            }
            connection.frames.as_file_mut().set_len(bytes.len() as u64).map_err(|e| e.to_string())?;
            connection
                .frames
                .as_file_mut()
                .seek(SeekFrom::Start(0))
                .and_then(|_| connection.frames.as_file_mut().write_all(&bytes))
                .map_err(|e| e.to_string())?;
            let image = json!({"offset":0,"width":frame.width,"height":frame.height,"origin":frame.origin,"scale":frame.scale});
            // Keep Source separate from offset-zero output: a later callback must never
            // read pixels the running plug-in has already overwritten.
            let mut source_now = image.clone();
            source_now["offset"] = json!(bytes.len());
            connection.frames.as_file_mut().write_all(&bytes).map_err(|e| e.to_string())?;
            connection.send(json!({"op":"render","effect":remote_id,"instance":instance,"time":time,"frame":image,"layer_size":layer_size,"flat":flat,"texts":texts,"frame_rate":fps,"frame_range":null}))?;
            let mut unavailable = None;
            let mut done = false;
            let mut sampled: std::collections::HashMap<u64, Value> = std::collections::HashMap::new();
            for _ in 0..1024 {
                let reply = connection.receive(deadline)?;
                let requested = reply["time"].as_f64().filter(|t| t.is_finite() && t.abs() <= 1.0e7);
                let same_time = requested.is_some_and(|t| (t - time).abs() < 1e-9);
                match reply["op"].as_str().unwrap_or_default() {
                    "done" => {
                        done = true;
                        break;
                    }
                    "error" => return Err(format!("OFX plugin: {}", reply["message"])),
                    "input_at" if same_time => connection.send(json!({"op":"image","image":source_now}))?,
                    "params_at" if same_time => connection.send(json!({"op":"params","values":[flat,texts]}))?,
                    "input_at" => {
                        let t = requested.ok_or("OFX requested a non-finite or out-of-range source time")?;
                        let key = t.to_bits();
                        let response = if let Some(image) = sampled.get(&key) {
                            image.clone()
                        } else if let Some(buf) = ctx.and_then(|c| c.env.host?.self_at(t, c.env.effect_index)) {
                            if sampled.len() >= 128 {
                                return Err("OFX temporal checkout exceeds 128 source frames per render".into());
                            }
                            let image = connection.append_image(&buf)?;
                            sampled.insert(key, image.clone());
                            image
                        } else {
                            unavailable.get_or_insert_with(|| format!("OFX temporal source frame unavailable at {t} layer seconds"));
                            Value::Null
                        };
                        connection.send(json!({"op":"image","image":response}))?;
                    }
                    "layer_input" => {
                        unavailable.get_or_insert_with(|| "OFX additional layer inputs are not connected by this adapter".to_string());
                        connection.send(json!({"op":"image","image":null}))?;
                    }
                    "params_at" => {
                        let t = requested.ok_or("OFX requested a non-finite or out-of-range parameter time")?;
                        let values = if let Some((context, evaluated)) = ctx.and_then(|c| Some((c, c.env.host?.params_at(t)?))) {
                            let geometry = effectcraft_effects::Buf { scale: frame.scale, offset: frame.origin, ..Default::default() };
                            let sampled_ctx = EffectCtx {
                                params: &evaluated,
                                time: t,
                                layer_size: context.layer_size,
                                seed: context.seed,
                                adjustment: context.adjustment,
                                env: context.env,
                            };
                            let values = effectcraft_effects::plugin::flatten(params.manifest, &sampled_ctx, &geometry);
                            let values = PluginParams { manifest: params.manifest, flat: &values };
                            let (flat, texts) = self.values(&values, frame)?;
                            json!([flat, texts])
                        } else {
                            unavailable.get_or_insert_with(|| format!("OFX temporal parameter sample unavailable at {t} layer seconds"));
                            Value::Null
                        };
                        connection.send(json!({"op":"params","values":values}))?;
                    }
                    _ => return Err("Unexpected OFX helper response".into()),
                }
            }
            if !done {
                return Err("OFX exceeded 1024 callbacks in one render".into());
            }
            if let Some(reason) = unavailable {
                return Err(reason);
            }
            connection
                .frames
                .as_file_mut()
                .seek(SeekFrom::Start(0))
                .and_then(|_| connection.frames.as_file_mut().read_exact(&mut bytes))
                .map_err(|e| e.to_string())?;
            let mut rendered = Vec::with_capacity(count as usize);
            for pixel in bytes.as_chunks::<16>().0 {
                let mut rgba = [0.0; 4];
                for (out, channel) in rgba.iter_mut().zip(pixel.as_chunks::<4>().0) {
                    *out = f32::from_ne_bytes(*channel);
                    if !out.is_finite() {
                        return Err("OFX helper returned non-finite pixels".into());
                    }
                }
                rendered.push(rgba);
            }
            frame.pixels.copy_from_slice(&rendered);
            Ok(())
        })();
        if result.is_ok() {
            let mut pool = pool().lock().map_err(|_| "OFX helper pool lock unavailable")?;
            if pool.len() >= MAX_IDLE_HELPERS {
                pool.remove(0);
            }
            pool.push(entry);
        } // A bad/hung/crashed helper is dropped/killed; the next frame reconnects.
        result
    }
}
impl EffectPlugin for RemoteEffect {
    fn native_gpu_available(&self) -> bool {
        let gpu = &self.descriptor.gpu;
        let forced_cpu = std::env::var("AFTERCRAFT_OFX_GPU").is_ok_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "0" | "off" | "false" | "cpu"));
        !forced_cpu
            && gpu["host"]["available"] == true
            && gpu["host"]["disabled"] != true
            && (gpu["openGL"] == "true" || gpu["openGL"] == "needed" || gpu["openGL"] == true)
    }
    fn manifest(&self) -> &PluginManifest {
        &self.descriptor.manifest
    }
    fn source(&self) -> String {
        self.source.clone()
    }
    fn render(&self, frame: &mut PluginFrame, params: &PluginParams, time: f64) -> Result<(), String> {
        self.process(frame, params, time, None)
    }
    fn render_with_context(&self, frame: &mut PluginFrame, params: &PluginParams, ctx: &effectcraft_effects::EffectCtx) -> Result<(), String> {
        self.process(frame, params, ctx.time, Some(ctx))
    }
}

#[cfg(test)]
mod profile_metadata_tests {
    use super::*;

    fn fixture(id: &str) -> (tempfile::TempDir, Value, Value, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        let cache = state.join("ofx-descriptors");
        fs::create_dir_all(&cache).unwrap();
        let worker = dir.path().join("effectcraft-helper-ofx.exe");
        let plugin = dir.path().join("fixture.ofx");
        fs::write(&worker, b"original metadata-only fixture; never executed").unwrap();
        fs::write(&plugin, b"original plugin-path fixture; never executed").unwrap();
        let param = json!({"id":"gain","name":"Gain","type":"slider","default":1.0,"min":0,"max":2});
        let profile = json!({
            "experimental_ofx":true,"worker":worker,"plugin":plugin,"state_dir":state,
            "manifest":{"api":1,"id":format!("ofx.{id}"),"name":"Cached GPU","category":"Test","params":[param.clone()]},
            "remote":{"manifest":{"api":1,"id":id,"name":"Cached GPU","category":"Test","params":[param]},"padding":false}
        });
        let metadata = json!({
            "id":id,"ofxId":id,"openGL":"true","cuda":null,"openCL":null,"openGLDepths":["OfxBitDepthFloat"],
            "hostGpu":{"available":true,"backend":"opengl","renderer":"Synthetic metadata fixture","disabled":false},
            "parameterSemantics":{"gain":{"ofxName":"gain","type":"OfxParamTypeDouble","doubleType":"OfxParamDoubleTypePlain","coordinates":null,
                "default":[1.0],"min":[0.0],"max":[2.0],"displayMin":[0.0],"displayMax":[2.0],"choices":[]}}
        });
        let path = cache.join(format!("{id}.json"));
        fs::write(&path, serde_json::to_vec(&metadata).unwrap()).unwrap();
        (dir, profile, metadata, path)
    }

    #[test]
    fn legacy_profile_enrichment_is_read_only_and_preserves_existing_fields() {
        let (dir, mut profile, metadata, cache) = fixture("org.test.cached_units");
        let disk_profile = dir.path().join("old.ecofx.json");
        let before = serde_json::to_vec(&profile).unwrap();
        fs::write(&disk_profile, &before).unwrap();
        assert!(crate::enrich_native_profile(&mut profile).unwrap());
        assert_eq!(profile["parameterSemantics"], metadata["parameterSemantics"]);
        assert_eq!(profile["gpu"]["host"], metadata["hostGpu"]);
        assert_eq!(fs::read(&disk_profile).unwrap(), before, "enrichment must never rewrite a user registration");
        fs::write(cache, b"corrupt after first enrichment").unwrap();
        let enriched = profile.clone();
        assert!(!enrich_profile(&mut profile).unwrap(), "complete profiles need no cache access");
        assert_eq!(profile, enriched);
    }

    #[test]
    fn only_absent_or_null_fields_are_filled() {
        let (_dir, mut profile, metadata, _) = fixture("org.test.kept_gpu");
        let gpu = json!({"host":{"available":false},"manual":"preserve"});
        profile["gpu"] = gpu.clone();
        profile["parameterSemantics"] = Value::Null;
        assert!(enrich_profile(&mut profile).unwrap());
        assert_eq!(profile["gpu"], gpu);
        assert_eq!(profile["parameterSemantics"], metadata["parameterSemantics"]);
    }

    #[test]
    fn mismatched_or_malformed_cache_does_not_partially_change_a_profile() {
        let (_dir, profile, metadata, path) = fixture("org.test.wrong_cache");
        for (field, replacement) in [
            ("id", json!("org.test.other")),
            ("parameterSemantics", json!({"unknown":{"doubleType":"OfxParamDoubleTypePlain"}})),
            ("parameterSemantics", json!({"gain":{"doubleType":["wrong shape"]}})),
            ("parameterSemantics", json!({"gain":{"default":["not a number"]}})),
            ("hostGpu", json!({"available":"true"})),
            ("hostGpu", json!({"available":true,"privateUnexpectedField":"reject"})),
        ] {
            let mut record = metadata.clone();
            record[field] = replacement;
            fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
            let mut candidate = profile.clone();
            assert!(enrich_profile(&mut candidate).is_err(), "{field}");
            assert_eq!(candidate, profile, "failed enrichment must be atomic");
        }
    }

    #[test]
    fn missing_oversized_traversing_or_mismatched_profile_identities_are_rejected() {
        let (_dir, profile, _, path) = fixture("org.test.safe_path");
        fs::File::create(&path).unwrap().set_len(MAX_DESCRIPTOR_BYTES as u64 + 1).unwrap();
        assert!(enrich_profile(&mut profile.clone()).unwrap_err().contains("8 MiB"));
        fs::remove_file(&path).unwrap();
        assert!(enrich_profile(&mut profile.clone()).unwrap_err().contains("unavailable"));
        for id in ["../escape", "..", "org/test", "CON", "LPT1", "org:test"] {
            let mut candidate = profile.clone();
            candidate["remote"]["manifest"]["id"] = json!(id);
            candidate["manifest"]["id"] = json!(format!("ofx.{id}"));
            assert!(enrich_profile(&mut candidate).unwrap_err().contains("safe metadata file"));
        }
        let mut mismatch = profile.clone();
        mismatch["manifest"]["id"] = json!("ofx.org.test.different");
        assert!(enrich_profile(&mut mismatch).is_err());
        let mut traversal = profile;
        traversal["state_dir"] = json!(path.parent().unwrap().join("../outside"));
        assert!(enrich_profile(&mut traversal).unwrap_err().contains("parent traversal"));
    }

    #[test]
    fn loader_reports_enriched_gpu_capabilities_without_executing_a_helper() {
        let (_dir, profile, metadata, _) = fixture("org.test.cached_gpu_read_only");
        let result = crate::loader(&serde_json::to_vec(&profile).unwrap(), "legacy.ecofx.json").unwrap();
        assert_eq!(result["nativeGpuCapabilities"]["host"], metadata["hostGpu"]);
        assert_eq!(result["nativeGpuCapabilities"]["openGL"], "true");
        assert_eq!(result["id"], "ofx.org.test.cached_gpu_read_only");
        // The worker and bundle above contain plain text. Registration would fail if it
        // tried to execute them; actual hardware initialization belongs to rendering.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn api_two_defaults_keep_flat_and_text_parameter_positions() {
        let remote = json!({"manifest":{"api":2,"id":"org.test.ofx.layout","name":"Layout","category":"Test","params":[
            {"id":"gain","name":"Gain","type":"slider","default":2,"min":0,"max":4,"integer":true},
            {"id":"opaque","name":"Opaque","type":"hidden","default":"preserve me"},
            {"id":"mask","name":"Mask","type":"layer"},
            {"id":"position","name":"Position","type":"point3","default":[10,20,3]}
        ]},"padding":false});
        let value = descriptor(&remote, Path::new("C:/worker.exe"), Path::new("C:/bundle.ofx"), Path::new("C:/state")).unwrap();
        let effect = RemoteEffect { descriptor: serde_json::from_value(value).unwrap(), source: String::new() };
        assert_eq!(effect.manifest().params.len(), 1);
        let params = PluginParams { manifest: effect.manifest(), flat: &[0.5] };
        let mut pixels = [[0.0; 4]; 1];
        let frame = PluginFrame { width: 1, height: 1, scale: 0.5, pixels: &mut pixels, origin: [0.0; 2] };
        let (flat, text) = effect.values(&params, &frame).unwrap();
        assert_eq!(flat, vec![0.5, -1.0, 5.0, 10.0, 3.0]);
        assert_eq!(text, vec!["preserve me"]);
    }
    #[test]
    #[ignore = "requires EFFECTCRAFT_OFX_TEST_WORKER and EFFECTCRAFT_OFX_TEST_BUNDLE"]
    fn ofx_helper_renders_controls_preserves_limits_and_recovers_from_exit() {
        let worker = PathBuf::from(std::env::var("EFFECTCRAFT_OFX_TEST_WORKER").unwrap());
        let bundle = PathBuf::from(std::env::var("EFFECTCRAFT_OFX_TEST_BUNDLE").unwrap());
        let state = tempfile::tempdir().unwrap();
        let (effects, errors) = discover(&worker, &bundle, state.path()).unwrap();
        assert!(errors.is_empty(), "{errors:?}");
        let make = |suffix: &str| {
            let remote = effects.iter().find(|e| e["manifest"]["id"].as_str().is_some_and(|s| s.ends_with(suffix))).unwrap();
            let profile = descriptor(remote, &worker, &bundle, state.path()).unwrap();
            load(&serde_json::to_vec(&profile).unwrap(), "test.ecofx.json").unwrap();
            effectcraft_effects::plugin::plugin(profile["manifest"]["id"].as_str().unwrap()).unwrap()
        };
        let invert = make(".invert");
        let mut large = descriptor(
            effects.iter().find(|e| e["manifest"]["id"].as_str().is_some_and(|id| id.ends_with(".invert"))).unwrap(),
            &worker,
            &bundle,
            state.path(),
        )
        .unwrap();
        large["manifest"]["id"] = json!("ofx.org.test.large_metadata");
        large["remote"]["notes"] = json!("x".repeat(1024 * 1024));
        let bytes = serde_json::to_vec(&large).unwrap();
        assert!(bytes.len() > 1024 * 1024);
        let loaded = load(&bytes, "large.ecofx.json").unwrap();
        assert!(loaded.0.name.ends_with(" [OFX]"));
        assert!(load(&vec![0; MAX_DESCRIPTOR_BYTES + 1], "oversized.ecofx.json").err().unwrap().contains("8 MiB"));
        let source = [[0.2, 0.3, 0.4, 1.0], [2.0, 0.1, 0.2, 1.0]];
        let run = |effect: &Arc<dyn EffectPlugin>, values: &[f64]| {
            let mut pixels = source;
            let params = PluginParams { manifest: effect.manifest(), flat: values };
            let result = effect.render(&mut PluginFrame { width: 2, height: 1, pixels: &mut pixels, scale: 0.5, origin: [0.0; 2] }, &params, 0.0);
            (result, pixels)
        };
        let (result, pixels) = run(&invert, &[1.0]);
        result.unwrap();
        for (actual, expected) in pixels.into_iter().flatten().zip([0.8, 0.7, 0.6, 1.0, -1.0, 0.9, 0.8, 1.0]) {
            assert!((actual - expected).abs() < 0.0001);
        }
        let (result, pixels) = run(&invert, &[0.0]);
        result.unwrap();
        assert_eq!(pixels, source);
        let echo = make(".echo");
        let (result, pixels) = run(&echo, &[]);
        assert!(result.unwrap_err().contains("temporal"));
        assert_eq!(pixels, source);
        struct TimeHost;
        impl effectcraft_effects::EffectHost for TimeHost {
            fn layer(&self, _: u64, _: bool) -> Option<effectcraft_effects::LayerPixels> {
                None
            }
            fn audio(&self, _: u64, _: f64, _: usize, _: u32) -> Option<Vec<f32>> {
                None
            }
            fn self_at(&self, time: f64, _: usize) -> Option<Buf> {
                let mut buf = Buf { scale: 0.5, offset: [2.0, 4.0], ..Default::default() };
                buf.img.width = 2;
                buf.img.height = 1;
                buf.img.data = vec![[time as f32 * 0.2, time as f32 * 0.3, time as f32 * 0.4, 1.0]; 2];
                Some(buf)
            }
        }
        let evaluated = effectcraft_effects::Params::default();
        let ctx = EffectCtx {
            params: &evaluated,
            time: 1.0,
            layer_size: [4.0, 2.0],
            seed: 0,
            adjustment: false,
            env: effectcraft_effects::EffectEnv { host: Some(&TimeHost), frame_rate: 10.0, ..Default::default() },
        };
        let mut pixels = [[0.2, 0.3, 0.4, 1.0]; 2];
        echo.render_with_context(
            &mut PluginFrame { width: 2, height: 1, pixels: &mut pixels, scale: 0.5, origin: [2.0, 4.0] },
            &PluginParams { manifest: echo.manifest(), flat: &[] },
            &ctx,
        )
        .unwrap();
        for pixel in pixels {
            for (actual, expected) in pixel.into_iter().zip([0.19, 0.285, 0.38, 1.0]) {
                assert!((actual - expected).abs() < 1e-5, "padded temporal frame: {pixel:?}");
            }
        }
        let border = make(".border");
        let (result, pixels) = run(&border, &[3.0]);
        assert!(result.unwrap_err().contains("expanded"));
        assert_eq!(pixels, source);
        run(&invert, &[0.0]).0.unwrap();
        {
            let mut pool = pool().lock().unwrap();
            let c = &mut pool.last_mut().unwrap().connection;
            c.child.kill().unwrap();
            c.child.wait().unwrap();
        }
        let (result, pixels) = run(&invert, &[1.0]);
        assert!(result.is_err());
        assert_eq!(pixels, source);
        run(&invert, &[1.0]).0.unwrap();
        pool().lock().unwrap().clear();
    }
}
