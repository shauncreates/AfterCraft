//! Native OpenFX GPU rendering, implemented from the public ofxGPURender.h contract.
//!
//! Windows uses a private, invisible WGL window and a hardware OpenGL context. Contexts and
//! textures never enter the main application. Every render pairs context attach/detach,
//! transfers float/short/byte clip images, and reads the output texture back. Other platforms
//! continue using the CPU host. No vendor SDK or vendor implementation is embedded here.

use std::ffi::c_char;
use std::sync::Arc;

use crate::clips::ClipImage;
use crate::ffi::*;
use crate::instance::Instance;
use crate::props::PropSet;

#[cfg(windows)]
mod win;

/// Only advertise an acceleration API after a hardware context and required functions work.
pub fn available() -> bool {
    #[cfg(windows)]
    {
        win::available()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Add the negotiated render mode to all render/sequence action arguments.
pub fn action_args(args: &PropSet) {
    args.put_int(IE_OPENGL_ENABLED, 0);
    #[cfg(windows)]
    win::action_args(args);
}

/// Try the GPU when the instance opts in. `Ok(false)` requests the ordinary CPU path.
/// GPU-only effects return a useful error when hardware is unavailable. License failures are
/// propagated unchanged: falling back must never turn an unlicensed result into success.
pub fn render(inst: &Instance, time: f64, window: OfxRectI, scale: f64, out: &Arc<ClipImage>) -> Result<bool, String> {
    #[cfg(not(windows))]
    let _ = (time, window, scale, out);
    let support = inst.obj.props.str(IE_OPENGL_RENDER_SUPPORTED).unwrap_or_default();
    if support != "true" && support != "needed" {
        trace(&inst.entry.ofx_id, "cpu", K_STAT_OK, &format!("Instance OpenGL support: {support}"));
        return Ok(false);
    }
    let required = support == "needed" || inst.obj.props.str("OfxImageEffectPropCPURenderSupported").as_deref() == Some("false");
    let result = if available() {
        #[cfg(windows)]
        {
            win::render(inst, time, window, scale, out)
        }
        #[cfg(not(windows))]
        {
            Err((K_STAT_ERR_MISSING_HOST_FEATURE, "Native OpenGL rendering is unavailable".to_string()))
        }
    } else {
        Err((K_STAT_ERR_MISSING_HOST_FEATURE, "No supported hardware OpenGL context is available".to_string()))
    };
    match result {
        Ok(()) => {
            trace(&inst.entry.ofx_id, "opengl", K_STAT_OK, "");
            Ok(true)
        }
        Err((K_STAT_UNLICENSED, _)) => Err("the plug-in reports it is unlicensed".into()),
        Err((status, reason)) if required => Err(format!("GPU-only plug-in could not render (OFX status {status}): {reason}")),
        Err((status, reason)) => {
            log::warn!("openfx: {} GPU render failed ({status}: {reason}); retrying on CPU", inst.entry.ofx_id);
            trace(&inst.entry.ofx_id, "cpu-fallback", status, &reason);
            Ok(false)
        }
    }
}

/// Optional bounded JSONL diagnostics, including whether a GPU attempt actually succeeded.
/// This is opt-in because normal preview rendering should not write a trace for every frame.
fn trace(effect: &str, backend: &str, status: OfxStatus, reason: &str) {
    use std::io::Write;
    let Some(path) = std::env::var_os("AFTERCRAFT_OFX_GPU_TRACE").map(std::path::PathBuf::from).filter(|p| p.is_absolute()) else {
        return;
    };
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > 16 * 1024 * 1024) {
        return;
    }
    let record = serde_json::json!({"effect":effect,"backend":backend,"status":status,"reason":reason});
    if let Ok(mut bytes) = serde_json::to_vec(&record) {
        bytes.push(b'\n');
        if let Ok(mut file) = std::fs::OpenOptions::new().append(true).create(true).open(path) {
            let _ = file.write_all(&bytes);
        }
    }
}

extern "C" fn clip_load_texture(clip: Handle, time: f64, format: *const c_char, region: *const OfxRectD, texture: *mut Handle) -> OfxStatus {
    guard(|| {
        if texture.is_null() {
            return K_STAT_ERR_BAD_HANDLE;
        }
        // SAFETY: the plug-in supplied an out-pointer, which is cleared even on failure.
        unsafe {
            *texture = std::ptr::null_mut();
        }
        #[cfg(windows)]
        {
            win::load_texture(clip, time, format, region, texture)
        }
        #[cfg(not(windows))]
        {
            let _ = (clip, time, format, region);
            K_STAT_ERR_UNSUPPORTED
        }
    })
}

extern "C" fn clip_free_texture(texture: Handle) -> OfxStatus {
    guard(|| {
        #[cfg(windows)]
        {
            win::free_texture(texture)
        }
        #[cfg(not(windows))]
        {
            let _ = texture;
            K_STAT_ERR_UNSUPPORTED
        }
    })
}

extern "C" fn flush_resources() -> OfxStatus {
    K_STAT_REPLY_DEFAULT
}

pub static SUITE: OfxImageEffectOpenGLRenderSuiteV1 = OfxImageEffectOpenGLRenderSuiteV1 { clip_load_texture, clip_free_texture, flush_resources };

/// GPU diagnostic information is queried without loading any vendor library.
pub fn info() -> serde_json::Value {
    #[cfg(windows)]
    {
        win::info()
    }
    #[cfg(not(windows))]
    {
        serde_json::json!({"available":false,"backend":"none","reason":"Native GPU host currently supports Windows OpenGL"})
    }
}
