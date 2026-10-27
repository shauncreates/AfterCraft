//! No native plugin code ever executes in the supervisor process.
mod aegp;
mod aegp_context;
mod ansi;
mod application;
mod conformance;
mod diagnostics;
mod frame;
mod memory;
mod native;
mod param_info;
mod pipl;
mod pixels;
mod process_job;
mod sequence;
mod smart;
mod timing;
mod utility;
mod worlds;

use serde_json::{Value, json};
use std::io::{Read, Write};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Duration, Instant};

const MARKER: &str = "EC_AEX_EVENT ";
const MAX_LOG: usize = 1024 * 1024;
static EXIT_CODE: AtomicI32 = AtomicI32::new(0);
fn print_report(report: &Value) -> Result<(), String> {
    println!("{}", serde_json::to_string_pretty(report).map_err(|e| e.to_string())?);
    if report["status"] != "probe_complete" && report["status"] != "frame_written" {
        EXIT_CODE.store(2, Ordering::Relaxed);
    }
    Ok(())
}

pub fn event(value: Value) {
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{MARKER}{value}");
    let _ = stdout.flush();
}

fn capture(mut pipe: impl Read) -> (String, bool) {
    let mut log = Vec::new();
    let mut buffer = [0u8; 8192];
    let mut truncated = false;
    loop {
        match pipe.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let keep = n.min(MAX_LOG.saturating_sub(log.len()));
                log.extend_from_slice(&buffer[..keep]);
                truncated |= keep < n;
            }
        }
    }
    (String::from_utf8_lossy(&log).into_owned(), truncated)
}

