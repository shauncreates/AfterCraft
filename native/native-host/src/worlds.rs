//! SDK world allocations and pixel-format queries; bounds are independent of plugin inputs.
use crate::memory;
use after_effects_sys as ae;
use std::{collections::BTreeMap, ptr, sync::Mutex};
static FORMATS: Mutex<BTreeMap<usize, i32>> = Mutex::new(BTreeMap::new());
pub unsafe extern "C" fn new_world(_effect: ae::PF_ProgPtr, w: i32, h: i32, _clear: ae::PF_Boolean, format: i32, out: *mut ae::PF_EffectWorld) -> i32 {
    if out.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let bpp = match format {
        ae::PF_PixelFormat_ARGB32 => 4,
        ae::PF_PixelFormat_ARGB64 => 8,
        ae::PF_PixelFormat_ARGB128 => 16,
        _ => return ae::PF_Err_BAD_CALLBACK_PARAM,
    };
    if w <= 0 || h <= 0 || w > 16384 || h > 16384 {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let Some(rowbytes) = w.checked_mul(bpp).and_then(|n| n.checked_add(15)).map(|n| n / 16 * 16) else {
        return ae::PF_Err_OUT_OF_MEMORY;
    };
    let Some(size) = (rowbytes as usize).checked_mul(h as usize) else {
        return ae::PF_Err_OUT_OF_MEMORY;
    };
    let mut data = ptr::null_mut();
    if unsafe { memory::allocate(size, &mut data) } != 0 {
        return ae::PF_Err_OUT_OF_MEMORY;
    }
    let Ok(mut formats) = FORMATS.lock() else {
        unsafe {
            memory::free(data);
        }
        return ae::PF_Err_OUT_OF_MEMORY;
    };
    formats.insert(data as usize, format);
    let mut world: ae::PF_EffectWorld = unsafe { std::mem::zeroed() };
    world.width = w;
    world.height = h;
    world.rowbytes = rowbytes;
    world.data = data.cast();
    world.world_flags = ae::PF_WorldFlag_WRITEABLE | if format == ae::PF_PixelFormat_ARGB64 { ae::PF_WorldFlag_DEEP } else { 0 };
    world.extent_hint = ae::PF_Rect { left: 0, top: 0, right: w, bottom: h };
    world.pix_aspect_ratio = ae::PF_RationalScale { num: 1, den: 1 };
    unsafe {
        *out = world;
    }
    0
}
pub unsafe extern "C" fn legacy_new(effect: ae::PF_ProgPtr, w: i32, h: i32, flags: i32, out: *mut ae::PF_EffectWorld) -> i32 {
    let format = if flags & ae::PF_NewWorldFlag_DEEP_PIXELS != 0 { ae::PF_PixelFormat_ARGB64 } else { ae::PF_PixelFormat_ARGB32 };
    unsafe { new_world(effect, w, h, 1, format, out) }
}
pub unsafe extern "C" fn dispose(_effect: ae::PF_ProgPtr, world: *mut ae::PF_EffectWorld) -> i32 {
    if world.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let data = unsafe { (*world).data };
    let Ok(mut formats) = FORMATS.lock() else {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    };
    if formats.remove(&(data as usize)).is_none() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    if unsafe { memory::free(data.cast()) } != 0 {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    unsafe {
        *world = std::mem::zeroed();
    }
    0
}
pub unsafe extern "C" fn pixel_format(world: *const ae::PF_EffectWorld, out: *mut i32) -> i32 {
    if world.is_null() || out.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let world = unsafe { &*world };
    let Ok(formats) = FORMATS.lock() else {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    };
    let format = formats.get(&(world.data as usize)).copied().unwrap_or(if world.world_flags & ae::PF_WorldFlag_DEEP != 0 {
        ae::PF_PixelFormat_ARGB64
    } else {
        ae::PF_PixelFormat_ARGB32
    });
    unsafe {
        *out = format;
    }
    0
}
pub unsafe extern "C" fn copy(
    _effect: ae::PF_ProgPtr,
    source: *mut ae::PF_EffectWorld,
    dest: *mut ae::PF_EffectWorld,
    sr: *mut ae::PF_Rect,
    dr: *mut ae::PF_Rect,
) -> i32 {
    if source.is_null() || dest.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let (source, dest) = unsafe { (&*source, &mut *dest) };
    let mut sf = 0;
    let mut df = 0;
    unsafe {
        pixel_format(source, &mut sf);
        pixel_format(dest, &mut df);
    }
    if sf != df || source.data.is_null() || dest.data.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let full = |w: &ae::PF_EffectWorld| ae::PF_Rect { left: 0, top: 0, right: w.width, bottom: w.height };
    let sr = if sr.is_null() { full(source) } else { unsafe { *sr } };
    let dr = if dr.is_null() { full(dest) } else { unsafe { *dr } };
    let valid = |r: &ae::PF_Rect, w: &ae::PF_EffectWorld| {
        r.left >= 0 && r.top >= 0 && r.right >= r.left && r.bottom >= r.top && r.right <= w.width && r.bottom <= w.height && w.rowbytes > 0
    };
    if !valid(&sr, source) || !valid(&dr, dest) || sr.right - sr.left != dr.right - dr.left || sr.bottom - sr.top != dr.bottom - dr.top {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let bpp = match sf {
        ae::PF_PixelFormat_ARGB32 => 4,
        ae::PF_PixelFormat_ARGB64 => 8,
        ae::PF_PixelFormat_ARGB128 => 16,
        _ => return ae::PF_Err_BAD_CALLBACK_PARAM,
    };
    if source.width.checked_mul(bpp).is_none_or(|n| n > source.rowbytes) || dest.width.checked_mul(bpp).is_none_or(|n| n > dest.rowbytes) {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    for row in 0..(sr.bottom - sr.top) {
        let si = (sr.top + row) as usize * source.rowbytes as usize + sr.left as usize * bpp as usize;
        let di = (dr.top + row) as usize * dest.rowbytes as usize + dr.left as usize * bpp as usize;
        unsafe {
            ptr::copy(source.data.cast::<u8>().add(si), dest.data.cast::<u8>().add(di), (sr.right - sr.left) as usize * bpp as usize);
        }
    }
    0
}
pub static SUITE2: ae::PF_WorldSuite2 =
    ae::PF_WorldSuite2 { PF_NewWorld: Some(new_world), PF_DisposeWorld: Some(dispose), PF_GetPixelFormat: Some(pixel_format) };
pub static SUITE1: ae::PF_WorldSuite1 = ae::PF_WorldSuite1 { new_world: Some(legacy_new), dispose_world: Some(dispose) };

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn allocation_tracks_format_and_releases() {
        unsafe {
            for format in [ae::PF_PixelFormat_ARGB32, ae::PF_PixelFormat_ARGB64, ae::PF_PixelFormat_ARGB128] {
                let mut world = std::mem::zeroed();
                assert_eq!(new_world(ptr::null_mut(), 7, 8, 1, format, &mut world), 0);
                let mut observed = 0;
                assert_eq!(pixel_format(&world, &mut observed), 0);
                assert_eq!(observed, format);
                assert_eq!(dispose(ptr::null_mut(), &mut world), 0);
                assert!(world.data.is_null());
            }
        }
    }
}
