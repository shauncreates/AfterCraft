//! AfterCraft-owned AEGP Persistent Data Suite 3, from the public SDK contract.
//!
//! Only this application's store is read. Missing keys receive the caller's
//! documented default; no Adobe preference or vendor activation data is imported.
//! Binary keys/strings retain all eight bits. File locks, bounds and atomic
//! replacement keep concurrent helper processes from losing each other's writes.
use after_effects_sys as ae;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    ffi::{c_char, c_void},
    fs::{File, OpenOptions, TryLockError},
    io::{Read, Write},
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    ptr,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, ae::A_Err>;
const PARAM: ae::A_Err = ae::A_Err_PARAMETER;
const IO: ae::A_Err = ae::A_Err_GENERIC;
const ALLOC: ae::A_Err = ae::A_Err_ALLOC;
const MAX_KEY: usize = 1024;
const MAX_VALUE: usize = 1024 * 1024;
const MAX_FILE: usize = 16 * 1024 * 1024;
const MAX_KEYS: usize = 4096;
static BLOB: OnceLock<usize> = OnceLock::new();
// The heap reserves at least one byte even for zero-length handles. Keep the
// requested AEGP length, distinct from that physical allocation size.
static LOGICAL_SIZES: Mutex<BTreeMap<usize, usize>> = Mutex::new(BTreeMap::new());
static TEMP_SERIAL: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
enum Stored {
    Data(Vec<u8>),
    Text(Vec<u8>),
    Long(i32),
    Float(f64),
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Store {
    version: u32,
    sections: BTreeMap<String, BTreeMap<String, Stored>>,
}

impl Default for Store {
    fn default() -> Self {
        Self { version: 1, sections: BTreeMap::new() }
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len().saturating_mul(2));
    for &b in bytes {
        s.push(DIGITS[usize::from(b >> 4)] as char);
        s.push(DIGITS[usize::from(b & 15)] as char);
    }
    s
}

fn unhex(text: &str) -> Result<Vec<u8>> {
    if text.len() > MAX_KEY * 2 || !text.len().is_multiple_of(2) {
        return Err(PARAM);
    }
    let (pairs, _) = text.as_bytes().as_chunks::<2>();
    pairs
        .iter()
        .map(|b| {
            let a = char::from(b[0]).to_digit(16).ok_or(PARAM)?;
            let c = char::from(b[1]).to_digit(16).ok_or(PARAM)?;
            let byte = (a * 16 + c) as u8;
            if byte == 0 { Err(PARAM) } else { Ok(byte) }
        })
        .collect()
}

fn valid(store: &Store) -> Result<()> {
    if store.version != 1 || store.sections.len() > MAX_KEYS {
        return Err(PARAM);
    }
    let mut count = 0usize;
    for (section, entries) in &store.sections {
        unhex(section)?;
        count = count.saturating_add(entries.len());
        if count > MAX_KEYS {
            return Err(ALLOC);
        }
        for (key, value) in entries {
            unhex(key)?;
            match value {
                Stored::Data(bytes) if bytes.len() <= MAX_VALUE => {}
                Stored::Text(bytes) if bytes.len() <= MAX_VALUE && !bytes.contains(&0) => {}
                Stored::Long(_) => {}
                Stored::Float(v) if v.is_finite() => {}
                _ => return Err(PARAM),
            }
        }
    }
    Ok(())
}

fn directory() -> Result<PathBuf> {
    #[cfg(test)]
    {
        static ROOT: OnceLock<PathBuf> = OnceLock::new();
        Ok(ROOT.get_or_init(|| std::env::temp_dir().join(format!("aftercraft-persistent-ffi-{}", std::process::id()))).clone())
    }
    #[cfg(not(test))]
    {
        let root = match std::env::var_os("AFTERCRAFT_CONFIG_DIR") {
            Some(root) => PathBuf::from(root),
            None => PathBuf::from(std::env::var_os("APPDATA").ok_or(IO)?).join("AfterCraft"),
        };
        if !root.is_absolute() {
            return Err(PARAM);
        }
        Ok(root.join("NativePreferences"))
    }
}

fn transact_at<T>(directory: &Path, operation: impl FnOnce(&mut Store) -> Result<(T, bool)>) -> Result<T> {
    std::fs::create_dir_all(directory).map_err(|_| IO)?;
    let lock = OpenOptions::new().create(true).truncate(false).read(true).write(true).open(directory.join("aegp-prefs.lock")).map_err(|_| IO)?;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match lock.try_lock() {
            Ok(()) => break,
            Err(TryLockError::WouldBlock) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            Err(_) => return Err(IO),
        }
    }
    let path = directory.join("aegp-prefs-v1.json");
    let mut store = match File::open(&path) {
        Ok(file) => {
            let mut bytes = Vec::new();
            file.take((MAX_FILE + 1) as u64).read_to_end(&mut bytes).map_err(|_| IO)?;
            if bytes.len() > MAX_FILE {
                return Err(ALLOC);
            }
            serde_json::from_slice::<Store>(&bytes).map_err(|_| PARAM)?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Store::default(),
        Err(_) => return Err(IO),
    };
    valid(&store)?;
    let (result, dirty) = operation(&mut store)?;
    if dirty {
        valid(&store)?;
        let bytes = serde_json::to_vec(&store).map_err(|_| IO)?;
        if bytes.len() > MAX_FILE {
            return Err(ALLOC);
        }
        let temp = directory.join(format!("aegp-prefs-{}-{}.tmp", std::process::id(), TEMP_SERIAL.fetch_add(1, Ordering::Relaxed)));
        let mut output = OpenOptions::new().write(true).create_new(true).open(&temp).map_err(|_| IO)?;
        let saved = (|| {
            output.write_all(&bytes).map_err(|_| IO)?;
            output.sync_all().map_err(|_| IO)?;
            drop(output);
            std::fs::rename(&temp, &path).map_err(|_| IO)
        })();
        if saved.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        saved?;
    }
    // Closing the lock releases the OS lock even if the helper is terminated.
    Ok(result)
}

fn transact<T>(operation: impl FnOnce(&mut Store) -> Result<(T, bool)>) -> Result<T> {
    transact_at(&directory()?, operation)
}

fn blob_valid(blob: ae::AEGP_PersistentBlobH) -> Result<()> {
    if !blob.is_null() && BLOB.get().is_some_and(|b| *b == blob as usize) { Ok(()) } else { Err(PARAM) }
}

unsafe fn c_bytes(value: *const c_char, limit: usize) -> Result<Vec<u8>> {
    if value.is_null() {
        return Err(PARAM);
    }
    let mut bytes = Vec::new();
    for i in 0..=limit {
        let byte = unsafe { *value.add(i) } as u8;
        if byte == 0 {
            return Ok(bytes);
        }
        if i == limit {
            return Err(ALLOC);
        }
        bytes.push(byte);
    }
    Err(PARAM)
}

unsafe fn keys(blob: ae::AEGP_PersistentBlobH, section: *const c_char, key: *const c_char) -> Result<(String, String)> {
    blob_valid(blob)?;
    Ok((hex(&unsafe { c_bytes(section, MAX_KEY) }?), hex(&unsafe { c_bytes(key, MAX_KEY) }?)))
}

fn obtain(section: String, key: String, default: Stored) -> Result<Stored> {
    transact(|store| {
        if let Some(value) = store.sections.get(&section).and_then(|s| s.get(&key)) {
            return Ok((value.clone(), false));
        }
        store.sections.entry(section).or_default().insert(key, default.clone());
        Ok((default, true))
    })
}

fn put(section: String, key: String, value: Stored) -> Result<()> {
    transact(|store| {
        store.sections.entry(section).or_default().insert(key, value);
        Ok(((), true))
    })
}

fn finish(callback: &str, result: Result<()>) -> ae::A_Err {
    let code = result.err().unwrap_or(0);
    crate::event(serde_json::json!({"stage":"persistent_callback","name":callback,"code":code}));
    code
}

unsafe fn write_string(value: &[u8], capacity: usize, output: *mut c_char) -> Result<()> {
    if output.is_null() || capacity == 0 || capacity > MAX_VALUE + 1 {
        return Err(PARAM);
    }
    let keep = value.len().min(capacity - 1);
    unsafe {
        ptr::copy_nonoverlapping(value.as_ptr(), output.cast::<u8>(), keep);
        *output.add(keep) = 0;
    }
    if capacity <= value.len() { Err(PARAM) } else { Ok(()) }
}

unsafe fn handle_bytes(handle: ae::AEGP_MemHandle) -> Result<Vec<u8>> {
    let pointer = unsafe { crate::memory::lock_handle(handle.cast()) };
    if pointer.is_null() {
        return Err(PARAM);
    }
    let physical = unsafe { crate::memory::handle_size(handle.cast()) } as usize;
    let size = LOGICAL_SIZES.lock().map_err(|_| IO)?.get(&(handle as usize)).copied().unwrap_or(physical);
    if size > physical {
        return Err(PARAM);
    }
    if size > MAX_VALUE {
        return Err(ALLOC);
    }
    Ok(unsafe { std::slice::from_raw_parts(pointer.cast::<u8>(), size) }.to_vec())
}

fn owned_handle(bytes: &[u8]) -> Result<ae::AEGP_MemHandle> {
    if bytes.len() > MAX_VALUE {
        return Err(ALLOC);
    }
    let handle = unsafe { crate::memory::new_handle(bytes.len() as u64) };
    if handle.is_null() {
        return Err(ALLOC);
    }
    let pointer = unsafe { crate::memory::lock_handle(handle) };
    if pointer.is_null() {
        unsafe {
            crate::memory::dispose_handle(handle);
        }
        return Err(ALLOC);
    }
    let Ok(mut sizes) = LOGICAL_SIZES.lock() else {
        unsafe {
            crate::memory::dispose_handle(handle);
        }
        return Err(IO);
    };
    sizes.insert(handle as usize, bytes.len());
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.cast::<u8>(), bytes.len());
    }
    Ok(handle.cast())
}

