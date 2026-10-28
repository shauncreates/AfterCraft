//! Native ABI code is confined to this separate diagnostic executable, outside AfterCraft.
//! The worker is crash containment, NOT an OS security sandbox. Only load trusted plugins.
use after_effects_sys as ae;
use serde_json::json;
use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char, c_void};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr;
use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::System::Diagnostics::Debug::{EXCEPTION_POINTERS, SetUnhandledExceptionFilter};
use windows_sys::Win32::System::Diagnostics::Debug::{SEM_FAILCRITICALERRORS, SEM_NOGPFAULTERRORBOX, SetErrorMode};
use windows_sys::Win32::System::LibraryLoader::*;

use crate::{ansi, application, event, frame, memory, pipl, utility, worlds};

thread_local! { static PARAMS: RefCell<Vec<ae::PF_ParamDef>> = const { RefCell::new(Vec::new()) }; }
thread_local! { static CURRENT_PARAMS: RefCell<Vec<ae::PF_ParamDef>> = const { RefCell::new(Vec::new()) }; }
thread_local! { static CURRENT_TIME: RefCell<i32> = const { RefCell::new(0) }; }
thread_local! { static CURRENT_SCALE: RefCell<u32> = const { RefCell::new(25) }; }
pub fn same_time(time: i32, scale: u32) -> bool {
    scale != 0 && CURRENT_TIME.with(|t| CURRENT_SCALE.with(|s| i64::from(time) * i64::from(*s.borrow()) == i64::from(*t.borrow()) * i64::from(scale)))
}
pub fn layer_world(index: i32) -> Option<ae::PF_EffectWorld> {
    CURRENT_PARAMS.with(|params| {
        let params = params.try_borrow().ok()?;
        let def = usize::try_from(index).ok().and_then(|i| params.get(i))?;
        if def.param_type != ae::PF_Param_LAYER {
            return None;
        }
        Some(unsafe { def.u.ld })
    })
}
unsafe extern "C" fn checkout(_effect: ae::PF_ProgPtr, index: i32, time: i32, _step: i32, scale: u32, out: *mut ae::PF_ParamDef) -> ae::PF_Err {
    let same_time =
        scale != 0 && CURRENT_TIME.with(|t| CURRENT_SCALE.with(|s| i64::from(time) * i64::from(*s.borrow()) == i64::from(*t.borrow()) * i64::from(scale)));
    if out.is_null() || !same_time {
        event(json!({"stage":"unsupported_checkout","index":index,"time":time,"scale":scale}));
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    CURRENT_PARAMS.with(|params| {
        let params = params.borrow();
        let Some(def) = usize::try_from(index).ok().and_then(|i| params.get(i)) else {
            return ae::PF_Err_INVALID_INDEX;
        };
        unsafe {
            *out = *def;
        }
        0
    })
}
unsafe extern "C" fn checkin(_effect: ae::PF_ProgPtr, _param: *mut ae::PF_ParamDef) -> ae::PF_Err {
    0
}
thread_local! {
    static PLUGIN_PATH: RefCell<String> = const { RefCell::new(String::new()) };
    static PLUGIN_MODULE: RefCell<HMODULE> = const { RefCell::new(ptr::null_mut()) };
    static CUSTOM_UI: RefCell<Option<ae::PF_CustomUIInfo>> = const { RefCell::new(None) };
}
unsafe extern "C" fn add_param(_effect: ae::PF_ProgPtr, index: i32, def: *mut ae::PF_ParamDef) -> ae::PF_Err {
    if def.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let mut def = unsafe { *def };
    let name: Vec<u8> = def.name_do_not_use_directly.iter().take_while(|c| **c != 0).map(|c| *c as u8).collect();
    let numeric = match crate::param_info::capture(&mut def) {
        Ok(control) => control,
        Err(code) => return code,
    };
    event(
        json!({"stage":"parameter","index":index,"diskId":unsafe {def.uu.id},"name":String::from_utf8_lossy(&name),"type":def.param_type,"numeric":numeric,"hidden":def.ui_flags & ae::PF_PUI_INVISIBLE != 0}),
    );
    PARAMS.with(|params| {
        let Ok(mut params) = params.try_borrow_mut() else {
            return ae::PF_Err_BAD_CALLBACK_PARAM;
        };
        if params.len() >= 1024 {
            return ae::PF_Err_OUT_OF_MEMORY;
        }
        // PARAM_SETUP convention: -1 appends, otherwise an explicit insertion position (input is 0).
        let position = if index == -1 {
            params.len()
        } else {
            match usize::try_from(index.saturating_sub(1)) {
                Ok(p) if p <= params.len() => p,
                _ => return ae::PF_Err_INVALID_INDEX,
            }
        };
        params.insert(position, def);
        0
    })
}
unsafe extern "C" fn register_ui(_effect: ae::PF_ProgPtr, info: *mut ae::PF_CustomUIInfo) -> ae::PF_Err {
    if info.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let info = unsafe { *info };
    CUSTOM_UI.with(|ui| *ui.borrow_mut() = Some(info));
    event(json!({"stage":"ui_registered", "events":info.events, "headless":true}));
    0
}
unsafe extern "C" fn platform_data(_effect: ae::PF_ProgPtr, which: ae::PF_PlatDataID, data: *mut c_void) -> ae::PF_Err {
    if data.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    if matches!(which, ae::PF_PlatData_EXE_FILE_PATH_DEPRECATED | ae::PF_PlatData_RES_FILE_PATH_DEPRECATED) {
        return PLUGIN_PATH.with(|path| {
            let path = path.borrow();
            // Legacy SDK path buffer is AEFX_MAX_PATH. Refuse non-ASCII rather than guess a codepage.
            if path.len() >= 255 || !path.is_ascii() {
                return ae::PF_Err_BAD_CALLBACK_PARAM;
            }
            unsafe {
                ptr::copy_nonoverlapping(path.as_ptr(), data.cast::<u8>(), path.len());
                *data.cast::<u8>().add(path.len()) = 0;
            }
            event(json!({"stage":"platform_data", "id":which, "supported":true}));
            0
        });
    }
    if matches!(which, ae::PF_PlatData_EXE_FILE_PATH_W | ae::PF_PlatData_RES_FILE_PATH_W) {
        return PLUGIN_PATH.with(|path| {
            let wide: Vec<u16> = path.borrow().encode_utf16().chain(Some(0)).collect();
            if wide.len() > 255 {
                return ae::PF_Err_BAD_CALLBACK_PARAM;
            }
            unsafe {
                ptr::copy_nonoverlapping(wide.as_ptr(), data.cast::<u16>(), wide.len());
            }
            0
        });
    }
    if which == ae::PF_PlatData_RES_DLLINSTANCE {
        PLUGIN_MODULE.with(|module| unsafe {
            *data.cast::<HMODULE>() = *module.borrow();
        });
        return 0;
    }
    if which == ae::PF_PlatData_MAIN_WND {
        unsafe {
            *data.cast::<*mut c_void>() = ptr::null_mut();
        }
        return 0;
    }
    event(json!({"stage":"unsupported_callback", "name":"get_platform_data", "id":which}));
    ae::PF_Err_INVALID_CALLBACK
}
unsafe extern "C" fn callback_addr(
    _effect: ae::PF_ProgPtr,
    _quality: ae::PF_Quality,
    _flags: ae::PF_ModeFlags,
    which: ae::PF_CallbackID,
    out: *mut ae::PF_CallbackFunc,
) -> ae::PF_Err {
    if !out.is_null() {
        unsafe {
            *out = ptr::null_mut();
        }
    }
    event(json!({"stage":"unsupported_callback", "name":"get_callback_addr", "id":which}));
    ae::PF_Err_INVALID_CALLBACK
}
unsafe extern "system" fn exception_filter(info: *const EXCEPTION_POINTERS) -> i32 {
    if !info.is_null() {
        let record = unsafe { (*info).ExceptionRecord };
        if !record.is_null() {
            let record = unsafe { &*record };
            event(
                json!({"stage":"native_exception", "code":record.ExceptionCode, "address":format!("{:p}",record.ExceptionAddress), "accessAddress": record.ExceptionInformation.get(1).copied()}),
            );
        }
    }
    1 // EXCEPTION_EXECUTE_HANDLER: terminate this worker, preserve the supervisor.
}

struct DllDirectory(*mut c_void);
impl Drop for DllDirectory {
    fn drop(&mut self) {
        unsafe {
            RemoveDllDirectory(self.0);
        }
    }
}

struct Module(HMODULE);
impl Drop for Module {
    fn drop(&mut self) {
        unsafe {
            FreeLibrary(self.0);
        }
    }
}
fn wide(path: &Path) -> Result<Vec<u16>, String> {
    let mut s: Vec<u16> = path.as_os_str().encode_wide().collect();
    if s.contains(&0) {
        return Err("path contains NUL".into());
    }
    s.push(0);
    Ok(s)
}
fn module(path: &Path, flags: u32) -> Result<Module, String> {
    let path_w = wide(path)?;
    // The path is absolute. Metadata mode does not run DllMain or resolve imports.
    let m = unsafe { LoadLibraryExW(path_w.as_ptr(), ptr::null_mut(), flags) };
    if m.is_null() {
        return Err(format!("cannot load {}: {}", path.display(), std::io::Error::last_os_error()));
    }
    Ok(Module(m))
}

struct Resources {
    values: Vec<Result<pipl::Descriptor, String>>,
}
unsafe extern "system" fn resource_callback(module: HMODULE, kind: *const u16, name: *const u16, context: isize) -> i32 {
    // context is a live Resources passed to synchronous EnumResourceNamesW.
    let resources = unsafe { &mut *(context as *mut Resources) };
    if resources.values.len() >= 128 {
        return 0;
    }
    let read = || -> Result<pipl::Descriptor, String> {
        let resource = unsafe { FindResourceW(module, name, kind) };
        if resource.is_null() {
            return Err("PiPL resource not found".into());
        }
        let size = unsafe { SizeofResource(module, resource) } as usize;
        if size == 0 || size > 1024 * 1024 {
            return Err("invalid PiPL resource size".into());
        }
        let loaded = unsafe { LoadResource(module, resource) };
        let bytes = unsafe { LockResource(loaded) };
        if bytes.is_null() {
            return Err("cannot read PiPL resource".into());
        }
        // Resource allocation is owned by module and remains valid throughout enumeration.
        pipl::parse(unsafe { std::slice::from_raw_parts(bytes.cast::<u8>(), size) })
    };
    resources.values.push(read());
    1
}

unsafe extern "C" fn acquire(name: *const c_char, version: i32, suite: *mut *const c_void) -> ae::SPErr {
    if !suite.is_null() {
        unsafe {
            *suite = ptr::null();
        }
    }
    if name.is_null() || suite.is_null() {
        return -1;
    }
    // SDK contract requires a NUL-terminated name. A violating plugin can only crash the worker.
    let name = unsafe { CStr::from_ptr(name) }.to_string_lossy();
    let value = match (name.as_ref(), version) {
        ("PF Handle Suite", 2) => &memory::HANDLE_SUITE as *const _ as *const c_void,
        ("PF ANSI Suite", 1) => &ansi::SUITE as *const _ as *const c_void,
        ("PF World Suite", 2) => &worlds::SUITE2 as *const _ as *const c_void,
        ("PF World Suite", 1) => &worlds::SUITE1 as *const _ as *const c_void,
        ("PF Utility Suite", 8) => &utility::SUITE as *const _ as *const c_void,
        ("PF AE App Suite", 6) => &application::SUITE as *const _ as *const c_void,
        ("PF Effect UI Suite", 1) => &crate::effect_ui::SUITE as *const _ as *const c_void,
        ("PF ColorParamSuite", 1) => &crate::value_suites::COLOR as *const _ as *const c_void,
        ("AEGP Persistent Data Suite", 3) => &crate::persistent::SUITE3 as *const _ as *const c_void,
        ("AEGP Memory Suite", 1) => &crate::persistent::MEMORY1 as *const _ as *const c_void,
        ("PF Effect Sequence Data Suite", 1) => &crate::sequence::SUITE as *const _ as *const c_void,
        ("AEGP PF Interface Suite", 1) => &crate::aegp::INTERFACE as *const _ as *const c_void,
        ("AEGP Layer Suite", 11) => &crate::aegp::LAYER as *const _ as *const c_void,
        ("AEGP Layer Suite", 13) => &crate::aegp::LAYER7 as *const _ as *const c_void,
        ("PF Color Settings Suite", 3) => &crate::aegp::COLOR_SETTINGS as *const _ as *const c_void,
        ("PF World Transform Suite", 1) => &crate::aegp::WORLD_TRANSFORM as *const _ as *const c_void,
        ("PF Iterate8 Suite", 1) => &crate::aegp::ITERATE8 as *const _ as *const c_void,
        ("AEGP Stream Suite", 4) => &crate::aegp::STREAM as *const _ as *const c_void,
        ("AEGP Camera Suite", 2) => &crate::aegp::CAMERA as *const _ as *const c_void,
        ("AEGP Utility Suite", 5) => &crate::aegp::UTILITY as *const _ as *const c_void,
        _ => ptr::null(),
    };
    let supported = !value.is_null();
    unsafe {
        *suite = value;
    }
    event(json!({"stage": "suite_requested", "name": name, "version": version, "supported": supported}));
    if supported { 0 } else { -1 }
}
unsafe extern "C" fn release(name: *const c_char, version: i32) -> ae::SPErr {
    if name.is_null() {
        return -1;
    }
    match (unsafe { CStr::from_ptr(name) }.to_bytes(), version) {
        (b"PF Handle Suite", 2)
        | (b"PF ANSI Suite", 1)
        | (b"PF World Suite", 1 | 2)
        | (b"PF Utility Suite", 8)
        | (b"PF AE App Suite", 6)
        | (b"PF Effect UI Suite", 1)
        | (b"PF ColorParamSuite", 1)
        | (b"AEGP Persistent Data Suite", 3)
        | (b"AEGP Memory Suite", 1)
        | (b"AEGP PF Interface Suite", 1)
        | (b"AEGP Layer Suite", 11)
        | (b"AEGP Layer Suite", 13)
        | (b"PF Color Settings Suite", 3)
        | (b"PF World Transform Suite", 1)
        | (b"PF Iterate8 Suite", 1)
        | (b"AEGP Stream Suite", 4)
        | (b"AEGP Camera Suite", 2)
        | (b"AEGP Utility Suite", 5)
        | (b"PF Effect Sequence Data Suite", 1) => 0,
        _ => -1,
    }
}
unsafe extern "C" fn equal(a: *const c_char, b: *const c_char) -> ae::SPBoolean {
    if a.is_null() || b.is_null() {
        return 0;
    }
    i32::from(unsafe { CStr::from_ptr(a) == CStr::from_ptr(b) })
}
unsafe extern "C" fn undefined() -> ae::SPErr {
    -1
}
unsafe extern "C" fn abort(_effect: ae::PF_ProgPtr) -> ae::PF_Err {
    0
}
unsafe extern "C" fn progress(_effect: ae::PF_ProgPtr, _current: i32, _total: i32) -> ae::PF_Err {
    0
}

pub fn worker(
    path: &Path,
    execute: bool,
    dll_dirs: &[std::path::PathBuf],
    render_job: Option<&frame::Job>,
    match_name: Option<&str>,
    host_api: frame::HostApi,
) -> Result<(), String> {
    // Prevent Windows crash/error dialogs from leaving a diagnostic process waiting forever.
    unsafe {
        SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
    }
    unsafe {
        SetUnhandledExceptionFilter(Some(exception_filter));
    }
    if path.extension().and_then(|e| e.to_str()).is_none_or(|e| !e.eq_ignore_ascii_case("aex")) {
        return Err("expected an .aex effect plugin".into());
    }
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    event(json!({"stage": "metadata", "path": path}));
    let metadata = module(&path, LOAD_LIBRARY_AS_DATAFILE | LOAD_LIBRARY_AS_IMAGE_RESOURCE)?;
    let kind: Vec<u16> = "PiPL\0".encode_utf16().collect();
    let mut resources = Resources { values: vec![] };
    unsafe {
        EnumResourceNamesW(metadata.0, kind.as_ptr(), Some(resource_callback), &mut resources as *mut _ as isize);
    }
    if resources.values.is_empty() {
        return Err("no PiPL resource found".into());
    }
    let descriptors: Vec<_> = resources.values.into_iter().collect::<Result<_, _>>()?;
    event(json!({"stage": "descriptors", "effects": descriptors}));
    if !execute {
        event(json!({"stage": "complete", "initializationAttempted": false}));
        return Ok(());
    }
    let effects: Vec<_> =
        descriptors.iter().filter(|d| d.kind == "eFKT" && !d.entry_point.is_empty() && match_name.is_none_or(|name| d.match_name == name)).collect();
    if effects.len() != 1 {
        return Err("probe requires exactly one x64 effect descriptor; other kinds are not invoked".into());
    }
    let descriptor = effects.first().ok_or("missing effect descriptor")?;
    let mut directory_cookies = Vec::new();
    for directory in dll_dirs {
        let directory_w = wide(directory)?;
        let cookie = unsafe { AddDllDirectory(directory_w.as_ptr()) };
        if cookie.is_null() {
            return Err(format!("cannot register DLL directory: {}", std::io::Error::last_os_error()));
        }
        directory_cookies.push(DllDirectory(cookie));
    }
    let symbol = CString::new(descriptor.entry_point.as_bytes()).map_err(|e| e.to_string())?;
    event(json!({"stage": "load", "entryPoint": descriptor.entry_point}));
    // Search only the plugin's directory, application directory, registered directories and System32.
    let native = module(&path, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS)?;
    let plain_path = path.to_string_lossy();
    PLUGIN_PATH.with(|p| *p.borrow_mut() = plain_path.strip_prefix("\\\\?\\").unwrap_or(&plain_path).to_string());
    PLUGIN_MODULE.with(|m| *m.borrow_mut() = native.0);
    let proc = unsafe { GetProcAddress(native.0, symbol.as_ptr().cast()) }.ok_or("PiPL entry point is not exported")?;
    // The descriptor kind and ABI are verified above; the DLL is kept alive until all calls finish.
    let entry: ae::PF_FilterProc = Some(unsafe {
        std::mem::transmute::<
            unsafe extern "system" fn() -> isize,
            unsafe extern "C" fn(ae::PF_Cmd, *mut ae::PF_InData, *mut ae::PF_OutData, ae::PF_ParamList, *mut ae::PF_LayerDef, *mut c_void) -> ae::PF_Err,
        >(proc)
    });
    let entry = entry.ok_or("no effect entry point")?;
    let mut basic = ae::SPBasicSuite {
        AcquireSuite: Some(acquire),
        ReleaseSuite: Some(release),
        IsEqual: Some(equal),
        AllocateBlock: Some(memory::allocate),
        FreeBlock: Some(memory::free),
        ReallocateBlock: Some(memory::reallocate),
        Undefined: Some(undefined),
    };
    // SDK POD structs: zero is valid for reserved fields, nullable pointers and optional callbacks.
    let mut input: ae::PF_InData = unsafe { std::mem::zeroed() };
    let mut output: ae::PF_OutData = unsafe { std::mem::zeroed() };
    let mut utils: ae::_PF_UtilCallbacks = unsafe { std::mem::zeroed() };
    utils.host_new_handle = Some(memory::new_handle);
    utils.host_lock_handle = Some(memory::lock_handle);
    utils.host_unlock_handle = Some(memory::unlock_handle);
    utils.host_dispose_handle = Some(memory::dispose_handle);
    utils.host_get_handle_size = Some(memory::handle_size);
    utils.host_resize_handle = Some(memory::resize_handle);
    utils.ansi = ansi::block();
    utils.new_world = Some(worlds::legacy_new);
    utils.dispose_world = Some(worlds::dispose);
    utils.copy = Some(worlds::copy);
    utils.fill = Some(crate::pixels::fill);
    utils.iterate = Some(crate::pixels::iterate);
    utils.get_platform_data = Some(platform_data);
    utils.get_callback_addr = Some(callback_addr);
    input.utils = &mut utils;
    let mut effect_token = 0u64;
    input.effect_ref = (&mut effect_token as *mut u64).cast();
    input.version.major = ae::PF_PLUG_IN_VERSION as i16;
    input.version.minor = ae::PF_PLUG_IN_SUBVERS as i16;
    // API profile selects the documented host ABI family. This worker remains an experimental
    // AfterCraft helper and never changes vendor licensing or loads Adobe implementation DLLs.
    let api_id = if matches!(host_api, frame::HostApi::AfterEffects) { *b"FXTC" } else { *b"EfCr" };
    input.appl_id = i32::from_be_bytes(api_id);
    input.quality = ae::PF_Quality_HI;
    input.pica_basicP = &mut basic;
    input.inter.abort = Some(abort);
    input.inter.progress = Some(progress);
    input.inter.add_param = Some(add_param);
    input.inter.register_ui = Some(register_ui);
    input.inter.checkout_param = Some(checkout);
    input.inter.checkin_param = Some(checkin);
    input.width = 64;
    input.height = 64;
    input.time_scale = 25;
    input.time_step = 1;
    input.local_time_step = 1;
    input.total_time = 25;
    input.downsample_x = ae::PF_RationalScale { num: 1, den: 1 };
    input.downsample_y = ae::PF_RationalScale { num: 1, den: 1 };
    input.pixel_aspect_ratio = ae::PF_RationalScale { num: 1, den: 1 };
    if let Some(job) = render_job {
        let clock = job.clock()?;
        input.time_scale = clock.scale;
        input.time_step = clock.step;
        input.local_time_step = clock.step;
        utility::set_ticks_per_frame(clock.ticks_per_frame);
    }
    event(json!({"stage": "global_setup", "hostApiId": String::from_utf8_lossy(&api_id)}));
    let result = unsafe { entry(ae::PF_Cmd_GLOBAL_SETUP, &mut input, &mut output, ptr::null_mut(), ptr::null_mut(), ptr::null_mut()) };
    let supports_smart = output.out_flags2 & ae::PF_OutFlag2_SUPPORTS_SMART_RENDER != 0;
    let message: Vec<u8> = output.return_msg.iter().take_while(|c| **c != 0).map(|c| *c as u8).collect();
    event(
        json!({"stage": "global_setup_result", "code": result, "message": String::from_utf8_lossy(&message), "outFlags": output.out_flags, "outFlags2": output.out_flags2}),
    );
    let mut render_error = None;
    if result == 0 {
        input.global_data = output.global_data;
        input.num_params = 1;
        event(json!({"stage": "params_setup"}));
        let setup = unsafe { entry(ae::PF_Cmd_PARAMS_SETUP, &mut input, &mut output, ptr::null_mut(), ptr::null_mut(), ptr::null_mut()) };
        event(json!({"stage": "params_setup_result", "code":setup, "count":output.num_params}));
        if setup != 0 {
            render_error = Some(format!("parameter setup failed ({setup})"));
        } else if let Err(error) = render_frame(entry, &mut input, &mut output, render_job, supports_smart) {
            render_error = Some(error);
        }
        event(json!({"stage": "global_setdown"}));
        let cleanup = unsafe { entry(ae::PF_Cmd_GLOBAL_SETDOWN, &mut input, &mut output, ptr::null_mut(), ptr::null_mut(), ptr::null_mut()) };
        event(json!({"stage": "global_setdown_result", "code": cleanup}));
        if cleanup != 0 && render_error.is_none() {
            render_error = Some(format!("global cleanup failed ({cleanup})"));
        }
    }
    if let Some(error) = render_error {
        return Err(error);
    }
    if result != 0 {
        return Err(format!("global setup failed ({result})"));
    }
    event(json!({"stage": "complete", "initializationAttempted": true, "globalSetupSucceeded": result == 0, "commercialCompatibilityVerified": false}));
    Ok(())
}

fn world(pixels: &mut [ae::PF_Pixel], w: i32, h: i32) -> ae::PF_LayerDef {
    let mut world: ae::PF_LayerDef = unsafe { std::mem::zeroed() };
    world.width = w;
    world.height = h;
    world.rowbytes = w * 4;
    world.data = pixels.as_mut_ptr();
    world.world_flags = ae::PF_WorldFlag_WRITEABLE;
    world.extent_hint = ae::PF_Rect { left: 0, top: 0, right: w, bottom: h };
    world.pix_aspect_ratio = ae::PF_RationalScale { num: 1, den: 1 };
    world
}
fn call(
    entry: unsafe extern "C" fn(ae::PF_Cmd, *mut ae::PF_InData, *mut ae::PF_OutData, ae::PF_ParamList, *mut ae::PF_LayerDef, *mut c_void) -> ae::PF_Err,
    cmd: i32,
    name: &str,
    input: &mut ae::PF_InData,
    output: &mut ae::PF_OutData,
    params: &mut [*mut ae::PF_ParamDef],
    world: &mut ae::PF_LayerDef,
) -> i32 {
    output.return_msg.fill(0);
    event(json!({"stage":name}));
    let code = unsafe { entry(cmd, input, output, params.as_mut_ptr(), world, ptr::null_mut()) };
    let message: Vec<u8> = output.return_msg.iter().take_while(|c| **c != 0).map(|c| *c as u8).collect();
    event(
        json!({"stage":format!("{name}_result"),"code":code,"message":String::from_utf8_lossy(&message),"errorMessage":output.out_flags & ae::PF_OutFlag_DISPLAY_ERROR_MESSAGE != 0}),
    );
    code
}
fn render_frame(
    entry: unsafe extern "C" fn(ae::PF_Cmd, *mut ae::PF_InData, *mut ae::PF_OutData, ae::PF_ParamList, *mut ae::PF_LayerDef, *mut c_void) -> ae::PF_Err,
    input: &mut ae::PF_InData,
    output: &mut ae::PF_OutData,
    job: Option<&frame::Job>,
    supports_smart: bool,
) -> Result<(), String> {
    let use_smart = match job.map(|job| &job.render_mode) {
        Some(frame::RenderMode::Auto) => supports_smart,
        None | Some(frame::RenderMode::Legacy) => false,
        Some(frame::RenderMode::Smart) if supports_smart => true,
        Some(frame::RenderMode::Smart) => return Err("Plugin does not advertise SmartFX".into()),
    };
    // Discovery initializes a normal sequence and its arbitrary defaults before teardown.
    // Several SDK plugins require this lifecycle even when no frame is rendered.
    let (w, h, mut source) =
        if let Some(job) = job { frame::load(&job.input)? } else { (64, 64, vec![ae::PF_Pixel { alpha: 0, red: 0, green: 0, blue: 0 }; 64 * 64]) };
    let logical = if let Some(job) = job { job.logical_size(w, h)? } else { [w, h] };
    let _context = crate::aegp_context::Guard::enter(input.effect_ref, logical);
    let downsample = job.map(|job| job.downsample).unwrap_or_default();
    input.width = logical[0] as i32;
    input.height = logical[1] as i32;
    input.downsample_x = ae::PF_RationalScale { num: downsample.num as i32, den: downsample.den };
    input.downsample_y = input.downsample_x;
    event(json!({"stage":"preview_geometry","layerSize":logical,"bufferSize":[w,h],"downsample":{"num":downsample.num,"den":downsample.den}}));
    input.extent_hint = ae::PF_Rect { left: 0, top: 0, right: w as i32, bottom: h as i32 };
    let clock = if let Some(job) = job { job.clock()? } else { crate::timing::clock(Default::default(), 0.0, None)? };
    input.time_scale = clock.scale;
    input.time_step = clock.step;
    input.local_time_step = clock.step;
    input.current_time = clock.current;
    input.total_time = clock.total;
    CURRENT_SCALE.with(|s| *s.borrow_mut() = clock.scale);
    event(json!({"stage":"render_clock","scale":clock.scale,"step":clock.step,"current":clock.current,"ticksPerFrame":clock.ticks_per_frame}));
    CURRENT_TIME.with(|t| *t.borrow_mut() = input.current_time);
    let source_world = world(&mut source, w as i32, h as i32);
    let mut defs = PARAMS.with(|params| params.borrow().clone());
    if usize::try_from(output.num_params).ok() != Some(defs.len() + 1) {
        return Err("plugin parameter count does not match registration".into());
    }
    for def in &mut defs {
        frame::defaults(def, w as i32, h as i32);
        if def.param_type == ae::PF_Param_LAYER {
            unsafe {
                let default = def.u.ld.dephault;
                def.u.ld = if default == ae::PF_LayerDefault_MYSELF { source_world } else { std::mem::zeroed() };
                def.u.ld.dephault = default;
            }
        }
    }
    for (index, value) in job.into_iter().flat_map(|job| &job.parameters) {
        let def = index.checked_sub(1).and_then(|i| defs.get_mut(i)).ok_or("parameter index outside registered range")?;
        frame::override_value(def, value).map_err(|e| format!("parameter {index}: {e}"))?;
    }
    // AE asks the plugin to construct/copy arbitrary values (for example Mocha data);
    // assigning a null default directly is not a valid runtime value for many plugins.
    let mut arbitrary_owned = Vec::new();
    for (index, def) in defs.iter_mut().enumerate() {
        if def.param_type != ae::PF_Param_ARBITRARY_DATA {
            continue;
        }
        let arb = unsafe { def.u.arb_d };
        let mut handle = ptr::null_mut();
        let mut extra: ae::PF_ArbParamsExtra = unsafe { std::mem::zeroed() };
        extra.id = arb.id;
        if arb.dephault.is_null() {
            extra.which_function = ae::PF_Arbitrary_NEW_FUNC;
            extra.u.new_func_params = ae::PF_ArbParamsExtra__bindgen_ty_1__bindgen_ty_1 { refconPV: arb.refconPV, arbPH: &mut handle };
        } else {
            extra.which_function = ae::PF_Arbitrary_COPY_FUNC;
            extra.u.copy_func_params =
                ae::PF_ArbParamsExtra__bindgen_ty_1__bindgen_ty_3 { refconPV: arb.refconPV, src_arbH: arb.dephault, dst_arbPH: &mut handle };
        }
        event(json!({"stage":"arbitrary_init","index":index+1,"id":arb.id}));
        let code =
            unsafe { entry(ae::PF_Cmd_ARBITRARY_CALLBACK, input, output, ptr::null_mut(), ptr::null_mut(), (&mut extra as *mut ae::PF_ArbParamsExtra).cast()) };
        event(json!({"stage":"arbitrary_init_result","index":index+1,"code":code}));
        if code != 0 {
            return Err(format!("arbitrary parameter {} failed ({code})", index + 1));
        }
        def.u.arb_d.value = handle;
        arbitrary_owned.push((arb.id, arb.refconPV, handle));
    }
    let mut first: ae::PF_ParamDef = unsafe { std::mem::zeroed() };
    first.param_type = ae::PF_Param_LAYER;
    first.u.ld = source_world;
    defs.insert(0, first);
    CURRENT_PARAMS.with(|params| *params.borrow_mut() = defs.clone());
    input.num_params = defs.len() as i32;
    let mut pointers: Vec<_> = defs.iter_mut().map(|d| d as *mut _).collect();
    let mut destination = vec![ae::PF_Pixel { alpha: 0, red: 0, green: 0, blue: 0 }; source.len()];
    let mut dest_world = world(&mut destination, w as i32, h as i32);
    output.width = w as i32;
    output.height = h as i32;
    let sequence = call(entry, ae::PF_Cmd_SEQUENCE_SETUP, "sequence_setup", input, output, &mut pointers, &mut dest_world);
    if sequence != 0 {
        return Err(format!("sequence setup failed ({sequence})"));
    }
    input.sequence_data = output.sequence_data;
    let sequence_guard = crate::sequence::Guard::enter(input.effect_ref, input.sequence_data);
    event(json!({"stage":"sequence_data_ready","present":!input.sequence_data.is_null()}));
    let (setup, render, cleanup) = if job.is_none() {
        (0, 0, 0)
    } else if use_smart {
        (0, crate::smart::render(entry, input, output, &mut dest_world), 0)
    } else {
        let setup = call(entry, ae::PF_Cmd_FRAME_SETUP, "frame_setup", input, output, &mut pointers, &mut dest_world);
        input.frame_data = output.frame_data;
        let render = if setup == 0 && output.width == w as i32 && output.height == h as i32 {
            call(entry, ae::PF_Cmd_RENDER, "render", input, output, &mut pointers, &mut dest_world)
        } else {
            ae::PF_Err_BAD_CALLBACK_PARAM
        };
        let cleanup = call(entry, ae::PF_Cmd_FRAME_SETDOWN, "frame_setdown", input, output, &mut pointers, &mut dest_world);
        (setup, render, cleanup)
    };
    input.frame_data = ptr::null_mut();
    drop(sequence_guard);
    let sequence_cleanup = call(entry, ae::PF_Cmd_SEQUENCE_SETDOWN, "sequence_setdown", input, output, &mut pointers, &mut dest_world);
    input.sequence_data = ptr::null_mut();
    CURRENT_PARAMS.with(|params| params.borrow_mut().clear());
    for (id, refcon, handle) in arbitrary_owned {
        let mut extra: ae::PF_ArbParamsExtra = unsafe { std::mem::zeroed() };
        extra.id = id;
        extra.which_function = ae::PF_Arbitrary_DISPOSE_FUNC;
        extra.u.dispose_func_params = ae::PF_ArbParamsExtra__bindgen_ty_1__bindgen_ty_2 { refconPV: refcon, arbH: handle };
        let code =
            unsafe { entry(ae::PF_Cmd_ARBITRARY_CALLBACK, input, output, ptr::null_mut(), ptr::null_mut(), (&mut extra as *mut ae::PF_ArbParamsExtra).cast()) };
        event(json!({"stage":"arbitrary_dispose_result","id":id,"code":code}));
    }
    if setup != 0 || render != 0 || cleanup != 0 || sequence_cleanup != 0 {
        return Err(format!("render failed: frame setup {setup}, render {render}, frame cleanup {cleanup}, sequence cleanup {sequence_cleanup}"));
    }
    if let Some(job) = job {
        frame::save(&job.output, w, h, &destination)?;
        event(json!({"stage":"frame_written","path":job.output,"width":w,"height":h,"bitDepth":8}));
    } else {
        event(json!({"stage":"sequence_probe_complete","renderAttempted":false}));
    }
    Ok(())
}
