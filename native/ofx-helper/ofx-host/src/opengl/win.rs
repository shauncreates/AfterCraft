//! WGL context ownership and the OpenFX texture callbacks. No window is ever shown.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, c_char, c_void};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Gdi::{GetDC, HDC, ReleaseDC};
use windows_sys::Win32::Graphics::OpenGL::*;
use windows_sys::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, WS_POPUP};

use crate::clips::{ClipImage, ClipObj, Comps, Depth};
use crate::ffi::*;
use crate::instance::{EffectObj, Instance};
use crate::props::PropSet;

const FRAMEBUFFER: u32 = 0x8D40;
const COLOR_ATTACHMENT0: u32 = 0x8CE0;
const FRAMEBUFFER_COMPLETE: u32 = 0x8CD5;
const MAX_TEXTURE_BYTES: usize = 512 * 1024 * 1024;
const MAX_HANDLES: usize = 128;

type GenFramebuffers = unsafe extern "system" fn(i32, *mut u32);
type BindFramebuffer = unsafe extern "system" fn(u32, u32);
type FramebufferTexture2D = unsafe extern "system" fn(u32, u32, u32, u32, i32);
type CheckFramebufferStatus = unsafe extern "system" fn(u32) -> u32;
type DeleteFramebuffers = unsafe extern "system" fn(i32, *const u32);
type BindBuffer = unsafe extern "system" fn(u32, u32);

#[derive(Clone, Copy)]
struct Functions {
    gen_framebuffers: GenFramebuffers,
    bind_framebuffer: BindFramebuffer,
    framebuffer_texture_2d: FramebufferTexture2D,
    check_framebuffer_status: CheckFramebufferStatus,
    delete_framebuffers: DeleteFramebuffers,
    bind_buffer: BindBuffer,
}

impl Functions {
    fn load() -> Result<Self, String> {
        macro_rules! proc {
            ($name:literal, $t:ty) => {{
                // SAFETY: the current context owns these standard OpenGL entry points. Each
                // concrete function signature below matches the public OpenGL API.
                let p = unsafe { wglGetProcAddress(concat!($name, "\0").as_ptr()) }.ok_or(concat!("Missing ", $name))?;
                if (p as usize) <= 3 || (p as usize) == usize::MAX {
                    return Err(concat!("Invalid ", $name).into());
                }
                unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, $t>(p) }
            }};
        }
        Ok(Self {
            gen_framebuffers: proc!("glGenFramebuffers", GenFramebuffers),
            bind_framebuffer: proc!("glBindFramebuffer", BindFramebuffer),
            framebuffer_texture_2d: proc!("glFramebufferTexture2D", FramebufferTexture2D),
            check_framebuffer_status: proc!("glCheckFramebufferStatus", CheckFramebufferStatus),
            delete_framebuffers: proc!("glDeleteFramebuffers", DeleteFramebuffers),
            bind_buffer: proc!("glBindBuffer", BindBuffer),
        })
    }
}

struct Context {
    window: HWND,
    dc: HDC,
    rc: HGLRC,
    previous_rc: HGLRC,
    previous_dc: HDC,
}

