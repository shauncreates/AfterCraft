//! End-to-end oracle: an original native gain effect must produce exact known pixels.
use serde_json::json;
use std::{path::Path, time::Duration};
pub fn run(fixture: &Path, directory: &Path) -> Result<serde_json::Value, String> {
    if !fixture.is_absolute() || !directory.is_absolute() {
        return Err("test paths must be absolute".into());
    }
    std::fs::create_dir(directory).map_err(|e| e.to_string())?;
    let plugin = directory.join("gain.aex");
    std::fs::copy(fixture, &plugin).map_err(|e| e.to_string())?;
    let probe = crate::supervise(&["worker-probe".into(), plugin.to_string_lossy().into_owned()], Duration::from_secs(5))?;
    std::fs::write(directory.join("probe-report.json"), serde_json::to_vec_pretty(&probe).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let events = probe["events"].as_array().ok_or("probe events missing")?;
    if probe["status"] != "probe_complete"
        || !events.iter().any(|e| e["stage"] == "sequence_probe_complete" && e["renderAttempted"] == false)
        || events.iter().any(|e| e["stage"] == "render" || e["stage"] == "smart_render" || e["stage"] == "frame_written")
    {
        return Err(format!("discovery sequence lifecycle failed: {probe}"));
    }
    let input = directory.join("input.png");
    let mut image = image::RgbaImage::new(8, 8);
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        *pixel = image::Rgba([20 + x as u8 * 16, 40 + y as u8 * 16, 80, 255]);
    }
    image.save(&input).map_err(|e| e.to_string())?;
    for (name, gain, overrides, timing) in [
        ("default", 2.0, json!({}), json!({})),
        ("override", 0.5, json!({"1":0.5}), json!({})),
        ("ntsc", 2.0, json!({}), json!({"frame_rate":{"num":30000,"den":1001},"frame_index":10000,"render_mode":"legacy"})),
    ] {
        let output = directory.join(format!("{name}.png"));
        let job_path = directory.join(format!("{name}.json"));
        let mut job = json!({"plugin":plugin,"input":input,"output":output,"parameters":overrides});
        if let (Some(job), Some(extra)) = (job.as_object_mut(), timing.as_object()) {
            job.extend(extra.clone());
        }
        std::fs::write(&job_path, serde_json::to_vec_pretty(&job).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let report = crate::supervise(&["worker-render".into(), job_path.to_string_lossy().into_owned()], Duration::from_secs(5))?;
        std::fs::write(directory.join(format!("{name}-report.json")), serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if report["exitCode"] != 0 || !output.is_file() {
            return Err(format!("fixture rendering failed: {report}"));
        }
        if name == "ntsc"
            && !report["events"].as_array().is_some_and(|events| {
                events.iter().any(|e| e["stage"] == "render_clock" && e["scale"] == 30000 && e["step"] == 1001 && e["current"] == 10010000)
            })
        {
            return Err("NTSC clock changed across native IPC".into());
        }
        let preserved_message = report["diagnostics"]["pluginMessages"].as_array().is_some_and(|messages| {
            messages
                .iter()
                .any(|m| (m["stage"] == "render_result" || m["stage"] == "smart_render_result") && m["message"] == "Synthetic fixture render diagnostic")
        });
        if !preserved_message || !report["workerStdout"].as_str().is_some_and(|s| s.contains("Synthetic fixture stdout diagnostic")) {
            return Err("native fixture diagnostics were lost during worker capture".into());
        }
        if name != "ntsc"
            && (!report["events"].as_array().is_some_and(|events| events.iter().any(|e| e["stage"] == "smart_render_result" && e["code"] == 0))
                || !report["workerStdout"].as_str().is_some_and(|s| s.contains("Synthetic SmartFX pre-render data disposed")))
        {
            return Err("SmartFX lifecycle or opaque-data cleanup failed".into());
        }
        let rendered = image::open(&output).map_err(|e| e.to_string())?.to_rgba8();
        if rendered.dimensions() != image.dimensions() {
            return Err("fixture output dimensions changed".into());
        }
        for (expected, actual) in image.pixels().zip(rendered.pixels()) {
            let gain_pixel = |c: u8| (f64::from(c) * gain).round().clamp(0.0, 255.0) as u8;
            let expected = image::Rgba([gain_pixel(expected.0[0]), gain_pixel(expected.0[1]), gain_pixel(expected.0[2]), 255]);
            if actual != &expected {
                return Err(format!("{name}: expected {expected:?}, got {actual:?}"));
            }
        }
    }
    Ok(
        json!({"nativeDefaultValues":true,"nativeParameterOverride":true,"nativeSmartFx":true,"nativeLegacyRender":true,"smartPreDataDisposed":true,"pngFrameExchange":true,"nativeDiagnosticsPreserved":true,"exactNtscClock":true,"exactPixelsVerified":192,"artifacts":directory}),
    )
}