unsafe extern "C" fn application_blob(out: *mut ae::AEGP_PersistentBlobH) -> ae::A_Err {
    finish(
        "AEGP_GetApplicationBlob",
        (|| {
            if out.is_null() {
                return Err(PARAM);
            }
            unsafe {
                *out = ptr::null_mut();
            }
            let handle = *BLOB.get_or_init(|| unsafe { crate::memory::new_handle(1) } as usize);
            if handle == 0 {
                return Err(ALLOC);
            }
            unsafe {
                *out = handle as ae::AEGP_PersistentBlobH;
            }
            Ok(())
        })(),
    )
}

unsafe extern "C" fn num_sections(blob: ae::AEGP_PersistentBlobH, out: *mut ae::A_long) -> ae::A_Err {
    finish(
        "AEGP_GetNumSections",
        (|| {
            blob_valid(blob)?;
            if out.is_null() {
                return Err(PARAM);
            }
            let count = transact(|s| Ok((s.sections.len(), false)))?;
            unsafe {
                *out = count as ae::A_long;
            }
            Ok(())
        })(),
    )
}

unsafe extern "C" fn section_by_index(blob: ae::AEGP_PersistentBlobH, index: ae::A_long, capacity: ae::A_long, out: *mut c_char) -> ae::A_Err {
    finish(
        "AEGP_GetSectionKeyByIndex",
        (|| {
            blob_valid(blob)?;
            let index = usize::try_from(index).map_err(|_| PARAM)?;
            let capacity = usize::try_from(capacity).map_err(|_| PARAM)?;
            let value = transact(|s| Ok((s.sections.keys().nth(index).cloned().ok_or(PARAM)?, false)))?;
            unsafe { write_string(&unhex(&value)?, capacity, out) }
        })(),
    )
}