impl Context {
    fn new() -> Result<Self, String> {
        // SAFETY: system STATIC class, private invisible window, no borrowed userdata. All
        // Win32/WGL handles are owned by this object and released on every error path.
        unsafe {
            let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
            let window = CreateWindowExW(0, class.as_ptr(), class.as_ptr(), WS_POPUP, 0, 0, 1, 1, null_mut(), null_mut(), null_mut(), null());
            if window.is_null() {
                return Err("Cannot create private OpenGL window".into());
            }
            let mut ctx = Self { window, dc: null_mut(), rc: null_mut(), previous_rc: wglGetCurrentContext(), previous_dc: wglGetCurrentDC() };
            ctx.dc = GetDC(window);
            if ctx.dc.is_null() {
                return Err("Cannot acquire OpenGL device context".into());
            }
            let mut pixel: PIXELFORMATDESCRIPTOR = std::mem::zeroed();
            pixel.nSize = std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16;
            pixel.nVersion = 1;
            pixel.dwFlags = PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL;
            pixel.iPixelType = PFD_TYPE_RGBA;
            pixel.cColorBits = 32;
            pixel.cAlphaBits = 8;
            pixel.iLayerType = PFD_MAIN_PLANE as u8;
            let format = ChoosePixelFormat(ctx.dc, &pixel);
            if format == 0 || SetPixelFormat(ctx.dc, format, &pixel) == 0 {
                return Err("No suitable OpenGL pixel format".into());
            }
            ctx.rc = wglCreateContext(ctx.dc);
            if ctx.rc.is_null() || wglMakeCurrent(ctx.dc, ctx.rc) == 0 {
                return Err("Cannot activate hardware OpenGL context".into());
            }
            let renderer = string(GL_RENDERER)?;
            if renderer.to_ascii_lowercase().contains("gdi generic") || renderer.to_ascii_lowercase().contains("llvmpipe") {
                return Err(format!("Software OpenGL renderer is not GPU acceleration: {renderer}"));
            }
            Functions::load()?;
            Ok(ctx)
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: only private handles; restore the calling thread's previous context first.
        unsafe {
            if !self.rc.is_null() {
                wglMakeCurrent(self.previous_dc, self.previous_rc);
                wglDeleteContext(self.rc);
            }
            if !self.dc.is_null() {
                ReleaseDC(self.window, self.dc);
            }
            DestroyWindow(self.window);
        }
    }
}

fn string(name: u32) -> Result<String, String> {
    // SAFETY: glGetString returns a context-owned NUL-terminated string.
    let ptr = unsafe { glGetString(name) };
    if ptr.is_null() {
        return Err("OpenGL returned no renderer information".into());
    }
    Ok(unsafe { CStr::from_ptr(ptr.cast()) }.to_string_lossy().into_owned())
}

fn disabled() -> bool {
    std::env::var("AFTERCRAFT_OFX_GPU").is_ok_and(|s| matches!(s.trim().to_ascii_lowercase().as_str(), "0" | "off" | "false" | "cpu"))
}

fn probe() -> &'static Result<(String, String, String), String> {
    static PROBE: OnceLock<Result<(String, String, String), String>> = OnceLock::new();
    PROBE.get_or_init(|| {
        let _lock = gpu_lock().lock().unwrap_or_else(PoisonError::into_inner);
        let _context = Context::new()?;
        Ok((string(GL_VENDOR)?, string(GL_RENDERER)?, string(GL_VERSION)?))
    })
}

pub fn available() -> bool {
    !disabled() && probe().is_ok()
}

pub fn info() -> serde_json::Value {
    match probe() {
        Ok((vendor, renderer, version)) => {
            serde_json::json!({"available":!disabled(),"backend":"opengl","vendor":vendor,"renderer":renderer,"version":version,"disabled":disabled()})
        }
        Err(reason) => serde_json::json!({"available":false,"backend":"opengl","reason":reason}),
    }
}

fn gpu_lock() -> &'static Mutex<()> {
    static LOCK: Mutex<()> = Mutex::new(());
    &LOCK
}

#[repr(C)]
struct TextureHandle {
    props: PropSet,
    texture: u32,
    owned: bool,
    bytes: usize,
}

struct State {
    functions: Functions,
    texture: u32,
    fbo: u32,
    handles: HashMap<usize, Box<TextureHandle>>,
    bytes: usize,
}

impl Drop for State {
    fn drop(&mut self) {
        // SAFETY: the render's context is still current when State is dropped. The remaining
        // input handles were not freed by the plug-in, so reclaim them before destroying WGL.
        unsafe {
            for h in self.handles.values() {
                if h.owned {
                    glDeleteTextures(1, &h.texture);
                }
            }
            (self.functions.delete_framebuffers)(1, &self.fbo);
            glDeleteTextures(1, &self.texture);
        }
    }
}

