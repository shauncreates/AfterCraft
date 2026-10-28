//! `effectcraft-helper-ofx`: AfterCraft's OpenFX effect helper.
//!
//! Started by AfterCraft with `--effectcraft-helper`, it loads the OpenFX plug-ins in the
//! standard OFX folders (and `OFX_PLUGIN_PATH`) with `effectcraft-ofx`, then serves them over
//! the external effect helper protocol (`docs/external-helpers.md`). The plug-ins run in this
//! process: if one crashes, AfterCraft reports it and starts the helper again. Plug-ins that
//! crash the helper while loading are remembered (in `EFFECTCRAFT_HELPER_STATE_DIR`) and skipped
//! on the next start.
//!
//! Run by hand: `effectcraft-helper-ofx --list [path]` prints what it would offer.

#[cfg(windows)]
mod process_job;

use std::path::PathBuf;
use std::sync::Arc;

use effectcraft_effects::plugin::{EffectPlugin, plugin};
use effectcraft_helper_host::protocol::HELPER_ARG;
use effectcraft_helper_host::serve::{Provider, serve};

struct Ofx {
    /// Folders loaded at start-up (the standard OFX folders and `OFX_PLUGIN_PATH`).
    paths: Vec<PathBuf>,
}

/// Load `path` and return the effects it registered (or had registered before).
fn load(path: &std::path::Path) -> (Vec<Arc<dyn EffectPlugin>>, Vec<String>) {
    let report = effectcraft_ofx::load_path(path);
    let effects = report.loaded.iter().filter_map(|e| plugin(&e.id)).collect();
    (effects, report.errors)
}

impl Provider for Ofx {
    fn name(&self) -> String {
        "OpenFX".into()
    }
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }
    fn loads(&self) -> Vec<String> {
        vec!["ofx".into(), "bundle".into()]
    }
    fn effects(&mut self) -> (Vec<Arc<dyn EffectPlugin>>, Vec<String>) {
        let (mut all, mut errors) = (vec![], vec![]);
        for p in &self.paths {
            if !p.exists() {
                continue;
            }
            let (e, errs) = load(p);
            all.extend(e);
            errors.extend(errs);
        }
        (all, errors)
    }
    fn load(&mut self, path: &str) -> Result<(Vec<Arc<dyn EffectPlugin>>, Vec<String>), String> {
        let (effects, errors) = load(std::path::Path::new(path));
        if effects.is_empty() && !errors.is_empty() {
            return Err(errors.join("; "));
        }
        Ok((effects, errors))
    }
}

fn main() {
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::System::Diagnostics::Debug::SetErrorMode(3);
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(windows)]
    if std::env::var_os("EFFECTCRAFT_OFX_CONTAINED").is_none() && !args.is_empty() {
        use std::os::windows::process::CommandExt;
        let result = (|| -> Result<i32, String> {
            let job = process_job::WorkerJob::new()?;
            let mut child = std::process::Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
                .args(&args)
                .env("EFFECTCRAFT_OFX_CONTAINED", "1")
                .creation_flags(0x08000000)
                .spawn()
                .map_err(|e| e.to_string())?;
            if let Err(error) = job.attach(&child) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
            let status = child.wait().map_err(|e| e.to_string())?;
            Ok(status.code().unwrap_or(1))
        })();
        std::process::exit(match result {
            Ok(code) => code,
            Err(error) => {
                eprintln!("OFX process containment: {error}");
                1
            }
        });
    }

    if let Some(dir) = std::env::var_os("EFFECTCRAFT_HELPER_STATE_DIR") {
        effectcraft_ofx::set_state_dir(PathBuf::from(dir));
    }
    let mut ofx = Ofx { paths: effectcraft_ofx::default_search_paths() };
    let code = if args.first().is_some_and(|a| a == "--gpu-info") {
        println!("{}", effectcraft_ofx::opengl::info());
        0
    } else if args.first().is_some_and(|a| a == "--describe") {
        if let Some(path) = args.get(1) {
            let report = effectcraft_ofx::load_path(std::path::Path::new(path));
            let records: Vec<_> = report
                .loaded
                .iter()
                .filter(|e| args.get(2).is_none_or(|id| e.id == *id || e.ofx_id == *id))
                .filter_map(|e| effectcraft_ofx::loader::metadata(&e.id))
                .collect();
            println!("{}", serde_json::json!({"effects":records,"errors":report.errors,"gpu":effectcraft_ofx::opengl::info()}));
            if records.is_empty() { 1 } else { 0 }
        } else {
            eprintln!("--describe requires an OFX bundle or binary path");
            2
        }
    } else if args.iter().any(|a| a == HELPER_ARG) {
        match serve(&mut ofx) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("effectcraft-helper-ofx: {e}");
                1
            }
        }
    } else if args.first().is_some_and(|a| a == "--list") {
        if let Some(p) = args.get(1) {
            ofx.paths = vec![PathBuf::from(p)];
        }
        let (effects, errors) = ofx.effects();
        for e in &effects {
            let m = e.manifest();
            println!("{}\t{}\t{}\t{} params", m.id, m.name, m.category, m.params.len());
        }
        for e in &errors {
            eprintln!("error: {e}");
        }
        println!("{} effect(s), {} error(s)", effects.len(), errors.len());
        0
    } else {
        eprintln!("effectcraft-helper-ofx {}: AfterCraft's OpenFX effect helper.", env!("CARGO_PKG_VERSION"));
        eprintln!("AfterCraft starts it with {HELPER_ARG}; put it in helpers/ next to AfterCraft.");
        eprintln!("`effectcraft-helper-ofx --list [path]` lists the plug-ins it finds.");
        2
    };
    // Stop the programs some plug-ins leave running (Sapphire's preset browser).
    effectcraft_ofx::shutdown();
    std::process::exit(code);
}