unsafe extern "C" fn exists(blob: ae::AEGP_PersistentBlobH, section: *const c_char, key: *const c_char, out: *mut ae::A_Boolean) -> ae::A_Err {
    finish(
        "AEGP_DoesKeyExist",
        (|| {
            if out.is_null() {
                return Err(PARAM);
            }
            let (section, key) = unsafe { keys(blob, section, key) }?;
            let found = transact(|s| Ok((s.sections.get(&section).is_some_and(|s| s.contains_key(&key)), false)))?;
            unsafe {
                *out = u8::from(found);
            }
            Ok(())
        })(),
    )
}

unsafe extern "C" fn num_keys(blob: ae::AEGP_PersistentBlobH, section: *const c_char, out: *mut ae::A_long) -> ae::A_Err {
    finish(
        "AEGP_GetNumKeys",
        (|| {
            blob_valid(blob)?;
            if out.is_null() {
                return Err(PARAM);
            }
            let section = hex(&unsafe { c_bytes(section, MAX_KEY) }?);
            let count = transact(|s| Ok((s.sections.get(&section).map_or(0, BTreeMap::len), false)))?;
            unsafe {
                *out = count as ae::A_long;
            }
            Ok(())
        })(),
    )
}

unsafe extern "C" fn key_by_index(
    blob: ae::AEGP_PersistentBlobH,
    section: *const c_char,
    index: ae::A_long,
    capacity: ae::A_long,
    out: *mut c_char,
) -> ae::A_Err {
    finish(
        "AEGP_GetValueKeyByIndex",
        (|| {
            blob_valid(blob)?;
            let section = hex(&unsafe { c_bytes(section, MAX_KEY) }?);
            let index = usize::try_from(index).map_err(|_| PARAM)?;
            let capacity = usize::try_from(capacity).map_err(|_| PARAM)?;
            let value = transact(|s| Ok((s.sections.get(&section).and_then(|s| s.keys().nth(index)).cloned().ok_or(PARAM)?, false)))?;
            unsafe { write_string(&unhex(&value)?, capacity, out) }
        })(),
    )
}