thread_local! { static FRAME: RefCell<Option<State>> = const { RefCell::new(None) }; }

struct Frame;
impl Drop for Frame {
    fn drop(&mut self) {
        FRAME.with(|f| {
            f.borrow_mut().take();
        });
    }
}

fn pixel_format(comps: Comps) -> u32 {
    match comps {
        Comps::Rgba => GL_RGBA,
        Comps::Rgb => GL_RGB,
        Comps::Alpha => GL_ALPHA,
    }
}
fn pixel_type(depth: Depth) -> u32 {
    match depth {
        Depth::Byte => GL_UNSIGNED_BYTE,
        Depth::Short => GL_UNSIGNED_SHORT,
        Depth::Float => GL_FLOAT,
    }
}
fn internal_format(depth: Depth, comps: Comps) -> i32 {
    match (depth, comps) {
        (Depth::Float, Comps::Rgba) => 0x8814,
        (Depth::Float, Comps::Rgb) => 0x8815,
        (Depth::Float, Comps::Alpha) => 0x8816,
        (Depth::Short, Comps::Rgba) => 0x805B,
        (Depth::Short, Comps::Rgb) => 0x8054,
        (Depth::Short, Comps::Alpha) => 0x803E,
        (Depth::Byte, Comps::Rgba) => 0x8058,
        (Depth::Byte, Comps::Rgb) => 0x8051,
        (Depth::Byte, Comps::Alpha) => 0x803C,
    }
}

fn image_bytes(image: &ClipImage) -> Option<usize> {
    image.width().checked_mul(image.height())?.checked_mul(image.pixel_bytes())
}

fn clear_pixel_store(functions: Functions, pack: bool) {
    // SAFETY: reset pack/unpack PBO and strides before using CPU pointers. A plug-in can leave
    // arbitrary GL state behind; retaining its PBO would reinterpret a CPU address as an offset.
    unsafe {
        (functions.bind_buffer)(if pack { 0x88EB } else { 0x88EC }, 0);
        for (p, value) in if pack {
            [(GL_PACK_ALIGNMENT, 1), (GL_PACK_ROW_LENGTH, 0), (GL_PACK_SKIP_ROWS, 0), (GL_PACK_SKIP_PIXELS, 0), (GL_PACK_SWAP_BYTES, 0)]
        } else {
            [(GL_UNPACK_ALIGNMENT, 1), (GL_UNPACK_ROW_LENGTH, 0), (GL_UNPACK_SKIP_ROWS, 0), (GL_UNPACK_SKIP_PIXELS, 0), (GL_UNPACK_SWAP_BYTES, 0)]
        } {
            glPixelStorei(p, value);
        }
    }
}

fn make_texture(image: &ClipImage, functions: Functions) -> Result<u32, OfxStatus> {
    let bytes = image_bytes(image).filter(|n| *n > 0 && *n <= MAX_TEXTURE_BYTES).ok_or(K_STAT_GPU_OUT_OF_MEMORY)?;
    let _ = bytes;
    let mut max_size = 0;
    // SAFETY: a context is current and the image buffer is host-owned for this entire call.
    unsafe {
        glGetIntegerv(GL_MAX_TEXTURE_SIZE, &mut max_size);
        if image.width() > max_size.max(0) as usize || image.height() > max_size.max(0) as usize {
            return Err(K_STAT_GPU_OUT_OF_MEMORY);
        }
        clear_pixel_store(functions, false);
        // Bound the driver error drain even if a broken driver repeatedly returns an error.
        for _ in 0..16 {
            if glGetError() == GL_NO_ERROR {
                break;
            }
        }
        let mut id = 0;
        glGenTextures(1, &mut id);
        if id == 0 {
            return Err(K_STAT_GPU_OUT_OF_MEMORY);
        }
        glBindTexture(GL_TEXTURE_2D, id);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR as i32);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR as i32);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, 0x812F);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, 0x812F);
        glTexImage2D(
            GL_TEXTURE_2D,
            0,
            internal_format(image.depth, image.comps),
            image.width() as i32,
            image.height() as i32,
            0,
            pixel_format(image.comps),
            pixel_type(image.depth),
            image.buf.ptr().cast(),
        );
        let status = glGetError();
        if status != GL_NO_ERROR {
            glDeleteTextures(1, &id);
            return Err(if status == GL_OUT_OF_MEMORY { K_STAT_GPU_OUT_OF_MEMORY } else { K_STAT_GPU_RENDER_FAILED });
        }
        Ok(id)
    }
}

