//! Bounded external AE parser and a separate, non-rendering scripting session. Native
//! plug-ins are registered from their descriptors; importing never renders their code.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use effectcraft_engine::{ScriptRequest, effects};
use effectcraft_project::{ItemKind, ParamUi, Project};
use serde_json::{Value, json};

const MAX_SOURCE: u64 = 64 * 1024 * 1024;
const MAX_SCRIPT: u64 = 32 * 1024 * 1024;
const MAX_REPORT: u64 = 64 * 1024 * 1024;
const MAX_LOG: u64 = 2 * 1024 * 1024;
const CONVERTER: &str = "aep2effectcraft.py";

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err(format!("{} exceeds the {} MiB import limit", path.display(), limit / 1024 / 1024));
    }
    Ok(bytes)
}

fn find_script() -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os("AFTERCRAFT_AEP_SCRIPT") {
        candidates.push(PathBuf::from(path));
    }
    if let Some(folder) = std::env::current_exe().ok().as_deref().and_then(Path::parent) {
        candidates.push(folder.join("aep-import").join(CONVERTER));
    }
    candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/aep-import").join(CONVERTER));
    candidates
        .into_iter()
        .find(|p| p.is_file())
        .ok_or_else(|| "The AE import converter is not installed beside AfterCraft (aep-import/aep2effectcraft.py)".into())
}

struct BoundedWriter<W> {
    inner: W,
    left: u64,
}
impl<W: Write> Write for BoundedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() as u64 > self.left {
            return Err(std::io::Error::other("AE import catalog exceeds 64 MiB"));
        }
        let written = self.inner.write(bytes)?;
        self.left -= written as u64;
        Ok(written)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn param_type(ui: &ParamUi) -> &'static str {
    match ui {
        ParamUi::Checkbox => "checkbox",
        ParamUi::Popup { .. } => "popup",
        ParamUi::Point => "point",
        ParamUi::Point3 => "point3",
        ParamUi::Color => "color",
        ParamUi::Angle => "angle",
        ParamUi::Layer => "layer",
        ParamUi::Text | ParamUi::File { .. } => "text",
        ParamUi::Hidden => "hidden",
        ParamUi::Path => "path",
        ParamUi::Gradient => "gradient",
        _ => "slider",
    }
}

fn catalog(path: &Path, options: &Value) -> Result<Vec<String>, String> {
    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut out = BoundedWriter { inner: std::io::BufWriter::new(file), left: MAX_REPORT };
    out.write_all(b"{\"version\":1,\"preferredEdition\":").map_err(|e| e.to_string())?;
    serde_json::to_writer(&mut out, &options["edition"]).map_err(|e| e.to_string())?;
    out.write_all(b",\"effects\":[").map_err(|e| e.to_string())?;
    for (index, effect) in effects::all().iter().enumerate() {
        if index > 0 {
            out.write_all(b",").map_err(|e| e.to_string())?;
        }
        let params: Vec<Value> = effect
            .params
            .iter()
            .map(|p| {
                let mut value = json!({"id":p.id,"name":p.name,"default":p.default.to_json(),"type":param_type(&p.ui),"ui":p.ui});
                if let ParamUi::Popup { options } = &p.ui {
                    value["options"] = json!(options);
                }
                value
            })
            .collect();
        serde_json::to_writer(&mut out, &json!({"id":effect.id,"name":effect.name,"category":effect.category,"params":params})).map_err(|e| e.to_string())?;
    }
    out.write_all(b"],\"nativeProfiles\":[").map_err(|e| e.to_string())?;
    let mut warnings = Vec::new();
    let mut profiles = Vec::new();
    if let Some(folder) = crate::config_dir().map(|d| d.join("Plug-ins")).filter(|p| p.is_dir()) {
        for entry in fs::read_dir(folder).map_err(|e| e.to_string())?.take(4096) {
            let path = entry.map_err(|e| e.to_string())?.path();
            let name = path.to_string_lossy().to_ascii_lowercase();
            if path.is_file() && (name.ends_with(".ecaex.json") || name.ends_with(".ecofx.json")) {
                profiles.push(path);
            }
        }
    }
    profiles.sort();
    let mut written = 0usize;
    for path in profiles {
        let profile = read_limited(&path, 8 * 1024 * 1024).and_then(|b| serde_json::from_slice::<Value>(&b).map_err(|e| e.to_string()));
        match profile {
            Ok(value) => {
                if written > 0 {
                    out.write_all(b",").map_err(|e| e.to_string())?;
                }
                serde_json::to_writer(&mut out, &value).map_err(|e| e.to_string())?;
                written += 1;
            }
            Err(e) => warnings.push(format!("Cannot include native descriptor {}: {e}", path.display())),
        }
    }
    out.write_all(b"]}").and_then(|_| out.flush()).map_err(|e| e.to_string())?;
    Ok(warnings)
}