unsafe extern "C" fn get_data_handle(
    _plugin: ae::AEGP_PluginID,
    blob: ae::AEGP_PersistentBlobH,
    section: *const c_char,
    key: *const c_char,
    default: ae::AEGP_MemHandle,
    out: *mut ae::AEGP_MemHandle,
) -> ae::A_Err {
    finish(
        "AEGP_GetDataHandle",
        (|| {
            if out.is_null() {
                return Err(PARAM);
            }
            unsafe {
                *out = ptr::null_mut();
            }
            let (section, key) = unsafe { keys(blob, section, key) }?;
            let default = if default.is_null() { Vec::new() } else { unsafe { handle_bytes(default) }? };
            let Stored::Data(value) = obtain(section, key, Stored::Data(default))? else {
                return Err(PARAM);
            };
            unsafe {
                *out = owned_handle(&value)?;
            }
            Ok(())
        })(),
    )
}

unsafe extern "C" fn get_data(
    blob: ae::AEGP_PersistentBlobH,
    section: *const c_char,
    key: *const c_char,
    size: ae::A_u_long,
    default: *const c_void,
    out: *mut c_void,
) -> ae::A_Err {
    finish(
        "AEGP_GetData",
        (|| {
            let size = size as usize;
            if size > MAX_VALUE || (size != 0 && out.is_null()) {
                return Err(PARAM);
            }
            let (section, key) = unsafe { keys(blob, section, key) }?;
            let default = if default.is_null() { vec![0; size] } else { unsafe { std::slice::from_raw_parts(default.cast::<u8>(), size) }.to_vec() };
            let Stored::Data(value) = obtain(section, key, Stored::Data(default))? else {
                return Err(PARAM);
            };
            if value.len() != size {
                return Err(PARAM);
            }
            if size != 0 {
                unsafe {
                    ptr::copy_nonoverlapping(value.as_ptr(), out.cast::<u8>(), size);
                }
            }
            Ok(())
        })(),
    )
}

unsafe extern "C" fn get_string(
    blob: ae::AEGP_PersistentBlobH,
    section: *const c_char,
    key: *const c_char,
    default: *const c_char,
    capacity: ae::A_u_long,
    out: *mut c_char,
    actual: *mut ae::A_u_long,
) -> ae::A_Err {
    finish(
        "AEGP_GetString",
        (|| {
            let (section, key) = unsafe { keys(blob, section, key) }?;
            let default = if default.is_null() { Vec::new() } else { unsafe { c_bytes(default, MAX_VALUE) }? };
            let Stored::Text(value) = obtain(section, key, Stored::Text(default))? else {
                return Err(PARAM);
            };
            if !actual.is_null() {
                unsafe {
                    *actual = (value.len() + 1) as ae::A_u_long;
                }
            }
            if capacity == 0 && out.is_null() && !actual.is_null() {
                return Ok(());
            }
            unsafe { write_string(&value, capacity as usize, out) }
        })(),
    )
}

