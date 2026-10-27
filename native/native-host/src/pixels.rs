//! Original serial 8-bit utility callbacks, from the published graphics/iteration contracts.
//! Vendor pointers are trusted only inside the disposable worker; declared dimensions are bounded.
use after_effects_sys as ae;
use std::{ffi::c_void, ptr};

unsafe fn world(pointer: *const ae::PF_EffectWorld) -> Result<ae::PF_EffectWorld, i32> {
    if pointer.is_null() {
        return Err(ae::PF_Err_BAD_CALLBACK_PARAM);
    }
    let value = unsafe { *pointer };
    let mut format = 0;
    if unsafe { crate::worlds::pixel_format(pointer, &mut format) } != 0
        || format != ae::PF_PixelFormat_ARGB32
        || value.data.is_null()
        || value.width <= 0
        || value.height <= 0
        || value.width > 16384
        || value.height > 16384
        || value.width.checked_mul(4).is_none_or(|n| n > value.rowbytes)
        || value.rowbytes <= 0
        || i64::from(value.rowbytes) * i64::from(value.height) > 256 * 1024 * 1024
    {
        return Err(ae::PF_Err_BAD_CALLBACK_PARAM);
    }
    Ok(value)
}
fn clip(area: ae::PF_Rect, width: i32, height: i32) -> ae::PF_Rect {
    ae::PF_Rect { left: area.left.clamp(0, width), right: area.right.clamp(0, width), top: area.top.clamp(0, height), bottom: area.bottom.clamp(0, height) }
}
unsafe fn pixel(world: &ae::PF_EffectWorld, x: i32, y: i32) -> *mut ae::PF_Pixel {
    // Callers clip coordinates to a validated positive-stride 8-bit world.
    unsafe { world.data.cast::<u8>().add(y as usize * world.rowbytes as usize + x as usize * 4).cast() }
}
pub unsafe extern "C" fn fill(_effect: ae::PF_ProgPtr, color: *const ae::PF_Pixel, area: *const ae::PF_Rect, dest: *mut ae::PF_EffectWorld) -> i32 {
    let dest = match unsafe { world(dest) } {
        Ok(w) => w,
        Err(e) => return e,
    };
    let color = if color.is_null() { unsafe { std::mem::zeroed() } } else { unsafe { *color } };
    let rect = if area.is_null() { ae::PF_Rect { left: 0, top: 0, right: dest.width, bottom: dest.height } } else { unsafe { *area } };
    let rect = clip(rect, dest.width, dest.height);
    for y in rect.top..rect.bottom {
        for x in rect.left..rect.right {
            unsafe { pixel(&dest, x, y).write(color) };
        }
    }
    0
}
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn iterate(
    input: *mut ae::PF_InData,
    progress_base: i32,
    progress_final: i32,
    source: *mut ae::PF_EffectWorld,
    area: *const ae::PF_Rect,
    refcon: *mut c_void,
    callback: ae::PF_IteratePixel8Func,
    dest: *mut ae::PF_EffectWorld,
) -> i32 {
    if input.is_null() || progress_base < 0 || progress_final < 0 {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let Some(callback) = callback else {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    };
    let dest = match unsafe { world(dest) } {
        Ok(w) => w,
        Err(e) => return e,
    };
    let source = if source.is_null() {
        None
    } else {
        match unsafe { world(source) } {
            Ok(w) => Some(w),
            Err(e) => return e,
        }
    };
    let width = source.as_ref().map_or(dest.width, |s| s.width.min(dest.width));
    let height = source.as_ref().map_or(dest.height, |s| s.height.min(dest.height));
    let rect = if area.is_null() { ae::PF_Rect { left: 0, top: 0, right: width, bottom: height } } else { unsafe { *area } };
    let rect = clip(rect, width, height);
    // Copy POD callback state; hold no Rust reference across a vendor callback or in-place write.
    let input = unsafe { *input };
    for y in rect.top..rect.bottom {
        if let Some(abort) = input.inter.abort {
            let result = unsafe { abort(input.effect_ref) };
            if result != 0 {
                return result;
            }
        }
        for x in rect.left..rect.right {
            let src = source.as_ref().map_or(ptr::null_mut(), |s| unsafe { pixel(s, x, y) });
            let result = unsafe { callback(refcon, x, y, src, pixel(&dest, x, y)) };
            if result != 0 {
                return result;
            }
        }
        if progress_final > 0
            && let Some(progress) = input.inter.progress
        {
            let rows = i64::from((rect.bottom - rect.top).max(1));
            let current = i64::from(progress_base) + i64::from(y - rect.top + 1) * (i64::from(progress_final) - i64::from(progress_base)) / rows;
            let result = unsafe { progress(input.effect_ref, current as i32, progress_final) };
            if result != 0 {
                return result;
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    unsafe fn buffer(data: &mut [u8; 48]) -> ae::PF_EffectWorld {
        let mut w: ae::PF_EffectWorld = unsafe { std::mem::zeroed() };
        w.data = data.as_mut_ptr().cast();
        w.width = 3;
        w.height = 3;
        w.rowbytes = 16;
        w
    }
    unsafe extern "C" fn shade(_: *mut c_void, x: i32, y: i32, src: *mut ae::PF_Pixel, dst: *mut ae::PF_Pixel) -> i32 {
        unsafe {
            dst.write(ae::PF_Pixel { alpha: 255, red: if src.is_null() { 17 } else { (*src).red + 1 }, green: x as u8, blue: y as u8 });
        }
        0
    }
    unsafe extern "C" fn failure(_: *mut c_void, _: i32, _: i32, _: *mut ae::PF_Pixel, _: *mut ae::PF_Pixel) -> i32 {
        123
    }
    unsafe extern "C" fn cancel(_: ae::PF_ProgPtr) -> i32 {
        321
    }
    #[test]
    fn clipped_fill_preserves_padding_and_clears_with_null_color() {
        unsafe {
            let mut data = [99; 48];
            let mut w = buffer(&mut data);
            let rect = ae::PF_Rect { left: -20, top: 1, right: 2, bottom: 100 };
            assert_eq!(fill(ptr::null_mut(), ptr::null(), &rect, &mut w), 0);
            assert_eq!(&data[16..24], &[0; 8]);
            assert_eq!(&data[24..32], &[99; 8]);
            assert_eq!(&data[..16], &[99; 16]);
            assert_eq!(&data[32..40], &[0; 8]);
            w.rowbytes = 1;
            assert_ne!(fill(ptr::null_mut(), ptr::null(), ptr::null(), &mut w), 0);
        }
    }
    #[test]
    fn iterate_supports_null_source_in_place_and_propagates_errors() {
        unsafe {
            let mut data = [0; 48];
            let mut w = buffer(&mut data);
            let mut input = std::mem::zeroed();
            assert_eq!(iterate(&mut input, 0, 0, ptr::null_mut(), ptr::null(), ptr::null_mut(), Some(shade), &mut w), 0);
            assert_eq!(&data[32..44], &[255, 17, 0, 2, 255, 17, 1, 2, 255, 17, 2, 2]);
            assert_eq!(&data[44..48], &[0; 4]);
            assert_eq!(iterate(&mut input, 0, 3, &mut w, ptr::null(), ptr::null_mut(), Some(shade), &mut w), 0);
            assert_eq!(data[1], 18);
            assert_eq!(iterate(&mut input, 0, 0, &mut w, ptr::null(), ptr::null_mut(), Some(failure), &mut w), 123);
            input.inter.abort = Some(cancel);
            assert_eq!(iterate(&mut input, 0, 0, &mut w, ptr::null(), ptr::null_mut(), Some(shade), &mut w), 321);
        }
    }
}
