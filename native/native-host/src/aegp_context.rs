//! Read-only single-image layer context. It is not an AE project or a licensing service.
use crate::event;
use after_effects_sys as ae;
use serde_json::json;
use std::{cell::RefCell, ptr};

struct Context {
    effect: ae::PF_ProgPtr,
    layer_token: usize,
}
thread_local! {static CONTEXT: RefCell<Option<Box<Context>>> = const {RefCell::new(None)};}
pub struct Guard;
impl Guard {
    pub fn enter(effect: ae::PF_ProgPtr) -> Self {
        CONTEXT.with(|c| *c.borrow_mut() = Some(Box::new(Context { effect, layer_token: 0 })));
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn context_rejects_foreign_and_expired_handles() {
        let mut token = 0usize;
        let effect = ptr::from_mut(&mut token).cast();
        let guard = Guard::enter(effect);
        let mut layer = ptr::null_mut();
        let mut flags = 0;
        unsafe {
            assert_eq!(effect_layer(effect, &mut layer), 0);
            assert_eq!(layer_flags(layer, &mut flags), 0);
            assert_eq!(flags, ae::AEGP_LayerFlag_VIDEO_ACTIVE | ae::AEGP_LayerFlag_EFFECTS_ACTIVE);
            assert_ne!(layer_flags(ptr::null_mut(), &mut flags), 0);
            drop(guard);
            assert_ne!(layer_flags(layer, &mut flags), 0);
        }
    }
}