unsafe extern "C" fn get_long(
    blob: ae::AEGP_PersistentBlobH,
    section: *const c_char,
    key: *const c_char,
    default: ae::A_long,
    out: *mut ae::A_long,
) -> ae::A_Err {
    finish(
        "AEGP_GetLong",
        (|| {
            if out.is_null() {
                return Err(PARAM);
            }
            let (section, key) = unsafe { keys(blob, section, key) }?;
            let Stored::Long(value) = obtain(section, key, Stored::Long(default))? else {
                return Err(PARAM);
            };
            unsafe {
                *out = value;
            }
            Ok(())
        })(),
    )
}

fn six_decimals(value: f64) -> Result<f64> {
    if !value.is_finite() {
        return Err(PARAM);
    }
    Ok(if value.abs() <= f64::MAX / 1_000_000.0 { (value * 1_000_000.0).round() / 1_000_000.0 } else { value })
}

unsafe extern "C" fn get_float(
    blob: ae::AEGP_PersistentBlobH,
    section: *const c_char,
    key: *const c_char,
    default: ae::A_FpLong,
    out: *mut ae::A_FpLong,
) -> ae::A_Err {
    finish(
        "AEGP_GetFpLong",
        (|| {
            if out.is_null() {
                return Err(PARAM);
            }
            let (section, key) = unsafe { keys(blob, section, key) }?;
            let Stored::Float(value) = obtain(section, key, Stored::Float(six_decimals(default)?))? else {
                return Err(PARAM);
            };
            unsafe {
                *out = value;
            }
            Ok(())
        })(),
    )
}

unsafe extern "C" fn set_data_handle(blob: ae::AEGP_PersistentBlobH, section: *const c_char, key: *const c_char, handle: ae::AEGP_MemHandle) -> ae::A_Err {
    finish(
        "AEGP_SetDataHandle",
        (|| {
            let (section, key) = unsafe { keys(blob, section, key) }?;
            put(section, key, Stored::Data(unsafe { handle_bytes(handle) }?))
        })(),
    )
}

unsafe extern "C" fn set_data(
    blob: ae::AEGP_PersistentBlobH,
    section: *const c_char,
    key: *const c_char,
    size: ae::A_u_long,
    value: *const c_void,
) -> ae::A_Err {
    finish(
        "AEGP_SetData",
        (|| {
            if size as usize > MAX_VALUE || (size != 0 && value.is_null()) {
                return Err(PARAM);
            }
            let (section, key) = unsafe { keys(blob, section, key) }?;
            let value = if size == 0 { Vec::new() } else { unsafe { std::slice::from_raw_parts(value.cast::<u8>(), size as usize) }.to_vec() };
            put(section, key, Stored::Data(value))
        })(),
    )
}

unsafe extern "C" fn set_string(blob: ae::AEGP_PersistentBlobH, section: *const c_char, key: *const c_char, value: *const c_char) -> ae::A_Err {
    finish(
        "AEGP_SetString",
        (|| {
            let (section, key) = unsafe { keys(blob, section, key) }?;
            put(section, key, Stored::Text(unsafe { c_bytes(value, MAX_VALUE) }?))
        })(),
    )
}

unsafe extern "C" fn set_long(blob: ae::AEGP_PersistentBlobH, section: *const c_char, key: *const c_char, value: ae::A_long) -> ae::A_Err {
    finish(
        "AEGP_SetLong",
        (|| {
            let (section, key) = unsafe { keys(blob, section, key) }?;
            put(section, key, Stored::Long(value))
        })(),
    )
}

