//! Original, tiny GPU conformance fixtures. They contain no vendor implementation.
use super::*;
use std::sync::atomic::AtomicBool;

static ATTACHED: AtomicBool = AtomicBool::new(false);

#[repr(C)]
struct OpenGLSuite {
    load: unsafe extern "C" fn(Clip, f64, *const c_char, *const RectD, *mut Props) -> Status,
    free: unsafe extern "C" fn(Props) -> Status,
    _flush: unsafe extern "C" fn() -> Status,
}

#[cfg(windows)]
#[link(name = "opengl32")]
unsafe extern "system" {
    fn glGetString(name: u32) -> *const u8;
    fn glGetError() -> u32;
    fn glBindTexture(target: u32, id: u32);
    fn glCopyTexSubImage2D(target: u32, level: i32, xo: i32, yo: i32, x: i32, y: i32, w: i32, h: i32);
    fn wglGetProcAddress(name: *const u8) -> Option<unsafe extern "system" fn() -> isize>;
}

pub fn attached() -> Status {
    #[cfg(windows)]
    unsafe {
        if glGetString(0x1F02).is_null() || ATTACHED.swap(true, Ordering::SeqCst) {
            return FAILED;
        }
        return OK;
    }
    #[cfg(not(windows))]
    {
        ERR_UNSUPPORTED
    }
}

pub fn detached() -> Status {
    #[cfg(windows)]
    unsafe {
        if glGetString(0x1F02).is_null() || !ATTACHED.swap(false, Ordering::SeqCst) {
            return FAILED;
        }
        return OK;
    }
    #[cfg(not(windows))]
    {
        ERR_UNSUPPORTED
    }
}

pub unsafe fn render(k: usize, effect: Effect, args: Props) -> Status {
    if !ATTACHED.load(Ordering::SeqCst) {
        return FAILED;
    }
    if k == GPU_FALLBACK {
        return 1002;
    }
    if k == GPU_UNLICENSED {
        return 15;
    }
    #[cfg(windows)]
    unsafe {
        let Some(host) = HOST.load(Ordering::SeqCst).as_ref() else {
            return FAILED;
        };
        let Some(gl) = ((host.fetch_suite)(host.host, c"OfxImageEffectOpenGLRenderSuite".as_ptr(), 1) as *const OpenGLSuite).as_ref() else {
            return ERR_UNSUPPORTED;
        };
        let s = suites();
        let time = s.get_dbl(args, c"OfxPropTime", 0).unwrap_or(0.0);
        let (mut source, mut output) = (null_mut(), null_mut());
        if (gl.load)(clip(effect, c"Source"), time, null(), null(), &mut source) != OK {
            return FAILED;
        }
        if (gl.load)(clip(effect, c"Output"), time, c"ignored-output-format".as_ptr(), null(), &mut output) != OK {
            (gl.free)(source);
            return FAILED;
        }
        let source_texture = s.get_int(source, c"OfxImageEffectPropOpenGLTextureIndex", 0).unwrap_or(0) as u32;
        let output_texture = s.get_int(output, c"OfxImageEffectPropOpenGLTextureIndex", 0).unwrap_or(0) as u32;
        if source_texture == 0 || output_texture == 0 {
            (gl.free)(source);
            (gl.free)(output);
            return FAILED;
        }
        type Gen = unsafe extern "system" fn(i32, *mut u32);
        type Bind = unsafe extern "system" fn(u32, u32);
        type Attach = unsafe extern "system" fn(u32, u32, u32, u32, i32);
        type Delete = unsafe extern "system" fn(i32, *const u32);
        let Some(gen_p) = wglGetProcAddress(c"glGenFramebuffers".as_ptr().cast()) else {
            return FAILED;
        };
        let Some(bind_p) = wglGetProcAddress(c"glBindFramebuffer".as_ptr().cast()) else {
            return FAILED;
        };
        let Some(attach_p) = wglGetProcAddress(c"glFramebufferTexture2D".as_ptr().cast()) else {
            return FAILED;
        };
        let Some(delete_p) = wglGetProcAddress(c"glDeleteFramebuffers".as_ptr().cast()) else {
            return FAILED;
        };
        let gen_fb = std::mem::transmute::<unsafe extern "system" fn() -> isize, Gen>(gen_p);
        let bind_fb = std::mem::transmute::<unsafe extern "system" fn() -> isize, Bind>(bind_p);
        let attach = std::mem::transmute::<unsafe extern "system" fn() -> isize, Attach>(attach_p);
        let delete_fb = std::mem::transmute::<unsafe extern "system" fn() -> isize, Delete>(delete_p);
        let mut fbo = 0;
        gen_fb(1, &mut fbo);
        bind_fb(0x8D40, fbo);
        attach(0x8D40, 0x8CE0, 0x0DE1, source_texture, 0);
        glBindTexture(0x0DE1, output_texture);
        let w = s.get_int(source, c"OfxImagePropBounds", 2).unwrap_or(0) - s.get_int(source, c"OfxImagePropBounds", 0).unwrap_or(0);
        let h = s.get_int(source, c"OfxImagePropBounds", 3).unwrap_or(0) - s.get_int(source, c"OfxImagePropBounds", 1).unwrap_or(0);
        glCopyTexSubImage2D(0x0DE1, 0, 0, 0, 0, 0, w, h);
        let error = glGetError();
        delete_fb(1, &fbo);
        if (gl.free)(source) != OK || (gl.free)(output) != OK {
            return FAILED;
        }
        if (gl.free)(source) != 9 {
            return FAILED;
        } // double frees must be rejected
        return if error == 0 { OK } else { 1002 };
    }
    #[cfg(not(windows))]
    {
        let _ = (effect, args);
        ERR_UNSUPPORTED
    }
}