fn supervise(args: &[String], timeout: Duration) -> Result<Value, String> {
    let job = process_job::WorkerJob::new()?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut child = Command::new(exe)
        .args(args)
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot start plugin worker: {e}"))?;
    if let Err(error) = job.attach(&child) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    let out = child.stdout.take().ok_or("worker stdout unavailable")?;
    let err = child.stderr.take().ok_or("worker stderr unavailable")?;
    let out_reader = std::thread::spawn(move || capture(out));
    let err_reader = std::thread::spawn(move || capture(err));
    let start = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => break status,
            None if start.elapsed() >= timeout => {
                timed_out = true;
                job.terminate()?;
                break child.wait().map_err(|e| e.to_string())?;
            }
            _ => std::thread::sleep(Duration::from_millis(20)),
        }
    };
    drop(job); // Kill descendants before joining pipe readers, even if they inherited stdout.
    let (stdout, out_cut) = out_reader.join().map_err(|_| "stdout reader failed")?;
    let (stderr, err_cut) = err_reader.join().map_err(|_| "stderr reader failed")?;
    let events: Vec<Value> = stdout.lines().filter_map(|s| s.strip_prefix(MARKER)).filter_map(|s| serde_json::from_str(s).ok()).collect();
    let complete = events.iter().any(|e| e["stage"] == "complete");
    let frame_written = events.iter().any(|e| e["stage"] == "frame_written");
    let diagnostics = diagnostics::summarize(&events);
    let worker_stdout = stdout.lines().filter(|line| !line.starts_with(MARKER)).collect::<Vec<_>>().join("\n");
    Ok(json!({
        "protocol": 1, "frameWritten":frame_written,"outputVerified":false,"effectcraftIntegrated":false,
        "status": if timed_out { "timeout" } else if !status.success() { "worker_failed" } else if frame_written {"frame_written"} else if complete { "probe_complete" } else { "incomplete" },
        "exitCode": status.code(), "elapsedMs": start.elapsed().as_millis(),
        "events": events, "diagnostics": diagnostics, "workerStdout": worker_stdout,
        "stderr": stderr, "logsTruncated": out_cut || err_cut,
    }))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("help");
    match command {
        "inspect" | "probe" => {
            let path = args.get(1).ok_or("supply an absolute .aex path")?;
            if !Path::new(path).is_absolute() {
                return Err("plugin path must be absolute".into());
            }
            probe_options(args.get(2..).unwrap_or_default())?;
            let mut worker_args = args.clone();
            if let Some(first) = worker_args.first_mut() {
                *first = format!("worker-{command}");
            }
            let report = supervise(&worker_args, Duration::from_secs(20))?;
            print_report(&report)?;
        }
        "worker-inspect" | "worker-probe" => {
            let path = args.get(1).ok_or("missing plugin path")?;
            let (dirs, match_name, api) = probe_options(args.get(2..).unwrap_or_default())?;
            native::worker(Path::new(path), command == "worker-probe", &dirs, None, match_name.as_deref(), api)?;
        }
        "render" => {
            if args.len() != 2 {
                return Err("render requires one absolute job.json path".into());
            }
            let path = args.get(1).ok_or("missing job path")?;
            if !Path::new(path).is_absolute() {
                return Err("job path must be absolute".into());
            }
            frame::Job::read(Path::new(path))?;
            let report = supervise(&["worker-render".into(), path.clone()], Duration::from_secs(60))?;
            print_report(&report)?;
        }
        "worker-render" => {
            let job = frame::Job::read(Path::new(args.get(1).ok_or("missing job path")?))?;
            native::worker(&job.plugin, true, &job.dll_dirs, Some(&job), job.match_name.as_deref(), job.host_api)?;
        }
        "self-test" => {
            let crash = supervise(&["worker-test-exit".into()], Duration::from_secs(2))?;
            let hang = supervise(&["worker-test-hang".into()], Duration::from_millis(150))?;
            let orphan = supervise(&["worker-test-orphan".into()], Duration::from_secs(2))?;
            if crash["status"] != "worker_failed" || crash["exitCode"] != 73 || hang["status"] != "timeout" || orphan["exitCode"] != 0 {
                return Err(format!("isolation tests failed: {crash}, {hang}"));
            }
            println!("{}", json!({"workerExitContained": true, "timeoutContained": true,"descendantPipeContained":true}));
        }
        "worker-test-exit" => std::process::exit(73),
        "worker-test-hang" => std::thread::sleep(Duration::from_secs(60)),
        "worker-test-orphan" => {
            let child = Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
                .arg("worker-test-hang")
                .creation_flags(0x08000000)
                .spawn()
                .map_err(|e| e.to_string())?;
            drop(child);
        }
        "roundtrip-test" => {
            if args.len() != 3 {
                return Err("roundtrip-test requires <absolute-fixture.dll> <new-absolute-directory>".into());
            }
            let report = conformance::run(Path::new(args.get(1).ok_or("missing fixture")?), Path::new(args.get(2).ok_or("missing directory")?))?;
            println!("{}", serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?);
        }
        _ => println!(
            "effectcraft-aex-probe inspect <absolute.aex> [--dll-dir <folder>]\neffectcraft-aex-probe probe <absolute.aex> [--dll-dir <folder>]\neffectcraft-aex-probe render <absolute-job.json>\neffectcraft-aex-probe self-test\neffectcraft-aex-probe roundtrip-test <fixture.dll> <new-directory>\nExperimental Windows x64 native host. Render output needs visual verification.\ninspect reads PiPL metadata; probe/render execute trusted plugin code in a child process.\nEffectCraft integration and commercial plugin compatibility are not established."
        ),
    }
    Ok(())
}
type ProbeOptions = (Vec<std::path::PathBuf>, Option<String>, frame::HostApi);
fn probe_options(args: &[String]) -> Result<ProbeOptions, String> {
    if args.len() > 36 || !args.len().is_multiple_of(2) {
        return Err("expected --dll-dir, --match-name or --host-api option/value pairs".into());
    }
    let mut dirs = Vec::new();
    let mut name = None;
    let mut api = frame::HostApi::ThirdParty;
    for pair in args.as_chunks::<2>().0 {
        let value = pair.get(1).ok_or("missing option value")?;
        match pair.first().map(String::as_str) {
            Some("--dll-dir") => {
                let path = Path::new(value);
                if !path.is_absolute() || !path.is_dir() || dirs.len() >= 16 {
                    return Err("at most 16 existing absolute DLL directories are allowed".into());
                }
                dirs.push(path.canonicalize().map_err(|e| e.to_string())?);
            }
            Some("--match-name") if !value.is_empty() && value.len() <= 512 && name.is_none() => name = Some(value.clone()),
            Some("--host-api") => {
                api = match value.as_str() {
                    "after_effects" => frame::HostApi::AfterEffects,
                    "third_party" => frame::HostApi::ThirdParty,
                    _ => return Err("unknown host API".into()),
                }
            }
            _ => return Err("unknown or duplicate probe option".into()),
        }
    }
    Ok((dirs, name, api))
}

fn main() {
    if let Err(error) = run() {
        event(json!({"stage": "error", "message": error}));
        std::process::exit(1);
    }
    let code = EXIT_CODE.load(Ordering::Relaxed);
    if code != 0 {
        std::process::exit(code);
    }
}