unsafe extern "C" fn set_float(blob: ae::AEGP_PersistentBlobH, section: *const c_char, key: *const c_char, value: ae::A_FpLong) -> ae::A_Err {
    finish(
        "AEGP_SetFpLong",
        (|| {
            let (section, key) = unsafe { keys(blob, section, key) }?;
            put(section, key, Stored::Float(six_decimals(value)?))
        })(),
    )
}

unsafe extern "C" fn delete(blob: ae::AEGP_PersistentBlobH, section: *const c_char, key: *const c_char) -> ae::A_Err {
    finish(
        "AEGP_DeleteEntry",
        (|| {
            let (section, key) = unsafe { keys(blob, section, key) }?;
            transact(|s| {
                let removed = s.sections.get_mut(&section).is_some_and(|s| s.remove(&key).is_some());
                if s.sections.get(&section).is_some_and(BTreeMap::is_empty) {
                    s.sections.remove(&section);
                }
                Ok(((), removed))
            })
        })(),
    )
}

unsafe extern "C" fn prefs_directory(out: *mut ae::AEGP_MemHandle) -> ae::A_Err {
    finish(
        "AEGP_GetPrefsDirectory",
        (|| {
            if out.is_null() {
                return Err(PARAM);
            }
            unsafe {
                *out = ptr::null_mut();
            }
            let directory = directory()?;
            std::fs::create_dir_all(&directory).map_err(|_| IO)?;
            let utf16: Vec<u16> = directory.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
            if utf16.len() > MAX_VALUE / 2 {
                return Err(ALLOC);
            }
            let bytes = unsafe { std::slice::from_raw_parts(utf16.as_ptr().cast::<u8>(), utf16.len() * 2) };
            unsafe {
                *out = owned_handle(bytes)?;
            }
            Ok(())
        })(),
    )
}

pub static SUITE3: ae::AEGP_PersistentDataSuite3 = ae::AEGP_PersistentDataSuite3 {
    AEGP_GetApplicationBlob: Some(application_blob),
    AEGP_GetNumSections: Some(num_sections),
    AEGP_GetSectionKeyByIndex: Some(section_by_index),
    AEGP_DoesKeyExist: Some(exists),
    AEGP_GetNumKeys: Some(num_keys),
    AEGP_GetValueKeyByIndex: Some(key_by_index),
    AEGP_GetDataHandle: Some(get_data_handle),
    AEGP_GetData: Some(get_data),
    AEGP_GetString: Some(get_string),
    AEGP_GetLong: Some(get_long),
    AEGP_GetFpLong: Some(get_float),
    AEGP_SetDataHandle: Some(set_data_handle),
    AEGP_SetData: Some(set_data),
    AEGP_SetString: Some(set_string),
    AEGP_SetLong: Some(set_long),
    AEGP_SetFpLong: Some(set_float),
    AEGP_DeleteEntry: Some(delete),
    AEGP_GetPrefsDirectory: Some(prefs_directory),
};

