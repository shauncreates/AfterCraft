//! Vendor-neutral native discovery. Only the external supervisor executes native DLLs.
use crate::PluginManifest;
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    fs,
    io::{Read, Write},
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

type Progress<'a> = &'a dyn Fn(u64, u64, &str) -> bool;

fn path(value: &Value, key: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(value.get(key).and_then(Value::as_str).ok_or_else(|| format!("missing {key}"))?);
    if !p.is_absolute() {
        return Err(format!("{key} must be an absolute path"));
    }
    Ok(p)
}
fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("missing destination directory")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| format!("cannot save {}: {e}", path.display()))?;
    Ok(())
}
fn read_json(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let mut bytes = Vec::new();
    fs::File::open(path).map_err(|e| e.to_string())?.take(1024 * 1024 + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() > 1024 * 1024 {
        return Err("plugin settings exceed 1 MiB".into());
    }
    let bytes = bytes.strip_prefix(&[239, 187, 191]).unwrap_or(&bytes);
    serde_json::from_slice(bytes).map_err(|e| e.to_string())
}
fn libraries(settings: &Path) -> Result<Vec<PathBuf>, String> {
    let value = read_json(settings)?;
    let dirs: Vec<PathBuf> = value["dll_dirs"].as_array().map(|a| a.iter().filter_map(Value::as_str).map(PathBuf::from).collect()).unwrap_or_default();
    if dirs.len() > 16 || dirs.iter().any(|p| !p.is_absolute() || !p.is_dir()) {
        return Err("plugin library settings contain missing folders or exceed 16 folders".into());
    }
    Ok(dirs)
}
/// Stable identifier from the SDK match name; moving a plugin does not change its effect id.
fn identifier(name: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for b in name.bytes() {
        hash = (hash ^ u64::from(b)).wrapping_mul(0x100000001b3);
    }
    format!("org.effectcraft.aex.e{hash:016x}")
}
fn scan(roots: Vec<PathBuf>, progress: Progress<'_>) -> Result<Vec<PathBuf>, String> {
    let mut pending: Vec<_> = roots.into_iter().map(|p| (p, 0)).collect();
    let mut seen = HashSet::new();
    let mut files = Vec::new();
    let mut visited = 0u64;
    while let Some((p, depth)) = pending.pop() {
        if !progress(0, 0, &format!("Discovering plugins: {}", p.display())) {
            return Err("cancelled".into());
        }
        if depth > 16 {
            return Err("plugin folder nesting exceeds 16 levels".into());
        }
        let p = p.canonicalize().map_err(|e| format!("{}: {e}", p.display()))?;
        if !seen.insert(p.clone()) {
            continue;
        }
        visited += 1;
        if visited > 100000 {
            return Err("plugin scan exceeds 100,000 entries; select a narrower folder".into());
        }
        if p.is_dir() {
            for entry in fs::read_dir(&p).map_err(|e| format!("{}: {e}", p.display()))? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.file_type().map_err(|e| e.to_string())?.is_symlink() {
                    continue;
                }
                pending.push((entry.path(), depth + 1));
                if pending.len() > 100000 {
                    return Err("plugin scan contains too many entries".into());
                }
            }
        } else if p.extension().is_some_and(|e| ["aex", "wasm", "wat", "ofx"].iter().any(|x| e.eq_ignore_ascii_case(x))) {
            files.push(p);
            if files.len() > 4096 {
                return Err("plugin scan exceeds 4096 binaries".into());
            }
        }
    }
    files.sort();
    Ok(files)
}
fn run(
    worker: &Path,
    command: &str,
    plugin: &Path,
    name: Option<&str>,
    dirs: &[PathBuf],
    timeout: Duration,
    cancel: &dyn Fn() -> bool,
) -> Result<Value, String> {
    let stdout = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
    let stderr = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
    let mut cmd = Command::new(worker);
    cmd.arg(command).arg(plugin).arg("--host-api").arg("after_effects");
    if let Some(name) = name {
        cmd.arg("--match-name").arg(name);
    }
    for dir in dirs {
        cmd.arg("--dll-dir").arg(dir);
    }
    let mut child = cmd
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(stdout.reopen().map_err(|e| e.to_string())?)
        .stderr(stderr.reopen().map_err(|e| e.to_string())?)
        .spawn()
        .map_err(|e| format!("cannot start native worker: {e}"))?;
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() < timeout && cancel() => std::thread::sleep(Duration::from_millis(50)),
            other => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(match other {
                    Err(e) => e.to_string(),
                    _ if start.elapsed() >= timeout => "native probe timed out".into(),
                    _ => "cancelled".into(),
                });
            }
        }
    };
    let mut bytes = Vec::new();
    stdout.reopen().map_err(|e| e.to_string())?.take(2 * 1024 * 1024 + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("native probe report exceeds 2 MiB".into());
    }
    let report: Value = serde_json::from_slice(&bytes).map_err(|e| format!("invalid worker report: {e}"))?;
    if !status.success() || report["protocol"] != 1 || report["status"] != "probe_complete" {
        let detail = report["events"]
            .as_array()
            .and_then(|a| a.iter().rev().find(|e| e["stage"] == "error"))
            .and_then(|v| v["message"].as_str())
            .unwrap_or("initialization failed; dependencies or host services may be missing");
        return Err(format!("{}: {detail}", report["status"]));
    }
    Ok(report)
}
fn profile(report: &Value, effect: &Value, worker: &Path, plugin: &Path, dirs: &[PathBuf]) -> Result<(Value, usize), String> {
    let events = report["events"].as_array().ok_or("missing probe events")?;
    if !events.iter().any(|e| e["stage"] == "params_setup_result" && e["code"] == 0) {
        return Err("plugin parameter registration did not complete".into());
    }
    let mut params = Vec::new();
    let mut indices = Vec::new();
    let mut index = 0usize;
    let mut unsupported = 0usize;
    for entry in events.iter().filter(|e| e["stage"] == "parameter") {
        index += 1;
        if index > 1024 || entry["index"] != -1 {
            return Err("unsupported native parameter count/insertion order".into());
        }
        if entry["hidden"] == true {
            continue;
        }
        let Some(control) = entry["numeric"].as_object() else {
            unsupported += 1;
            continue;
        };
        let mut control = control.clone();
        control.insert("id".into(), json!(format!("p{index}")));
        control.insert(
            "name".into(),
            json!(entry["name"].as_str().filter(|s| !s.trim().is_empty()).map(str::to_string).unwrap_or_else(|| format!("Parameter {index}"))),
        );
        let Ok(param) = serde_json::from_value::<effectcraft_effects::plugin::PluginParam>(Value::Object(control)) else {
            unsupported += 1;
            continue;
        };
        let check = PluginManifest {
            api: 1,
            id: "org.effectcraft.parameter_check".into(),
            name: "Parameter".into(),
            category: "Native".into(),
            version: String::new(),
            author: String::new(),
            description: String::new(),
            params: vec![param.clone()],
        };
        if check.validate().is_err() {
            unsupported += 1;
            continue;
        }
        params.push(param);
        indices.push(index);
    }
    let name = effect["name"].as_str().filter(|s| !s.is_empty()).ok_or("effect has no display name")?;
    let match_name = effect["match_name"].as_str().unwrap_or_default();
    let key = if match_name.is_empty() { format!("{}:{}", plugin.display(), effect["entry_point"]) } else { match_name.into() };
    let manifest = PluginManifest {
        api: 1,
        id: identifier(&key),
        name: name.into(),
        category: effect["category"].as_str().filter(|s| !s.is_empty()).unwrap_or("Native Adobe Plug-ins").into(),
        version: String::new(),
        author: String::new(),
        description: format!(
            "Experimental native Adobe API host. Initialization passed; rendering compatibility is unverified. {unsupported} unsupported/custom controls use native defaults."
        ),
        params,
    };
    manifest.validate()?;
    Ok((
        json!({"experimental_aex":true,"worker":worker,"plugin":plugin,"match_name":if match_name.is_empty(){None}else{Some(match_name)},
        "dll_dirs":dirs,"host_api":"after_effects","parameter_indices":indices,"manifest":manifest}),
        unsupported,
    ))
}
fn adobe_roots(programs: &Path) -> Result<(bool, Vec<PathBuf>), String> {
    let adobe = programs.join("Adobe");
    let mut roots = Vec::new();
    let mut detected = false;
    if adobe.is_dir() {
        for entry in fs::read_dir(&adobe).map_err(|e| e.to_string())?.take(128) {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_name().to_string_lossy().starts_with("Adobe After Effects") && entry.path().join("Support Files/AfterFX.exe").is_file() {
                detected = true;
                let plugins = entry.path().join("Support Files/Plug-ins");
                if plugins.is_dir() {
                    roots.push(plugins);
                }
            }
        }
    }
    if detected {
        for common in [adobe.join("Common/Plug-ins/7.0/MediaCore"), programs.join("Common Files/Adobe/Plug-Ins/7.0/MediaCore")] {
            if common.is_dir() {
                roots.push(common);
            }
        }
    }
    Ok((detected, roots))
}
pub fn install(request: &Value, progress: Progress<'_>) -> Result<Value, String> {
    let settings = path(request, "settings_folder")?;
    fs::create_dir_all(&settings).map_err(|e| e.to_string())?;
    let preferences = settings.join("native-plugin-settings.json");
    let private_plugins = settings.join("NativePlugins");
    if request["operation"] == "edition_preference" {
        let preference_file = settings.join("native-plugin-preference.json");
        if let Some(edition) = request.get("edition") {
            if !matches!(edition.as_str(), Some("adobe" | "ofx")) {
                return Err("edition must be adobe or ofx".into());
            }
            write(&preference_file, &serde_json::to_vec_pretty(&json!({"version":1,"edition":edition})).map_err(|e| e.to_string())?)?;
        }
        let edition = if preference_file.is_file() {
            let stored = read_json(&preference_file)?;
            let edition = stored["edition"].as_str().filter(|v| matches!(*v, "adobe" | "ofx")).ok_or("invalid saved plug-in edition preference")?;
            Some(edition.to_string())
        } else {
            None
        };
        let (adobe_host, roots) = match std::env::var_os("ProgramFiles") {
            Some(programs) => adobe_roots(&PathBuf::from(programs))?,
            None => (false, vec![]),
        };
        let adobe_plugins = edition.is_none() && scan(roots, progress)?.iter().any(|path| path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("aex")));
        let effective = if adobe_host { edition.as_deref().unwrap_or("ofx") } else { "ofx" };
        return Ok(
            json!({"profiles":[],"edition":edition,"effectiveEdition":effective,"adobeHostDetected":adobe_host,"needsChoice":edition.is_none() && adobe_plugins,
            "preferenceFile":preference_file}),
        );
    }
    if request["operation"] == "install_folder" || request["operation"] == "vendor_installer" {
        fs::create_dir_all(&private_plugins).map_err(|e| e.to_string())?;
        if request["operation"] == "vendor_installer" {
            let installer = path(request, "path")?;
            if !installer.is_file() {
                return Err("select a downloaded vendor installer".into());
            }
            let system = std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:/Windows"));
            let mut command = if installer.extension().is_some_and(|e| e.eq_ignore_ascii_case("msi")) {
                let mut command = Command::new(system.join("System32/msiexec.exe"));
                command.arg("/i").arg(&installer);
                command
            } else if installer.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe")) {
                Command::new(&installer)
            } else {
                return Err("vendor installers must be .exe or .msi files".into());
            };
            if let Some(parent) = installer.parent() {
                command.current_dir(parent);
            }
            command.creation_flags(0x08000000).spawn().map_err(|e| format!("cannot open vendor installer: {e}"))?;
            return Ok(json!({"profiles":[],"installerLaunched":true,"installationComplete":false,"customDestination":private_plugins,
                "toast":"Vendor installer opened. Choose the AfterCraft install folder if a custom destination is offered; after finishing, scan installed plug-ins."}));
        }
        if request["open"] != false {
            let system = std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:/Windows"));
            Command::new(system.join("explorer.exe")).arg(&private_plugins).creation_flags(0x08000000).spawn().map_err(|e| e.to_string())?;
        }
        return Ok(json!({"profiles":[],"folder":private_plugins,"toast":format!("Plug-in install destination: {}",private_plugins.display())}));
    }
    let mut dirs = libraries(&preferences)?;
    if request["operation"] == "add_library" {
        let folder = path(request, "folder")?.canonicalize().map_err(|e| e.to_string())?;
        if !folder.is_dir() {
            return Err("select a library folder".into());
        }
        let mut additions = vec![folder.clone()];
        // Shared-library packages commonly put DLLs one directory below their library root.
        for child in fs::read_dir(&folder).map_err(|e| e.to_string())? {
            let child = child.map_err(|e| e.to_string())?;
            if child.file_type().map_err(|e| e.to_string())?.is_dir()
                && !child.file_type().map_err(|e| e.to_string())?.is_symlink()
                && fs::read_dir(child.path())
                    .map_err(|e| e.to_string())?
                    .take(4096)
                    .filter_map(Result::ok)
                    .any(|f| f.path().extension().is_some_and(|e| e.eq_ignore_ascii_case("dll")))
            {
                additions.push(child.path().canonicalize().map_err(|e| e.to_string())?);
            }
        }
        for p in additions {
            if !dirs.contains(&p) {
                dirs.push(p);
            }
        }
        if dirs.len() > 16 {
            return Err("at most 16 shared-library directories are supported; select a narrower folder".into());
        }
        let result =
            json!({"dll_dirs":dirs,"profiles":[],"toast":"Added plug-in library folder. Scan the plug-in folder again to retry missing dependencies."});
        write(&preferences, &serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?)?;
        return Ok(result);
    }
    let worker = path(request, "worker")?;
    if !worker.is_file() {
        return Err("native worker is missing beside AfterCraft; rebuild/reinstall the experimental package".into());
    }
    if let Some(extra) = request["dll_dirs"].as_array() {
        for p in extra {
            let p = PathBuf::from(p.as_str().ok_or("invalid DLL directory")?);
            if !p.is_absolute() || !p.is_dir() {
                return Err("DLL folders must be existing absolute paths".into());
            }
            if !dirs.contains(&p) {
                dirs.push(p);
            }
        }
    }
    if dirs.len() > 16 {
        return Err("at most 16 DLL folders are supported".into());
    }
    let mut package_path = None;
    let mut adobe_host_detected = None;
    let roots = if request["installed"] == true {
        let mut roots = Vec::new();
        if let Some(programs) = std::env::var_os("ProgramFiles") {
            let (detected, adobe) = adobe_roots(&PathBuf::from(programs))?;
            adobe_host_detected = Some(detected);
            roots.extend(adobe);
        }
        let common_ofx =
            std::env::var_os("CommonProgramFiles").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:/Program Files/Common Files")).join("OFX/Plugins");
        if common_ofx.is_dir() {
            roots.push(common_ofx);
        }
        for p in [private_plugins, settings.join("NativePackages")] {
            if p.is_dir() {
                roots.push(p);
            }
        }
        for p in [Some(settings.join("OFX")), worker.parent().map(|p| p.join("OFX"))].into_iter().flatten() {
            if p.is_dir() {
                roots.push(p);
            }
        }
        if let Some(paths) = std::env::var_os("OFX_PLUGIN_PATH") {
            for p in std::env::split_paths(&paths) {
                if p.is_dir() {
                    roots.push(p);
                }
            }
        }
        roots
    } else {
        let selected = path(request, if request["path"].is_string() { "path" } else { "folder" })?;
        if selected.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")) {
            let unpacked = crate::package::unpack(&selected, &settings.join("NativePackages"), progress)?;
            package_path = Some(unpacked.clone());
            vec![unpacked]
        } else {
            vec![selected]
        }
    };
    let mut files = scan(roots, progress)?;
    if request["installed"] == true && adobe_host_detected != Some(true) {
        files.retain(|path| !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("aex")));
    }
    let destination = settings.join("Plug-ins");
    fs::create_dir_all(&destination).map_err(|e| e.to_string())?;
    let mut profiles = Vec::new();
    let mut installed = Vec::new();
    let mut errors = Vec::new();
    let mut skipped = 0usize;
    let mut updated = 0usize;
    let timeout_seconds = request["probe_timeout_seconds"].as_u64().unwrap_or(10);
    if !(1..=20).contains(&timeout_seconds) {
        return Err("probe_timeout_seconds must be in 1..20".into());
    }
    let mut timeouts = std::collections::HashMap::<PathBuf, usize>::new();
    for (i, plugin) in files.iter().enumerate() {
        let message = format!("Installing {}", plugin.display());
        if !progress(i as u64, files.len() as u64, &message) {
            return Err("cancelled".into());
        }
        let parent = plugin.parent().unwrap_or(plugin).to_path_buf();
        if request["path"].is_null() && timeouts.get(&parent).copied().unwrap_or(0) >= 3 {
            errors.push(json!({"path":plugin,"error":"Initialization skipped after three consecutive timeouts in this folder. Select this file with Install Effect Plug-in to retry."}));
            continue;
        }
        let mut timed_out = false;
        let result = (|| -> Result<(), String> {
            let extension = plugin.extension().and_then(|s| s.to_str()).unwrap_or_default().to_ascii_lowercase();
            if extension == "ofx" {
                let helper = path(request, "ofx_worker")?;
                let state = settings.join("ofx-helper-state");
                let (effects, issues) = crate::ofx::discover(&helper, plugin, &state)?;
                for error in issues {
                    errors.push(json!({"path":plugin,"error":error}));
                }
                for effect in effects {
                    let descriptor = crate::ofx::descriptor(&effect, &helper, plugin, &state)?;
                    let id = descriptor["manifest"]["id"].as_str().ok_or("missing OFX profile id")?;
                    let target = destination.join(format!("{id}.ecofx.json"));
                    if target.exists() {
                        let old = read_json(&target)?;
                        if old["plugin"] != descriptor["plugin"] {
                            errors.push(json!({"path":plugin,"error":"OFX id already belongs to a different binary"}));
                            continue;
                        }
                        skipped += 1;
                        if old != descriptor {
                            write(&target, &serde_json::to_vec_pretty(&descriptor).map_err(|e| e.to_string())?)?;
                            updated += 1;
                        }
                    } else {
                        write(&target, &serde_json::to_vec_pretty(&descriptor).map_err(|e| e.to_string())?)?;
                    }
                    profiles.push(target.clone());
                    installed.push(json!({"path":plugin,"name":descriptor["manifest"]["name"],"category":descriptor["manifest"]["category"],"id":id,"profile":target,"format":"ofx","renderingVerified":false}));
                }
                return Ok(());
            }
            if extension == "wasm" || extension == "wat" {
                if fs::metadata(plugin).map_err(|e| e.to_string())?.len() > 64 * 1024 * 1024 {
                    return Err("WASM file exceeds 64 MiB".into());
                }
                let target = destination.join(format!("{}.{}", identifier(&plugin.to_string_lossy()), extension));
                if !target.exists() {
                    write(&target, &fs::read(plugin).map_err(|e| e.to_string())?)?;
                } else {
                    skipped += 1;
                }
                profiles.push(target.clone());
                installed.push(json!({"path":plugin,"profile":target,"format":"wasm"}));
                return Ok(());
            }
            let inspected = run(&worker, "inspect", plugin, None, &dirs, Duration::from_secs(10), &|| progress(i as u64, files.len() as u64, &message))?;
            let effects = inspected["events"]
                .as_array()
                .and_then(|e| e.iter().find(|e| e["stage"] == "descriptors"))
                .and_then(|e| e["effects"].as_array())
                .ok_or("no effect metadata")?;
            for effect in effects.iter().filter(|e| e["kind"] == "eFKT") {
                let name = effect["match_name"].as_str().filter(|s| !s.is_empty());
                let one = (|| -> Result<(), String> {
                    let report =
                        run(&worker, "probe", plugin, name, &dirs, Duration::from_secs(timeout_seconds), &|| progress(i as u64, files.len() as u64, &message))?;
                    let (descriptor, unsupported) = profile(&report, effect, &worker, plugin, &dirs)?;
                    let id = descriptor["manifest"]["id"].as_str().ok_or("missing effect id")?;
                    let target = destination.join(format!("{id}.ecaex.json"));
                    if target.exists() {
                        let old = read_json(&target)?;
                        if old["plugin"] != descriptor["plugin"] || old["match_name"] != descriptor["match_name"] {
                            return Err("duplicate SDK match name at a different plugin path; existing registration retained".into());
                        }
                        skipped += 1;
                        if old != descriptor {
                            write(&target, &serde_json::to_vec_pretty(&descriptor).map_err(|e| e.to_string())?)?;
                            updated += 1;
                        }
                    } else {
                        write(&target, &serde_json::to_vec_pretty(&descriptor).map_err(|e| e.to_string())?)?;
                    }
                    profiles.push(target.clone());
                    installed.push(json!({"path":plugin,"name":effect["name"],"category":effect["category"],"id":id,"profile":target,"unsupportedControls":unsupported,"metadataWarnings":effect["warnings"],"renderingVerified":false}));
                    Ok(())
                })();
                if let Err(error) = one {
                    timed_out |= error.starts_with("native probe timed out") || error.starts_with("\"timeout\"");
                    errors.push(json!({"path":plugin,"name":effect["name"],"error":error}));
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            timed_out |= error.starts_with("native probe timed out") || error.starts_with("\"timeout\"");
            errors.push(json!({"path":plugin,"error":error}));
        }
        let previous = timeouts.get(&parent).copied().unwrap_or(0);
        timeouts.insert(parent, if timed_out { previous.saturating_add(1) } else { 0 });
    }
    progress(files.len() as u64, files.len() as u64, "Plugin discovery complete");
    if files.is_empty() {
        errors.push(json!({"error":"No native effect or WASM binaries were found. Vendor setup programs must be run first; scripts use File > Scripts, and CEP/UXP extensions require a separate runtime."}));
    }
    let result = json!({"profiles":profiles,"installed":installed,"alreadyInstalled":skipped,"updatedProfiles":updated,"errors":errors,"discoveredBinaries":files.len(),"packageFolder":package_path,"adobeHostDetected":adobe_host_detected,
        "report":settings.join("native-plugin-install-report.json"),
        "toast":format!("Plug-in scan: {} effects available, {skipped} already installed, {} errors. See Progress for details.",installed.len(),errors.len())});
    write(&settings.join("native-plugin-install-report.json"), &serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adobe_migration_requires_an_installed_after_effects_host() {
        let dir = tempfile::tempdir().unwrap();
        let common = dir.path().join("Adobe/Common/Plug-ins/7.0/MediaCore");
        fs::create_dir_all(&common).unwrap();
        assert_eq!(adobe_roots(dir.path()).unwrap(), (false, vec![]));
        let support = dir.path().join("Adobe/Adobe After Effects Test/Support Files");
        fs::create_dir_all(support.join("Plug-ins")).unwrap();
        assert_eq!(adobe_roots(dir.path()).unwrap(), (false, vec![]));
        fs::write(support.join("AfterFX.exe"), b"original detection fixture, never executed").unwrap();
        let (detected, roots) = adobe_roots(dir.path()).unwrap();
        assert!(detected);
        assert!(roots.contains(&common));
        assert!(roots.contains(&support.join("Plug-ins")));
    }
    #[test]
    fn edition_choice_is_saved_once_and_invalid_choices_leave_it_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        for edition in ["adobe", "ofx"] {
            let saved = install(&json!({"settings_folder":dir.path(),"operation":"edition_preference","edition":edition}), &|_, _, _| true).unwrap();
            assert_eq!(saved["edition"], edition);
            assert_eq!(saved["needsChoice"], false);
            let reopened = install(&json!({"settings_folder":dir.path(),"operation":"edition_preference"}), &|_, _, _| true).unwrap();
            assert_eq!(reopened["edition"], edition);
            assert_eq!(reopened["needsChoice"], false);
        }
        assert!(install(&json!({"settings_folder":dir.path(),"operation":"edition_preference","edition":"other"}), &|_, _, _| true).is_err());
        assert_eq!(read_json(&dir.path().join("native-plugin-preference.json")).unwrap()["edition"], "ofx");
    }
    #[test]
    fn discovery_is_recursive_deduplicated_and_cancellable() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("nested")).unwrap();
        fs::write(dir.path().join("nested/test.aex"), b"original invalid fixture").unwrap();
        fs::write(dir.path().join("ignore.exe"), b"not a plugin").unwrap();
        assert_eq!(scan(vec![dir.path().into(), dir.path().join("nested")], &|_, _, _| true).unwrap().len(), 1);
        assert_eq!(scan(vec![dir.path().into()], &|_, _, _| false).unwrap_err(), "cancelled");
        assert_eq!(identifier("Vendor Effect"), identifier("Vendor Effect"));
        assert_ne!(identifier("Vendor Effect"), identifier("Other Effect"));
    }
    #[test]
    fn install_folder_does_not_require_a_worker_or_valid_library_configuration() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("native-plugin-settings.json"), "invalid preferences").unwrap();
        let result = install(&json!({"settings_folder":dir.path(),"operation":"install_folder","open":false}), &|_, _, _| true).unwrap();
        assert!(Path::new(result["folder"].as_str().unwrap()).is_dir());
        assert_eq!(result["profiles"], json!([]));
    }
    #[test]
    #[ignore = "requires the original native fixture and worker"]
    fn installs_a_downloaded_zip_with_native_dependencies_preserved() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("download.zip");
        let mut archive = zip::ZipWriter::new(fs::File::create(&source).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        archive.start_file("Vendor/gain.aex", options).unwrap();
        archive.write_all(&fs::read(std::env::var("EFFECTCRAFT_AEX_TEST_PLUGIN").unwrap()).unwrap()).unwrap();
        archive.start_file("Vendor/resources/data.txt", options).unwrap();
        archive.write_all(b"original synthetic resource").unwrap();
        archive.finish().unwrap();
        let result =
            install(&json!({"settings_folder":dir.path(),"worker":std::env::var("EFFECTCRAFT_AEX_TEST_WORKER").unwrap(),"path":source}), &|_, _, _| true)
                .unwrap();
        assert_eq!(result["errors"], json!([]));
        assert_eq!(result["installed"].as_array().unwrap().len(), 1);
        let package = Path::new(result["packageFolder"].as_str().unwrap());
        assert_eq!(fs::read(package.join("Vendor/resources/data.txt")).unwrap(), b"original synthetic resource");
        assert!(Path::new(result["installed"][0]["path"].as_str().unwrap()).starts_with(package));
    }
    #[test]
    #[ignore = "requires the original native fixture and worker"]
    fn installs_a_non_vendor_specific_fixture_and_reloads_after_restart() {
        let dir = tempfile::tempdir().unwrap();
        let request = json!({"settings_folder":dir.path(),"worker":std::env::var("EFFECTCRAFT_AEX_TEST_WORKER").unwrap(),"path":std::env::var("EFFECTCRAFT_AEX_TEST_PLUGIN").unwrap()});
        let result = install(&request, &|_, _, _| true).unwrap();
        assert_eq!(result["errors"], json!([]));
        assert_eq!(result["installed"].as_array().unwrap().len(), 1);
        let saved = PathBuf::from(result["profiles"][0].as_str().unwrap());
        let loaded = crate::loader(&fs::read(&saved).unwrap(), saved.to_str().unwrap()).unwrap();
        assert_eq!(loaded["name"], "Test Gain [Adobe]");
        let again = install(&request, &|_, _, _| true).unwrap();
        assert_eq!(again["alreadyInstalled"], 1);
    }
}
