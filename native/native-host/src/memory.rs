//! Bounded host-owned allocations. Handles remain stable when their data is resized.
use after_effects_sys as ae;
use std::{collections::BTreeMap, ffi::c_void, ptr, sync::Mutex};
use windows_sys::Win32::System::Memory::{GetProcessHeap, HEAP_ZERO_MEMORY, HeapAlloc, HeapFree, HeapReAlloc};

const MAX_BLOCK: usize = 64 * 1024 * 1024;
const MAX_TOTAL: usize = 256 * 1024 * 1024;
#[derive(Default)]
struct Memory {
    blocks: BTreeMap<usize, usize>,
    handles: BTreeMap<usize, usize>,
    total: usize,
}
static MEMORY: Mutex<Memory> = Mutex::new(Memory { blocks: BTreeMap::new(), handles: BTreeMap::new(), total: 0 });
impl Memory {
    fn alloc(&mut self, size: usize) -> *mut c_void {
        let size = size.max(1);
        if size > MAX_BLOCK || self.total.saturating_add(size) > MAX_TOTAL || self.blocks.len() >= 8192 {
            return ptr::null_mut();
        }
        let p = unsafe { HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, size) };
        if !p.is_null() {
            self.blocks.insert(p as usize, size);
            self.total += size;
        }
        p
    }
    fn free(&mut self, p: *mut c_void) -> bool {
        let Some(size) = self.blocks.remove(&(p as usize)) else {
            return false;
        };
        self.total = self.total.saturating_sub(size);
        unsafe { HeapFree(GetProcessHeap(), 0, p) != 0 }
    }
    fn resize(&mut self, p: *mut c_void, size: usize) -> *mut c_void {
        let Some(old) = self.blocks.get(&(p as usize)).copied() else {
            return ptr::null_mut();
        };
        let size = size.max(1);
        if size > MAX_BLOCK || self.total.saturating_sub(old).saturating_add(size) > MAX_TOTAL {
            return ptr::null_mut();
        }
        let next = unsafe { HeapReAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, p, size) };
        if !next.is_null() {
            self.blocks.remove(&(p as usize));
            self.blocks.insert(next as usize, size);
            self.total = self.total - old + size;
        }
        next
    }
}
pub unsafe extern "C" fn allocate(size: usize, out: *mut *mut c_void) -> ae::SPErr {
    if out.is_null() {
        return -1;
    }
    unsafe {
        *out = ptr::null_mut();
    }
    let Ok(mut memory) = MEMORY.lock() else {
        return -1;
    };
    let p = memory.alloc(size);
    unsafe {
        *out = p;
    }
    if p.is_null() { -1 } else { 0 }
}
pub unsafe extern "C" fn free(p: *mut c_void) -> ae::SPErr {
    let Ok(mut memory) = MEMORY.lock() else {
        return -1;
    };
    // Block APIs must not invalidate handle-owned storage.
    if memory.handles.contains_key(&(p as usize)) || memory.handles.values().any(|v| *v == p as usize) {
        return -1;
    }
    if memory.free(p) { 0 } else { -1 }
}
pub unsafe extern "C" fn reallocate(p: *mut c_void, size: usize, out: *mut *mut c_void) -> ae::SPErr {
    if out.is_null() {
        return -1;
    }
    unsafe {
        *out = ptr::null_mut();
    }
    let Ok(mut memory) = MEMORY.lock() else {
        return -1;
    };
    if memory.handles.contains_key(&(p as usize)) || memory.handles.values().any(|v| *v == p as usize) {
        return -1;
    }
    let next = memory.resize(p, size);
    unsafe {
        *out = next;
    }
    if next.is_null() { -1 } else { 0 }
}
pub unsafe extern "C" fn new_handle(size: u64) -> ae::PF_Handle {
    let Ok(size) = usize::try_from(size) else {
        return ptr::null_mut();
    };
    let Ok(mut memory) = MEMORY.lock() else {
        return ptr::null_mut();
    };
    let data = memory.alloc(size);
    if data.is_null() {
        return ptr::null_mut();
    }
    let handle = memory.alloc(std::mem::size_of::<*mut c_void>()).cast::<*mut c_void>();
    if handle.is_null() {
        memory.free(data);
        return ptr::null_mut();
    }
    unsafe {
        *handle = data;
    }
    memory.handles.insert(handle as usize, data as usize);
    handle
}
pub unsafe extern "C" fn lock_handle(handle: ae::PF_Handle) -> *mut c_void {
    let Ok(memory) = MEMORY.lock() else {
        return ptr::null_mut();
    };
    memory.handles.get(&(handle as usize)).map(|p| *p as *mut c_void).unwrap_or(ptr::null_mut())
}
pub unsafe extern "C" fn unlock_handle(_handle: ae::PF_Handle) { /* Non-moving allocation. */
}
pub unsafe extern "C" fn dispose_handle(handle: ae::PF_Handle) {
    if let Ok(mut memory) = MEMORY.lock()
        && let Some(data) = memory.handles.remove(&(handle as usize))
    {
        memory.free(data as *mut c_void);
        memory.free(handle.cast());
    }
}
pub unsafe extern "C" fn handle_size(handle: ae::PF_Handle) -> u64 {
    let Ok(memory) = MEMORY.lock() else {
        return 0;
    };
    memory.handles.get(&(handle as usize)).and_then(|p| memory.blocks.get(p)).copied().unwrap_or(0) as u64
}
pub unsafe extern "C" fn resize_handle(size: u64, handle: *mut ae::PF_Handle) -> ae::PF_Err {
    if handle.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let Ok(size) = usize::try_from(size) else {
        return ae::PF_Err_OUT_OF_MEMORY;
    };
    let h = unsafe { *handle };
    let Ok(mut memory) = MEMORY.lock() else {
        return ae::PF_Err_OUT_OF_MEMORY;
    };
    let Some(data) = memory.handles.get(&(h as usize)).copied() else {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    };
    let next = memory.resize(data as *mut c_void, size);
    if next.is_null() {
        return ae::PF_Err_OUT_OF_MEMORY;
    }
    memory.handles.insert(h as usize, next as usize);
    unsafe {
        *h = next;
    }
    0
}
pub static HANDLE_SUITE: ae::PF_HandleSuite1 = ae::PF_HandleSuite1 {
    host_new_handle: Some(new_handle),
    host_lock_handle: Some(lock_handle),
    host_unlock_handle: Some(unlock_handle),
    host_dispose_handle: Some(dispose_handle),
    host_get_handle_size: Some(handle_size),
    host_resize_handle: Some(resize_handle),
};
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handle_resizing_preserves_data_and_identity() {
        unsafe {
            let mut h = new_handle(8);
            assert!(!h.is_null());
            let original = h;
            *lock_handle(h).cast::<u64>() = 123456;
            assert_eq!(resize_handle(1024, &mut h), 0);
            assert_eq!(h, original);
            assert_eq!(*lock_handle(h).cast::<u64>(), 123456);
            assert_eq!(handle_size(h), 1024);
            dispose_handle(h);
            assert!(lock_handle(h).is_null());
            dispose_handle(h);
        }
    }
    #[test]
    fn oversized_allocation_fails_without_replacing_handle() {
        unsafe {
            assert!(new_handle(u64::MAX).is_null());
            let mut h = new_handle(8);
            assert!(!h.is_null());
            assert_ne!(resize_handle(u64::MAX, &mut h), 0);
            assert_eq!(handle_size(h), 8);
            dispose_handle(h);
        }
    }
    #[test]
    fn blocks_reject_foreign_pointers() {
        unsafe {
            assert_ne!(free(std::ptr::dangling_mut::<c_void>()), 0);
            let mut block = ptr::null_mut();
            assert_eq!(allocate(12, &mut block), 0);
            assert_eq!(free(block), 0);
            assert_ne!(free(block), 0);
        }
    }
}