fn convert(source: &Path, original: &str, directory: &Path, options: &Value) -> Result<(String, Value), String> {
    let script = find_script()?;
    let output = directory.join("project.jsx");
    let report = directory.join("report.json");
    let catalog_file = directory.join("catalog.json");
    let warnings = catalog(&catalog_file, options)?;
    let stdout_path = directory.join("converter.stdout");
    let stderr_path = directory.join("converter.stderr");
    let executable_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
    let bundled_python = executable_dir.as_ref().map(|p| p.join("runtime/python/python.exe"));
    let python = std::env::var_os("AFTERCRAFT_AEP_PYTHON")
        .or_else(|| bundled_python.filter(|p| p.is_file()).map(PathBuf::into_os_string))
        .unwrap_or_else(|| if cfg!(windows) { "python.exe".into() } else { "python3".into() });
    let mut command = Command::new(python);
    command
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .arg(&script)
        .arg(source)
        .arg("-o")
        .arg(&output)
        .arg("--catalog")
        .arg(catalog_file)
        .arg("--report")
        .arg(&report)
        .arg("--source-project")
        .arg(original)
        .current_dir(directory)
        .stdin(Stdio::null())
        .stdout(File::create(&stdout_path).map_err(|e| e.to_string())?)
        .stderr(File::create(&stderr_path).map_err(|e| e.to_string())?);
    let site = std::env::var_os("AFTERCRAFT_AEP_SITE_PACKAGES")
        .or_else(|| executable_dir.as_ref().map(|p| p.join("runtime/python/Lib/site-packages")).filter(|p| p.is_dir()).map(PathBuf::into_os_string));
    if let Some(site) = site {
        command.arg("--site-packages").arg(site);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = ChildGuard(command.spawn().map_err(|e| {
        format!("Cannot start Python for AE import: {e}. Repair the AfterCraft installation or configure AFTERCRAFT_AEP_PYTHON with py-aep installed.")
    })?);
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.0.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if start.elapsed() > Duration::from_secs(120) {
            return Err("AE project conversion exceeded 120 seconds; the original and open project are unchanged".into());
        }
        for path in [&stdout_path, &stderr_path] {
            if fs::metadata(path).map_err(|e| e.to_string())?.len() > MAX_LOG {
                return Err("AE project converter exceeded its diagnostic output limit".into());
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    for path in [&stdout_path, &stderr_path] {
        if fs::metadata(path).map_err(|e| e.to_string())?.len() > MAX_LOG {
            return Err("AE project converter exceeded its diagnostic output limit".into());
        }
    }
    if !status.success() {
        let stderr = read_limited(&stderr_path, MAX_LOG)?;
        let reason = String::from_utf8_lossy(&stderr);
        return Err(format!("AE conversion failed ({status}): {}", reason.chars().take(4000).collect::<String>()));
    }
    let code = String::from_utf8(read_limited(&output, MAX_SCRIPT)?).map_err(|e| e.to_string())?;
    let mut report: Value = serde_json::from_slice(&read_limited(&report, MAX_REPORT)?).map_err(|e| format!("Invalid import report: {e}"))?;
    if !report.is_object() {
        return Err("AE converter report must be an object".into());
    }
    report["catalogWarnings"] = json!(warnings);
    Ok((code, report))
}

pub fn import(path: &str, options: &Value) -> Result<(Project, Value), String> {
    if !options["edition"].is_null() && !matches!(options["edition"].as_str(), Some("adobe" | "ofx")) {
        return Err("AE import edition must be adobe or ofx".into());
    }
    let original = Path::new(path).canonicalize().map_err(|e| format!("Cannot open AE project: {e}"))?;
    let source = read_limited(&original, MAX_SOURCE)?;
    let root = crate::config_dir().ok_or("Cannot locate the import report folder")?.join("AepImports");
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let job = tempfile::Builder::new().prefix("import-").tempdir_in(root).map_err(|e| e.to_string())?;
    let input = job.path().join(original.file_name().ok_or("AE project has no filename")?);
    fs::write(&input, source).map_err(|e| e.to_string())?;
    let original = original.to_string_lossy().to_string();
    let mut staged = crate::staging_session();
    // Descriptors register deferred rendering adapters; no commercial plug-in is
    // rendered or probed here. This also makes command-line imports see installed ids.
    let registrations = if let Some(folder) = crate::config_dir().map(|d| d.join("Plug-ins")).filter(|p| p.is_dir()) {
        Some(staged.execute("effect.plugins.load", json!({"folder":folder})).map_err(|e| e.to_string())?)
    } else {
        None
    };
    let (code, mut report) = convert(&input, &original, job.path(), options)?;
    let runner = staged.script.ok_or("AE import requires the scripting engine")?;
    let runtime = runner(&mut staged, &ScriptRequest { code: &code, name: &original, console: false });
    if runtime["ok"] != true || runtime.get("error").is_some_and(|e| !e.is_null()) {
        return Err(format!("Cannot rebuild the AE project: {}", runtime["error"]));
    }
    let mut comps = 0usize;
    let mut layers = 0usize;
    let mut three_d = 0usize;
    let mut effect_count = 0usize;
    let mut keyframes = 0usize;
    let mut property_count = 0usize;
    for (_, comp) in staged.project.comps() {
        comps += 1;
        layers += comp.layers.len();
        for layer in &comp.layers {
            three_d += usize::from(layer.is_3d());
            effect_count += layer.props.sub("effects").map_or(0, |group| group.groups().count());
            let mut stack: Vec<_> = layer.props.children.iter().map(|node| (node, 0usize)).collect();
            while let Some((node, depth)) = stack.pop() {
                property_count = property_count.saturating_add(1);
                if depth > 64 || property_count > 1_000_000 {
                    return Err("Staged AE property tree exceeds import limits".into());
                }
                match node {
                    effectcraft_project::Node::Prop(property) => {
                        keyframes = keyframes.saturating_add(property.keys.len());
                        if keyframes > 1_000_000 {
                            return Err("Staged AE keyframes exceed import limits".into());
                        }
                    }
                    effectcraft_project::Node::Group(group) => stack.extend(group.children.iter().map(|node| (node, depth + 1))),
                }
            }
        }
    }
    let footage = staged.project.items.values().filter(|i| matches!(i.kind, ItemKind::Footage(_))).count();
    report["actualImported"] = json!({"items":staged.project.items.len(),"comps":comps,"layers":layers,"footage":footage,
        "effects":effect_count,"keyframes":keyframes,"properties":property_count,"threeDLayers":three_d});
    report["runtime"] = runtime;
    report["pluginRegistration"] = json!(registrations);
    report["sourceProject"] = json!(original);
    report["fidelityVerified"] = json!(false);
    report["artifactDirectory"] = json!(job.path());
    report["preservedSourcePath"] = json!(input);
    let bytes = serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_REPORT {
        return Err("Combined AE import report exceeds 64 MiB".into());
    }
    fs::write(job.path().join("report.json"), bytes).map_err(|e| e.to_string())?;
    let project = (*staged.project).clone();
    let _directory = job.keep();
    Ok((project, report))
}