pub fn action_args(args: &PropSet) {
    FRAME.with(|f| {
        if let Some(s) = f.borrow().as_ref() {
            args.put_int(IE_OPENGL_ENABLED, 1);
            args.put_int(GL_TEXTURE_INDEX, s.texture as i32);
            args.put_int(GL_TEXTURE_TARGET, GL_TEXTURE_2D as i32);
        }
    });
}

pub fn render(inst: &Instance, time: f64, window: OfxRectI, scale: f64, out: &Arc<ClipImage>) -> Result<(), (OfxStatus, String)> {
    let fail = |s: OfxStatus| (s, format!("OpenGL operation failed (OFX status {s})"));
    let _lock = gpu_lock().lock().unwrap_or_else(PoisonError::into_inner);
    let _context = Context::new().map_err(|e| (K_STAT_ERR_MISSING_HOST_FEATURE, e))?;
    let functions = Functions::load().map_err(|e| (K_STAT_ERR_MISSING_HOST_FEATURE, e))?;
    let depths = inst.obj.props.strs("OfxOpenGLPropPixelDepth");
    if !depths.is_empty() && !depths.iter().any(|d| d == out.depth.name()) {
        return Err((K_STAT_ERR_IMAGE_FORMAT, "Plug-in GPU pixel depths do not include the negotiated clip format".into()));
    }
    if FRAME.with(|f| f.borrow().is_some()) {
        return Err((K_STAT_GPU_RENDER_FAILED, "Nested OpenGL renders are not supported".into()));
    }
    let texture = make_texture(out, functions).map_err(fail)?;
    let mut state = State { functions, texture, fbo: 0, handles: HashMap::new(), bytes: image_bytes(out).unwrap_or(MAX_TEXTURE_BYTES) };
    // SAFETY: attach the owned output texture to a private framebuffer, never a default/user FBO.
    unsafe {
        (functions.gen_framebuffers)(1, &mut state.fbo);
        (functions.bind_framebuffer)(FRAMEBUFFER, state.fbo);
        (functions.framebuffer_texture_2d)(FRAMEBUFFER, COLOR_ATTACHMENT0, GL_TEXTURE_2D, texture, 0);
        if state.fbo == 0 || (functions.check_framebuffer_status)(FRAMEBUFFER) != FRAMEBUFFER_COMPLETE {
            return Err(fail(K_STAT_GPU_RENDER_FAILED));
        }
        glViewport(0, 0, out.width() as i32, out.height() as i32);
    }
    FRAME.with(|f| {
        *f.borrow_mut() = Some(state);
    });
    let _frame = Frame;
    inst.obj.props.put_int(IE_OPENGL_ENABLED, 1);
    let attached = inst.entry.call(ACT_GL_ATTACHED, inst.handle(), None, None);
    if attached != K_STAT_OK && attached != K_STAT_REPLY_DEFAULT {
        inst.obj.props.put_int(IE_OPENGL_ENABLED, 0);
        return Err(fail(attached));
    }
    let begin = inst.begin_sequence(time, scale);
    let status = if begin == K_STAT_OK || begin == K_STAT_REPLY_DEFAULT { inst.render(time, window, scale) } else { begin };
    let end = inst.end_sequence(time, scale);
    // Read the host's output texture, regardless of which framebuffer a plug-in left bound.
    let readback = if status == K_STAT_OK || status == K_STAT_REPLY_DEFAULT {
        unsafe {
            clear_pixel_store(functions, true);
            glBindTexture(GL_TEXTURE_2D, texture);
            glGetTexImage(GL_TEXTURE_2D, 0, pixel_format(out.comps), pixel_type(out.depth), out.buf.ptr().cast());
            glFinish();
            glGetError() == GL_NO_ERROR
        }
    } else {
        false
    };
    let detached = inst.entry.call(ACT_GL_DETACHED, inst.handle(), None, None);
    inst.obj.props.put_int(IE_OPENGL_ENABLED, 0);
    for status in [status, end, detached] {
        if status != K_STAT_OK && status != K_STAT_REPLY_DEFAULT {
            return Err(fail(status));
        }
    }
    if !readback {
        return Err(fail(K_STAT_GPU_RENDER_FAILED));
    }
    log::debug!("openfx: {} rendered on hardware OpenGL", inst.entry.ofx_id);
    Ok(())
}

