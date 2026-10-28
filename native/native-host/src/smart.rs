//! CPU/8-bit SmartFX with current-time checkouts. No GPU context is advertised.
use crate::{event, native};
use after_effects_sys as ae;
use serde_json::json;
use std::{cell::RefCell, collections::BTreeMap, ffi::c_void, ptr};
pub type Entry = unsafe extern "C" fn(i32, *mut ae::PF_InData, *mut ae::PF_OutData, ae::PF_ParamList, *mut ae::PF_LayerDef, *mut c_void) -> i32;
struct State {
    effect: ae::PF_ProgPtr,
    output: *mut ae::PF_EffectWorld,
    layers: BTreeMap<i32, Box<ae::PF_EffectWorld>>,
    guid: u64,
    reference_size: [i32; 2],
}
thread_local! {static STATE:RefCell<Option<State>>=const{RefCell::new(None)};}
struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        STATE.with(|s| *s.borrow_mut() = None);
    }
}
unsafe extern "C" fn checkout_layer(
    effect: ae::PF_ProgPtr,
    index: i32,
    id: i32,
    request: *const ae::PF_RenderRequest,
    time: i32,
    _step: i32,
    scale: u32,
    out: *mut ae::PF_CheckoutResult,
) -> i32 {
    if request.is_null() || out.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    if !native::same_time(time, scale) {
        event(json!({"stage":"unsupported_checkout","index":index,"time":time,"scale":scale,"smart":true}));
        return ae::PF_Err_INVALID_CALLBACK;
    }
    let Some(world) = native::layer_world(index) else {
        return ae::PF_Err_INVALID_INDEX;
    };
    let request = unsafe { &*request };
    if request.rect.left > request.rect.right || request.rect.top > request.rect.bottom {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    STATE.with(|s| {
        let Ok(mut s) = s.try_borrow_mut() else {
            return ae::PF_Err_BAD_CALLBACK_PARAM;
        };
        let Some(s) = s.as_mut().filter(|s| s.effect == effect) else {
            return ae::PF_Err_BAD_CALLBACK_PARAM;
        };
        if s.layers.len() >= 1024 || s.layers.contains_key(&id) {
            return ae::PF_Err_BAD_CALLBACK_PARAM;
        }
        let full = ae::PF_LRect { left: 0, top: 0, right: world.width.max(0), bottom: world.height.max(0) };
        let mut result: ae::PF_CheckoutResult = unsafe { std::mem::zeroed() };
        result.max_result_rect = full;
        result.result_rect = intersection(full, request.rect);
        result.par = ae::PF_RationalScale { num: 1, den: 1 };
        result.ref_width = s.reference_size[0];
        result.ref_height = s.reference_size[1];
        s.layers.insert(id, Box::new(world));
        unsafe {
            *out = result;
        }
        event(json!({"stage":"smart_checkout","index":index,"id":id}));
        0
    })
}
fn intersection(a: ae::PF_LRect, b: ae::PF_LRect) -> ae::PF_LRect {
    let left = a.left.max(b.left);
    let top = a.top.max(b.top);
    let right = a.right.min(b.right);
    let bottom = a.bottom.min(b.bottom);
    if left >= right || top >= bottom { ae::PF_LRect { left: 0, top: 0, right: 0, bottom: 0 } } else { ae::PF_LRect { left, top, right, bottom } }
}
unsafe extern "C" fn mix(effect: ae::PF_ProgPtr, len: u32, data: *const c_void) -> i32 {
    if len > 1024 * 1024 || (len > 0 && data.is_null()) {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    STATE.with(|s| {
        let Ok(mut s) = s.try_borrow_mut() else {
            return ae::PF_Err_BAD_CALLBACK_PARAM;
        };
        let Some(s) = s.as_mut().filter(|s| s.effect == effect) else {
            return ae::PF_Err_BAD_CALLBACK_PARAM;
        };
        for offset in 0..len as usize {
            s.guid = (s.guid ^ u64::from(unsafe { *data.cast::<u8>().add(offset) })).wrapping_mul(1099511628211);
        }
        0
    })
}
unsafe extern "C" fn pixels(effect: ae::PF_ProgPtr, id: i32, out: *mut *mut ae::PF_EffectWorld) -> i32 {
    if out.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    unsafe {
        *out = ptr::null_mut();
    }
    STATE.with(|s| {
        let Ok(mut s) = s.try_borrow_mut() else {
            return ae::PF_Err_BAD_CALLBACK_PARAM;
        };
        let Some(s) = s.as_mut().filter(|s| s.effect == effect) else {
            return ae::PF_Err_BAD_CALLBACK_PARAM;
        };
        let Some(world) = s.layers.get_mut(&id) else {
            return ae::PF_Err_INVALID_INDEX;
        };
        unsafe {
            *out = &mut **world;
        }
        0
    })
}
unsafe extern "C" fn checkin(effect: ae::PF_ProgPtr, id: i32) -> i32 {
    STATE.with(|s| {
        let Ok(s) = s.try_borrow() else {
            return ae::PF_Err_BAD_CALLBACK_PARAM;
        };
        if s.as_ref().is_some_and(|s| s.effect == effect && s.layers.contains_key(&id)) { 0 } else { ae::PF_Err_INVALID_INDEX }
    })
}
unsafe extern "C" fn output(effect: ae::PF_ProgPtr, out: *mut *mut ae::PF_EffectWorld) -> i32 {
    if out.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    unsafe {
        *out = ptr::null_mut();
    }
    STATE.with(|s| {
        let Ok(s) = s.try_borrow() else {
            return ae::PF_Err_BAD_CALLBACK_PARAM;
        };
        let Some(s) = s.as_ref().filter(|s| s.effect == effect) else {
            return ae::PF_Err_BAD_CALLBACK_PARAM;
        };
        unsafe {
            *out = s.output;
        }
        0
    })
}
struct PreData(ae::PF_PreRenderOutput);
impl Drop for PreData {
    fn drop(&mut self) {
        if !self.0.pre_render_data.is_null()
            && let Some(delete) = self.0.delete_pre_render_data_func
        {
            unsafe {
                delete(self.0.pre_render_data);
            }
        }
    }
}
fn invoke(entry: Entry, command: i32, name: &str, input: &mut ae::PF_InData, out: &mut ae::PF_OutData, extra: *mut c_void) -> i32 {
    out.return_msg.fill(0);
    event(json!({"stage":name}));
    let code = unsafe { entry(command, input, out, ptr::null_mut(), ptr::null_mut(), extra) };
    let message: Vec<u8> = out.return_msg.iter().take_while(|c| **c != 0).map(|c| *c as u8).collect();
    event(
        json!({"stage":format!("{name}_result"),"code":code,"message":String::from_utf8_lossy(&message),"errorMessage":out.out_flags & ae::PF_OutFlag_DISPLAY_ERROR_MESSAGE !=0}),
    );
    code
}
pub fn render(entry: Entry, input: &mut ae::PF_InData, out: &mut ae::PF_OutData, world: &mut ae::PF_EffectWorld) -> i32 {
    STATE.with(|s| {
        *s.borrow_mut() = Some(State {
            effect: input.effect_ref,
            output: world,
            layers: BTreeMap::new(),
            guid: 14695981039346656037,
            reference_size: [input.width, input.height],
        })
    });
    let _guard = Guard;
    let mut request: ae::PF_RenderRequest = unsafe { std::mem::zeroed() };
    request.rect = ae::PF_LRect { left: 0, top: 0, right: world.width, bottom: world.height };
    request.channel_mask = ae::PF_ChannelMask_ARGB;
    request.field = ae::PF_Field_FRAME;
    let mut pre_in: ae::PF_PreRenderInput = unsafe { std::mem::zeroed() };
    pre_in.output_request = request;
    pre_in.bitdepth = 8;
    pre_in.what_gpu = ae::PF_GPU_Framework_NONE;
    let mut pre_out: ae::PF_PreRenderOutput = unsafe { std::mem::zeroed() };
    let mut pre_cb = ae::PF_PreRenderCallbacks { checkout_layer: Some(checkout_layer), GuidMixInPtr: Some(mix) };
    let mut pre = ae::PF_PreRenderExtra { input: &mut pre_in, output: &mut pre_out, cb: &mut pre_cb };
    let code = invoke(entry, ae::PF_Cmd_SMART_PRE_RENDER, "smart_pre_render", input, out, (&mut pre as *mut ae::PF_PreRenderExtra).cast());
    let pre_data = PreData(pre_out);
    if code != 0 {
        return code;
    }
    // SmartFX may use out_data.frame_data as well as pre_render_data. It is
    // produced by pre-render and must be visible during the matching render.
    input.frame_data = out.frame_data;
    let r = pre_out.result_rect;
    if r.left < 0 || r.top < 0 || r.right > world.width || r.bottom > world.height || r.left > r.right || r.top > r.bottom {
        event(
            json!({"stage":"unsupported_callback","name":"expanded_smart_output","result":[r.left,r.top,r.right,r.bottom],"requested":[0,0,world.width,world.height]}),
        );
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let mut smart_in: ae::PF_SmartRenderInput = unsafe { std::mem::zeroed() };
    smart_in.output_request = request;
    smart_in.bitdepth = 8;
    smart_in.pre_render_data = pre_data.0.pre_render_data;
    smart_in.what_gpu = ae::PF_GPU_Framework_NONE;
    let mut cb = ae::PF_SmartRenderCallbacks { checkout_layer_pixels: Some(pixels), checkin_layer_pixels: Some(checkin), checkout_output: Some(output) };
    let mut extra = ae::PF_SmartRenderExtra { input: &mut smart_in, cb: &mut cb };
    invoke(entry, ae::PF_Cmd_SMART_RENDER, "smart_render", input, out, (&mut extra as *mut ae::PF_SmartRenderExtra).cast())
}
#[cfg(test)]
mod tests {
    use super::*;
    unsafe extern "C" fn frame_fixture(
        command: i32,
        input: *mut ae::PF_InData,
        out: *mut ae::PF_OutData,
        _: ae::PF_ParamList,
        _: *mut ae::PF_LayerDef,
        extra: *mut c_void,
    ) -> i32 {
        if command == ae::PF_Cmd_SMART_PRE_RENDER {
            let pre = unsafe { &mut *extra.cast::<ae::PF_PreRenderExtra>() };
            let request = unsafe { (*pre.input).output_request.rect };
            unsafe {
                (*pre.output).result_rect = request;
                (*pre.output).max_result_rect = request;
                (*out).frame_data = (*input).global_data;
                if (*input).width == 100 {
                    (*pre.output).result_rect.left = -10;
                }
            }
            return 0;
        }
        if command == ae::PF_Cmd_SMART_RENDER {
            return if unsafe { (*input).frame_data == (*input).global_data } { 0 } else { ae::PF_Err_BAD_CALLBACK_PARAM };
        }
        ae::PF_Err_INVALID_CALLBACK
    }
    #[test]
    fn smart_frame_data_reaches_render_and_expanded_requests_fail_before_render() {
        unsafe {
            let mut input: ae::PF_InData = std::mem::zeroed();
            input.effect_ref = ptr::dangling_mut();
            input.global_data = ptr::dangling_mut();
            input.width = 3;
            input.height = 2;
            let mut out = std::mem::zeroed();
            let mut world: ae::PF_EffectWorld = std::mem::zeroed();
            world.width = 3;
            world.height = 2;
            assert_eq!(render(frame_fixture, &mut input, &mut out, &mut world), 0);
            assert_eq!(input.frame_data, input.global_data);
            input.width = 100;
            assert_eq!(render(frame_fixture, &mut input, &mut out, &mut world), ae::PF_Err_BAD_CALLBACK_PARAM);
        }
    }
    #[test]
    fn clips_empty_and_partial_requests_without_overflow() {
        let full = ae::PF_LRect { left: 0, top: 0, right: 32, bottom: 16 };
        let c = intersection(full, ae::PF_LRect { left: i32::MIN, top: 5, right: 10, bottom: i32::MAX });
        assert_eq!((c.left, c.top, c.right, c.bottom), (0, 5, 10, 16));
        let c = intersection(full, ae::PF_LRect { left: 40, top: 0, right: 50, bottom: 20 });
        assert_eq!((c.left, c.right), (0, 0));
    }
}
