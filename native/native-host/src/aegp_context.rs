//! Read-only single-image layer context. It is not an AE project or a licensing service.
use crate::event;
use after_effects_sys as ae;
use serde_json::json;
use std::{cell::RefCell, ptr};

struct Context {
    effect: ae::PF_ProgPtr,
    layer_token: usize,
    comp_token: usize,
    dimensions: [u32; 2],
}
thread_local! {static CONTEXT: RefCell<Option<Box<Context>>> = const {RefCell::new(None)};}
pub struct Guard;
impl Guard {
    pub fn enter(effect: ae::PF_ProgPtr, dimensions: [u32; 2]) -> Self {
        CONTEXT.with(|c| *c.borrow_mut() = Some(Box::new(Context { effect, layer_token: 0, comp_token: 0, dimensions })));
        Self
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        CONTEXT.with(|c| *c.borrow_mut() = None);
    }
}
fn with_layer(layer: ae::AEGP_LayerH, name: &str, write: impl FnOnce()) -> i32 {
    CONTEXT.with(|c| {
        let c = c.borrow();
        let Some(c) = c.as_ref() else {
            return ae::PF_Err_BAD_CALLBACK_PARAM;
        };
        if layer != ptr::from_ref(&c.layer_token).cast_mut().cast() {
            return ae::PF_Err_BAD_CALLBACK_PARAM;
        }
        event(json!({"stage":"aegp_callback","name":name}));
        write();
        0
    })
}
pub unsafe extern "C" fn effect_layer(effect: ae::PF_ProgPtr, out: *mut ae::AEGP_LayerH) -> i32 {
    if out.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    unsafe {
        *out = ptr::null_mut();
    }
    CONTEXT.with(|c| {
        let c = c.borrow();
        let Some(c) = c.as_ref().filter(|c| c.effect == effect) else {
            return ae::PF_Err_BAD_CALLBACK_PARAM;
        };
        unsafe {
            *out = ptr::from_ref(&c.layer_token).cast_mut().cast();
        }
        event(json!({"stage":"aegp_callback","name":"AEGP_GetEffectLayer"}));
        0
    })
}
pub unsafe extern "C" fn effect_camera(effect: ae::PF_ProgPtr, time: *const ae::A_Time, out: *mut ae::AEGP_LayerH) -> i32 {
    event(json!({"stage":"aegp_callback","name":"AEGP_GetEffectCamera"}));
    if out.is_null() || time.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    unsafe {
        *out = ptr::null_mut();
    }
    let mut layer = ptr::null_mut();
    unsafe { effect_layer(effect, &mut layer) }
}
pub unsafe extern "C" fn comp_time(effect: ae::PF_ProgPtr, time: i32, scale: u32, out: *mut ae::A_Time) -> i32 {
    event(json!({"stage":"aegp_callback","name":"AEGP_ConvertEffectToCompTime"}));
    if out.is_null() || scale == 0 {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let mut layer = ptr::null_mut();
    let error = unsafe { effect_layer(effect, &mut layer) };
    if error == 0 {
        unsafe {
            *out = ae::A_Time { value: time, scale };
        }
    }
    error
}

pub unsafe extern "C" fn register(_refcon: ae::AEGP_GlobalRefcon, name: *const ae::A_char, out: *mut ae::AEGP_PluginID) -> i32 {
    use std::sync::Mutex;
    static NEXT_ID: Mutex<i32> = Mutex::new(1);
    if name.is_null() || out.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    // A process-local host allocation, not Adobe application registration or licensing.
    let Ok(mut next) = NEXT_ID.lock() else {
        return ae::PF_Err_OUT_OF_MEMORY;
    };
    let id = *next;
    let Some(incremented) = id.checked_add(1) else {
        return ae::PF_Err_OUT_OF_MEMORY;
    };
    *next = incremented;
    unsafe {
        *out = id;
    }
    event(json!({"stage":"aegp_callback","name":"AEGP_RegisterWithAEGP","hostLocalId":id}));
    0
}
pub unsafe extern "C" fn layer_flags(layer: ae::AEGP_LayerH, out: *mut ae::AEGP_LayerFlags) -> i32 {
    if out.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    with_layer(layer, "AEGP_GetLayerFlags", || unsafe {
        *out = (ae::AEGP_LayerFlag_VIDEO_ACTIVE | ae::AEGP_LayerFlag_EFFECTS_ACTIVE) as _;
    })
}
pub unsafe extern "C" fn layer_3d(layer: ae::AEGP_LayerH, out: *mut ae::A_Boolean) -> i32 {
    if out.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    with_layer(layer, "AEGP_IsLayer3D", || unsafe {
        *out = 0;
    })
}
pub unsafe extern "C" fn layer_2d(layer: ae::AEGP_LayerH, out: *mut ae::A_Boolean) -> i32 {
    if out.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    with_layer(layer, "AEGP_IsLayer2D", || unsafe {
        *out = 1;
    })
}
pub unsafe extern "C" fn object_type(layer: ae::AEGP_LayerH, out: *mut ae::AEGP_ObjectType) -> i32 {
    if out.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    with_layer(layer, "AEGP_GetLayerObjectType", || unsafe {
        *out = ae::AEGP_ObjectType_AV;
    })
}

/// This worker owns a single raster input, without a separate editable mask stack.
/// Its layer-space bounds are the logical input extent, independent of preview scale.
pub unsafe extern "C" fn masked_bounds(layer: ae::AEGP_LayerH, time_mode: ae::AEGP_LTimeMode, time: *const ae::A_Time, out: *mut ae::A_FloatRect) -> i32 {
    if out.is_null() || time.is_null() || !matches!(i32::from(time_mode), ae::AEGP_LTimeMode_LayerTime | ae::AEGP_LTimeMode_CompTime) {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    if unsafe { (*time).scale } == 0 {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    with_layer(layer, "AEGP_GetLayerMaskedBounds", || {
        CONTEXT.with(|c| {
            if let Some(context) = c.borrow().as_ref() {
                unsafe {
                    *out = ae::A_FloatRect { left: 0.0, top: 0.0, right: f64::from(context.dimensions[0]), bottom: f64::from(context.dimensions[1]) };
                }
            }
        });
    })
}

/// The worker's one-layer render context has no offset or stretch. The caller has
/// already selected the source frame before crossing this process boundary.
pub unsafe extern "C" fn layer_time(layer: ae::AEGP_LayerH, time: *const ae::A_Time, out: *mut ae::A_Time) -> i32 {
    if out.is_null() || time.is_null() || unsafe { (*time).scale } == 0 {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    with_layer(layer, "AEGP_ConvertLayerToCompTime", || unsafe { *out = *time })
}

/// Effects run on the source raster before the compositor applies layer transforms.
/// This isolated one-layer context therefore has an identity world transform.
pub unsafe extern "C" fn layer_world(layer: ae::AEGP_LayerH, time: *const ae::A_Time, out: *mut ae::A_Matrix4) -> i32 {
    if out.is_null() || time.is_null() || unsafe { (*time).scale } == 0 {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    with_layer(layer, "AEGP_GetLayerToWorldXform", || unsafe {
        *out = ae::A_Matrix4 { mat: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]] };
    })
}

pub unsafe extern "C" fn parent_comp(layer: ae::AEGP_LayerH, out: *mut ae::AEGP_CompH) -> i32 {
    if out.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    unsafe { *out = ptr::null_mut() };
    with_layer(layer, "AEGP_GetLayerParentComp", || {
        CONTEXT.with(|c| {
            if let Some(context) = c.borrow().as_ref() {
                unsafe { *out = ptr::from_ref(&context.comp_token).cast_mut().cast() };
            }
        });
    })
}

/// No ICC working profile is attached to this raster-only PNG transport. Do not
/// invent a vendor profile or inherit the separate Adobe application's settings.
pub unsafe extern "C" fn working_space(_plugin: ae::AEGP_PluginID, comp: ae::AEGP_CompH, out: *mut ae::AEGP_ColorProfileP) -> i32 {
    if out.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    unsafe { *out = ptr::null_mut() };
    CONTEXT.with(|c| {
        let c = c.borrow();
        if !c.as_ref().is_some_and(|context| comp == ptr::from_ref(&context.comp_token).cast_mut().cast()) {
            return ae::PF_Err_BAD_CALLBACK_PARAM;
        }
        event(json!({"stage":"aegp_callback","name":"AEGP_GetNewWorkingSpaceColorProfile","profile":null,"colorManagement":"unmanaged"}));
        0
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn context_rejects_foreign_and_expired_handles() {
        let mut token = 0usize;
        let effect = ptr::from_mut(&mut token).cast();
        let guard = Guard::enter(effect, [1920, 1080]);
        let mut layer = ptr::null_mut();
        let mut flags = 0;
        unsafe {
            assert_eq!(effect_layer(effect, &mut layer), 0);
            assert_eq!(layer_flags(layer, &mut flags), 0);
            assert_eq!(flags, ae::AEGP_LayerFlag_VIDEO_ACTIVE | ae::AEGP_LayerFlag_EFFECTS_ACTIVE);
            let mut converted = ae::A_Time { value: 0, scale: 0 };
            let rational = ae::A_Time { value: -31, scale: 60 };
            assert_eq!(layer_time(layer, &rational, &mut converted), 0);
            assert_eq!((converted.value, converted.scale), (-31, 60));
            let mut comp = ptr::null_mut();
            assert_eq!(parent_comp(layer, &mut comp), 0);
            let mut profile = ptr::dangling_mut();
            assert_eq!(working_space(1, comp, &mut profile), 0);
            assert!(profile.is_null());
            assert_ne!(working_space(1, ptr::null_mut(), &mut profile), 0);
            let mut matrix: ae::A_Matrix4 = std::mem::zeroed();
            assert_eq!(layer_world(layer, &rational, &mut matrix), 0);
            assert_eq!(matrix.mat[0], [1.0, 0.0, 0.0, 0.0]);
            assert_eq!(matrix.mat[3], [0.0, 0.0, 0.0, 1.0]);
            let mut bounds: ae::A_FloatRect = std::mem::zeroed();
            let time = ae::A_Time { value: 0, scale: 25 };
            assert_eq!(masked_bounds(layer, ae::AEGP_LTimeMode_LayerTime as _, &time, &mut bounds), 0);
            assert_eq!([bounds.left, bounds.top, bounds.right, bounds.bottom], [0.0, 0.0, 1920.0, 1080.0]);
            assert_ne!(masked_bounds(layer, ae::AEGP_LTimeMode_LayerTime as _, ptr::null(), &mut bounds), 0);
            assert_ne!(layer_flags(ptr::null_mut(), &mut flags), 0);
            drop(guard);
            assert_ne!(layer_flags(layer, &mut flags), 0);
        }
    }
}