unsafe extern "C" fn mem_new(
    _plugin: ae::AEGP_PluginID,
    _name: *const c_char,
    size: ae::AEGP_MemSize,
    flags: ae::AEGP_MemFlag,
    out: *mut ae::AEGP_MemHandle,
) -> ae::A_Err {
    if out.is_null() || flags & !3 != 0 {
        return PARAM;
    }
    unsafe {
        *out = ptr::null_mut();
    }
    let handle = unsafe { crate::memory::new_handle(u64::from(size)) };
    if handle.is_null() {
        return ALLOC;
    }
    let Ok(mut sizes) = LOGICAL_SIZES.lock() else {
        unsafe {
            crate::memory::dispose_handle(handle);
        }
        return IO;
    };
    sizes.insert(handle as usize, size as usize);
    unsafe {
        *out = handle.cast();
    }
    0
}
unsafe extern "C" fn mem_free(handle: ae::AEGP_MemHandle) -> ae::A_Err {
    if unsafe { crate::memory::lock_handle(handle.cast()) }.is_null() {
        return PARAM;
    }
    let Ok(mut sizes) = LOGICAL_SIZES.lock() else {
        return IO;
    };
    sizes.remove(&(handle as usize));
    unsafe {
        crate::memory::dispose_handle(handle.cast());
    }
    0
}
unsafe extern "C" fn mem_lock(handle: ae::AEGP_MemHandle, out: *mut *mut c_void) -> ae::A_Err {
    if out.is_null() {
        return PARAM;
    }
    let pointer = unsafe { crate::memory::lock_handle(handle.cast()) };
    unsafe {
        *out = pointer;
    }
    if pointer.is_null() { PARAM } else { 0 }
}
unsafe extern "C" fn mem_unlock(handle: ae::AEGP_MemHandle) -> ae::A_Err {
    if unsafe { crate::memory::lock_handle(handle.cast()) }.is_null() { PARAM } else { 0 }
}
unsafe extern "C" fn mem_size(handle: ae::AEGP_MemHandle, out: *mut ae::AEGP_MemSize) -> ae::A_Err {
    if out.is_null() || unsafe { crate::memory::lock_handle(handle.cast()) }.is_null() {
        return PARAM;
    }
    let Ok(sizes) = LOGICAL_SIZES.lock() else {
        return IO;
    };
    let size = sizes.get(&(handle as usize)).copied().unwrap_or_else(|| unsafe { crate::memory::handle_size(handle.cast()) } as usize);
    unsafe {
        *out = size as ae::AEGP_MemSize;
    }
    0
}
unsafe extern "C" fn mem_resize(_name: *const c_char, size: ae::AEGP_MemSize, handle: ae::AEGP_MemHandle) -> ae::A_Err {
    if unsafe { crate::memory::lock_handle(handle.cast()) }.is_null() {
        return PARAM;
    }
    let Ok(mut sizes) = LOGICAL_SIZES.lock() else {
        return IO;
    };
    let mut handle = handle.cast();
    match unsafe { crate::memory::resize_handle(u64::from(size), &mut handle) } {
        0 => {
            sizes.insert(handle as usize, size as usize);
            0
        }
        _ => ALLOC,
    }
}
unsafe extern "C" fn mem_reporting(_enabled: ae::A_Boolean) -> ae::A_Err {
    crate::event(serde_json::json!({"stage":"unsupported_callback","name":"AEGP_SetMemReportingOn"}));
    ae::A_Err_GENERIC
}
unsafe extern "C" fn mem_stats(_plugin: ae::AEGP_PluginID, _count: *mut ae::A_long, _size: *mut ae::A_long) -> ae::A_Err {
    crate::event(serde_json::json!({"stage":"unsupported_callback","name":"AEGP_GetMemStats"}));
    ae::A_Err_GENERIC
}
pub static MEMORY1: ae::AEGP_MemorySuite1 = ae::AEGP_MemorySuite1 {
    AEGP_NewMemHandle: Some(mem_new),
    AEGP_FreeMemHandle: Some(mem_free),
    AEGP_LockMemHandle: Some(mem_lock),
    AEGP_UnlockMemHandle: Some(mem_unlock),
    AEGP_GetMemHandleSize: Some(mem_size),
    AEGP_ResizeMemHandle: Some(mem_resize),
    AEGP_SetMemReportingOn: Some(mem_reporting),
    AEGP_GetMemStats: Some(mem_stats),
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn test_directory() -> PathBuf {
        let path = std::env::temp_dir().join(format!("aftercraft-persistent-test-{}-{}", std::process::id(), TEMP_SERIAL.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn typed_defaults_persist_and_binary_names_round_trip() {
        let directory = test_directory();
        let section = hex(&[b's', 0xff]);
        let key = hex(&[b'k', 0x80]);
        transact_at(&directory, |s| {
            s.sections.entry(section.clone()).or_default().insert(key.clone(), Stored::Text(vec![0xfe, b'x']));
            Ok(((), true))
        })
        .unwrap();
        let loaded = transact_at(&directory, |s| Ok((s.sections.get(&section).and_then(|s| s.get(&key)).cloned(), false))).unwrap();
        assert_eq!(loaded, Some(Stored::Text(vec![0xfe, b'x'])));
        assert_eq!(unhex(&section).unwrap(), vec![b's', 0xff]);
        assert_eq!(six_decimals(0.1234567).unwrap(), 0.123457);
        assert!(six_decimals(f64::NAN).is_err());
        assert!(six_decimals(f64::MAX).unwrap().is_finite());
    }

    #[test]
    fn concurrent_transactions_do_not_overwrite_each_other() {
        let directory = test_directory();
        let threads: Vec<_> = (0..8)
            .map(|i| {
                let directory = directory.clone();
                std::thread::spawn(move || {
                    transact_at(&directory, |s| {
                        s.sections.entry(hex(b"concurrent")).or_default().insert(hex(&[i + 1]), Stored::Long(i32::from(i)));
                        Ok(((), true))
                    })
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap().unwrap();
        }
        let count = transact_at(&directory, |s| Ok((s.sections[&hex(b"concurrent")].len(), false))).unwrap();
        assert_eq!(count, 8);
    }

    #[test]
    fn corrupt_store_is_preserved_and_limits_refuse_writes() {
        let directory = test_directory();
        let path = directory.join("aegp-prefs-v1.json");
        std::fs::write(&path, b"invalid store").unwrap();
        assert!(transact_at(&directory, |_| Ok(((), true))).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"invalid store");
        let mut store = Store::default();
        store.sections.entry(hex(b"s")).or_default().insert(hex(b"k"), Stored::Data(vec![0; MAX_VALUE + 1]));
        assert!(valid(&store).is_err());
        assert!(unhex("00").is_err());
        assert!(unhex("bad").is_err());
    }

    #[test]
    fn ffi_defaults_strings_handles_and_type_errors() {
        let section = CString::new(format!("synthetic-prefs-{}", TEMP_SERIAL.fetch_add(1, Ordering::Relaxed))).unwrap();
        let long_key = c"count";
        let mut blob = ptr::null_mut();
        unsafe {
            assert_eq!(application_blob(&mut blob), 0);
            let mut n = 0;
            assert_eq!(get_long(blob, section.as_ptr(), long_key.as_ptr(), 42, &mut n), 0);
            assert_eq!(n, 42);
            assert_eq!(get_long(blob, section.as_ptr(), long_key.as_ptr(), 99, &mut n), 0);
            assert_eq!(n, 42, "getter defaults must not replace existing values");
            assert_eq!(get_long(ptr::null_mut(), section.as_ptr(), long_key.as_ptr(), 0, &mut n), PARAM);
            assert_eq!(get_long(blob, section.as_ptr(), long_key.as_ptr(), 0, ptr::null_mut()), PARAM);
            let mut actual = 0;
            let mut small = [0i8; 3];
            assert_eq!(get_string(blob, section.as_ptr(), c"text".as_ptr(), c"hello".as_ptr(), 3, small.as_mut_ptr(), &mut actual), PARAM);
            assert_eq!(actual, 6);
            assert_eq!(small, [b'h' as i8, b'e' as i8, 0]);
            assert_eq!(get_string(blob, section.as_ptr(), c"text".as_ptr(), ptr::null(), 0, ptr::null_mut(), &mut actual), 0);
            assert_eq!(actual, 6);
            assert_eq!(get_long(blob, section.as_ptr(), c"text".as_ptr(), 0, &mut n), PARAM);
            let default = owned_handle(b"opaque\0bytes").unwrap();
            let mut got = ptr::null_mut();
            assert_eq!(get_data_handle(0, blob, section.as_ptr(), c"data".as_ptr(), default, &mut got), 0);
            assert_ne!(default, got);
            assert_eq!(handle_bytes(got).unwrap(), b"opaque\0bytes");
            assert_eq!(mem_free(default), 0);
            assert_eq!(mem_free(got), 0);
            let empty = owned_handle(&[]).unwrap();
            let mut size = 99;
            assert_eq!(mem_size(empty, &mut size), 0);
            assert_eq!(size, 0);
            assert!(handle_bytes(empty).unwrap().is_empty());
            assert_eq!(mem_free(empty), 0);
            assert_eq!(delete(blob, section.as_ptr(), long_key.as_ptr()), 0);
            let mut found = 1;
            assert_eq!(exists(blob, section.as_ptr(), long_key.as_ptr(), &mut found), 0);
            assert_eq!(found, 0);
        }
    }
}