pub fn load_texture(clip: Handle, time: f64, format: *const c_char, _region: *const OfxRectD, out: *mut Handle) -> OfxStatus {
    // SAFETY: OFX clip handles have the same validation as clipGetImage.
    let Some(c) = (unsafe { ClipObj::from_handle(clip) }) else {
        return K_STAT_ERR_BAD_HANDLE;
    };
    let Some(ctx) = (unsafe { c.owner.as_ref() }).and_then(EffectObj::current_render) else {
        return K_STAT_FAILED;
    };
    let Some(image) = ctx.image(&c.name, time) else {
        return K_STAT_FAILED;
    };
    let is_output = c.name == CLIP_OUTPUT;
    // The OpenFX contract explicitly ignores requested format for the Output clip.
    let requested = if is_output || format.is_null() {
        image.depth
    } else {
        let Some(name) = (unsafe { str_arg(format) }) else {
            return K_STAT_ERR_VALUE;
        };
        let Some(depth) = Depth::from_name(name) else {
            return K_STAT_ERR_IMAGE_FORMAT;
        };
        depth
    };
    // Output format is fixed by the host. Input conversion is bounded by the same frame budget.
    let image = if !is_output && requested != image.depth {
        let Some(n) = image.width().checked_mul(image.height()).filter(|n| *n <= MAX_TEXTURE_BYTES / 16) else {
            return K_STAT_GPU_OUT_OF_MEMORY;
        };
        let mut pixels = vec![[0.0; 4]; n];
        image.write_rows(&mut pixels, false);
        Arc::new(ClipImage::from_rows(&pixels, image.width(), (0, 0, image.width(), image.height()), image.bounds, image.rod, requested, image.comps))
    } else {
        image
    };
    FRAME.with(|f| {
        let mut f = f.borrow_mut();
        let Some(s) = f.as_mut() else {
            return K_STAT_ERR_UNSUPPORTED;
        };
        if s.handles.len() >= MAX_HANDLES {
            return K_STAT_GPU_OUT_OF_MEMORY;
        }
        let Some(bytes) = image_bytes(&image).filter(|n| is_output || s.bytes.checked_add(*n).is_some_and(|b| b <= MAX_TEXTURE_BYTES)) else {
            return K_STAT_GPU_OUT_OF_MEMORY;
        };
        let texture = if is_output {
            s.texture
        } else {
            match make_texture(&image, s.functions) {
                Ok(id) => id,
                Err(status) => return status,
            }
        };
        if is_output {
            // SAFETY: host-owned framebuffer and texture, with a current private context.
            unsafe {
                (s.functions.bind_framebuffer)(FRAMEBUFFER, s.fbo);
            }
        } else {
            s.bytes += bytes;
        }
        let props = PropSet::new();
        props.put_str(P_TYPE, TYPE_IMAGE);
        props.put_int(GL_TEXTURE_INDEX, texture as i32);
        props.put_int(GL_TEXTURE_TARGET, GL_TEXTURE_2D as i32);
        props.put_ints(IMG_BOUNDS, &[image.bounds.x1, image.bounds.y1, image.bounds.x2, image.bounds.y2]);
        props.put_ints(IMG_ROD, &[image.rod.x1, image.rod.y1, image.rod.x2, image.rod.y2]);
        props.put_int(IMG_ROW_BYTES, image.row_bytes);
        props.put_str(IMG_FIELD, FIELD_NONE);
        props.put_str(IMG_UNIQUE_ID, &format!("aftercraft-gpu-{}", crate::instance::RenderCtx::next_image_id()));
        props.put_double(IMG_PIXEL_ASPECT_RATIO, 1.0);
        props.put_str(IE_PIXEL_DEPTH, image.depth.name());
        props.put_str(IE_COMPONENTS, image.comps.name());
        props.put_str(IE_PREMULTIPLICATION, image.premult);
        props.put_doubles(IE_RENDER_SCALE, &[ctx.geom.scale, ctx.geom.scale]);
        let handle = Box::new(TextureHandle { props, texture, owned: !is_output, bytes });
        let ptr = &*handle as *const TextureHandle as usize;
        s.handles.insert(ptr, handle);
        // SAFETY: props is the first repr(C) field and remains stable until clipFreeTexture.
        unsafe {
            *out = ptr as *mut c_void;
        }
        K_STAT_OK
    })
}

pub fn free_texture(handle: Handle) -> OfxStatus {
    FRAME.with(|f| {
        let mut f = f.borrow_mut();
        let Some(s) = f.as_mut() else {
            return K_STAT_ERR_BAD_HANDLE;
        };
        let Some(h) = s.handles.remove(&(handle as usize)) else {
            return K_STAT_ERR_BAD_HANDLE;
        };
        if h.owned {
            // SAFETY: current render owns this input texture and removes its handle once.
            unsafe {
                glDeleteTextures(1, &h.texture);
            }
            s.bytes = s.bytes.saturating_sub(h.bytes);
        }
        K_STAT_OK
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_texture_roundtrip_preserves_hdr_rgba_and_orientation() {
        if !available() {
            assert!(std::env::var_os("AFTERCRAFT_REQUIRE_GPU_TESTS").is_none(), "GPU required but unavailable: {}", info());
            return;
        }
        let _lock = gpu_lock().lock().unwrap();
        let _context = Context::new().unwrap();
        let functions = Functions::load().unwrap();
        let pixels = [[0.1, 0.2, 0.3, 0.4], [2.5, -0.2, 0.0, 1.0], [0.8, 0.4, 0.2, 1.0], [0.0; 4]];
        let b = OfxRectI { x1: -3, y1: 7, x2: -1, y2: 9 };
        let image = ClipImage::from_rows(&pixels, 2, (0, 0, 2, 2), b, b, Depth::Float, Comps::Rgba);
        let texture = make_texture(&image, functions).unwrap();
        let result = ClipImage::blank(b, b, Depth::Float, Comps::Rgba);
        unsafe {
            clear_pixel_store(functions, true);
            glGetTexImage(GL_TEXTURE_2D, 0, GL_RGBA, GL_FLOAT, result.buf.ptr().cast());
            glDeleteTextures(1, &texture);
            assert_eq!(glGetError(), GL_NO_ERROR);
        }
        let mut output = [[0.0; 4]; 4];
        result.write_rows(&mut output, false);
        assert_eq!(pixels, output);
        assert_eq!(free_texture(1usize as Handle), K_STAT_ERR_BAD_HANDLE);
    }
}
