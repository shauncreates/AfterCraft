//! Original test effect. It uses the public AE effect ABI and no proprietary implementations.
use after_effects_sys as ae;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
static SEQUENCE_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[unsafe(no_mangle)]
/// Entry point for the original synthetic gain effect.
///
/// # Safety
/// The caller must provide the SDK-defined valid pointers for each selector. During render,
/// parameter pointers and worlds must remain valid, with buffers covering their declared rows.
pub unsafe extern "C" fn EffectMain(
    cmd: i32,
    input: *mut ae::PF_InData,
    output: *mut ae::PF_OutData,
    params: ae::PF_ParamList,
    dest: *mut ae::PF_LayerDef,
    _extra: *mut c_void,
) -> i32 {
    if input.is_null() || output.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let (input, output) = unsafe { (&mut *input, &mut *output) };
    match cmd {
        ae::PF_Cmd_GLOBAL_SETUP => {
            SEQUENCE_INITIALIZED.store(false, Ordering::Relaxed);
            output.my_version = 1;
            output.out_flags = ae::PF_OutFlag_PIX_INDEPENDENT;
            output.out_flags2 = ae::PF_OutFlag2_SUPPORTS_SMART_RENDER;
            0
        }
        ae::PF_Cmd_PARAMS_SETUP => {
            let Some(add) = input.inter.add_param else {
                return ae::PF_Err_INVALID_CALLBACK;
            };
            let mut def: ae::PF_ParamDef = unsafe { std::mem::zeroed() };
            def.param_type = ae::PF_Param_FLOAT_SLIDER;
            for (a, b) in def.name_do_not_use_directly.iter_mut().zip(b"Gain") {
                *a = *b as i8;
            }
            def.u.fs_d.dephault = 2.0;
            def.u.fs_d.valid_min = 0.0;
            def.u.fs_d.valid_max = 4.0;
            def.u.fs_d.slider_min = 0.0;
            def.u.fs_d.slider_max = 4.0;
            let code = unsafe { add(input.effect_ref, -1, &mut def) };
            if code != 0 {
                return code;
            }
            let mut clock: ae::PF_ParamDef = unsafe { std::mem::zeroed() };
            clock.param_type = ae::PF_Param_CHECKBOX;
            for (a, b) in clock.name_do_not_use_directly.iter_mut().zip(b"Encode Clock") {
                *a = *b as i8;
            }
            let code = unsafe { add(input.effect_ref, -1, &mut clock) };
            if code != 0 {
                return code;
            }
            let mut color: ae::PF_ParamDef = unsafe { std::mem::zeroed() };
            color.param_type = ae::PF_Param_COLOR;
            color.u.cd.dephault = ae::PF_Pixel { alpha: 255, red: 255, green: 255, blue: 255 };
            let mut point: ae::PF_ParamDef = unsafe { std::mem::zeroed() };
            point.param_type = ae::PF_Param_POINT;
            let mut angle: ae::PF_ParamDef = unsafe { std::mem::zeroed() };
            angle.param_type = ae::PF_Param_ANGLE;
            let mut mode: ae::PF_ParamDef = unsafe { std::mem::zeroed() };
            mode.param_type = ae::PF_Param_POPUP;
            mode.u.pd.num_choices = 2;
            mode.u.pd.dephault = 1;
            mode.u.pd.u.namesptr = c"Gain|Typed Controls".as_ptr();
            for mut def in [color, point, angle, mode] {
                let code = unsafe { add(input.effect_ref, -1, &mut def) };
                if code != 0 {
                    return code;
                }
            }
            let mut preview: ae::PF_ParamDef = unsafe { std::mem::zeroed() };
            preview.param_type = ae::PF_Param_CHECKBOX;
            let code = unsafe { add(input.effect_ref, -1, &mut preview) };
            if code != 0 {
                return code;
            }
            output.num_params = 8;
            0
        }
        ae::PF_Cmd_SEQUENCE_SETUP => {
            SEQUENCE_INITIALIZED.store(true, Ordering::Relaxed);
            if input.utils.is_null() {
                return ae::PF_Err_BAD_CALLBACK_PARAM;
            }
            let Some(new_handle) = (unsafe { (*input.utils).host_new_handle }) else {
                return ae::PF_Err_INVALID_CALLBACK;
            };
            output.sequence_data = unsafe { new_handle(8) };
            if output.sequence_data.is_null() { ae::PF_Err_OUT_OF_MEMORY } else { 0 }
        }
        ae::PF_Cmd_SEQUENCE_SETDOWN => {
            if input.utils.is_null() {
                return ae::PF_Err_BAD_CALLBACK_PARAM;
            }
            let Some(dispose) = (unsafe { (*input.utils).host_dispose_handle }) else {
                return ae::PF_Err_INVALID_CALLBACK;
            };
            unsafe {
                dispose(input.sequence_data);
            }
            output.sequence_data = std::ptr::null_mut();
            0
        }
        ae::PF_Cmd_SMART_PRE_RENDER => unsafe { pre_render(input, _extra) },
        ae::PF_Cmd_GLOBAL_SETDOWN => {
            if SEQUENCE_INITIALIZED.load(Ordering::Relaxed) {
                0
            } else {
                ae::PF_Err_BAD_CALLBACK_PARAM
            }
        }
        ae::PF_Cmd_SMART_RENDER => unsafe { smart_render(input, output, _extra) },
        ae::PF_Cmd_RENDER => {
            let service_result = unsafe { check_layer_services(input) };
            if service_result != 0 {
                return service_result;
            }
            for (dest, byte) in output.return_msg.iter_mut().zip(b"Synthetic fixture render diagnostic") {
                *dest = *byte as i8;
            }
            println!("Synthetic fixture stdout diagnostic");
            if params.is_null() || dest.is_null() {
                return ae::PF_Err_BAD_CALLBACK_PARAM;
            }
            let (first, gain, clock) = unsafe { (*params, *params.add(1), *params.add(2)) };
            if first.is_null() || gain.is_null() || clock.is_null() {
                return ae::PF_Err_BAD_CALLBACK_PARAM;
            }
            let (source, gain, dest) = unsafe { ((*first).u.ld, (*gain).u.fs_d.value, &mut *dest) };
            let encode_clock = unsafe { (*clock).u.bd.value != 0 };
            let typed = unsafe { *params.add(6) };
            let color = unsafe { *params.add(3) };
            let point = unsafe { *params.add(4) };
            let angle = unsafe { *params.add(5) };
            let preview = unsafe { *params.add(7) };
            if typed.is_null() || color.is_null() || point.is_null() || angle.is_null() || preview.is_null() {
                return ae::PF_Err_BAD_CALLBACK_PARAM;
            }
            let typed = unsafe { (*typed).u.pd.value == 2 };
            if source.data.is_null()
                || dest.data.is_null()
                || source.width != dest.width
                || source.height != dest.height
                || source.rowbytes < source.width * 4
                || dest.rowbytes < dest.width * 4
            {
                return ae::PF_Err_BAD_CALLBACK_PARAM;
            }
            if input.utils.is_null() {
                return ae::PF_Err_BAD_CALLBACK_PARAM;
            }
            let (Some(fill), Some(iterate)) = (unsafe { ((*input.utils).fill, (*input.utils).iterate) }) else {
                return ae::PF_Err_INVALID_CALLBACK;
            };
            let result = unsafe { fill(input.effect_ref, std::ptr::null(), std::ptr::null(), dest) };
            if result != 0 {
                return result;
            }
            let mut context = PixelContext {
                gain,
                fixed: if unsafe { (*preview).u.bd.value != 0 } {
                    Some(ae::PF_Pixel {
                        alpha: 255,
                        red: input.width.rem_euclid(256) as u8,
                        green: (i64::from(input.downsample_x.den) * 16 + i64::from(input.downsample_x.num)).rem_euclid(256) as u8,
                        blue: (unsafe { (i64::from((*point).u.td.x_value) + i64::from((*point).u.td.y_value)) / 65536 }).rem_euclid(256) as u8,
                    })
                } else if encode_clock {
                    Some(ae::PF_Pixel {
                        alpha: 255,
                        red: input.time_step.rem_euclid(256) as u8,
                        green: (input.time_scale % 256) as u8,
                        blue: input.current_time.rem_euclid(256) as u8,
                    })
                } else if typed {
                    Some(unsafe {
                        ae::PF_Pixel {
                            alpha: 255,
                            red: (*color).u.cd.value.red,
                            green: ((i64::from((*point).u.td.x_value) + i64::from((*point).u.td.y_value)) / 65536).rem_euclid(256) as u8,
                            blue: ((*angle).u.ad.value / 65536).rem_euclid(256) as u8,
                        }
                    })
                } else {
                    None
                },
            };
            let mut source = source;
            unsafe { iterate(input, 0, source.height, &mut source, std::ptr::null(), (&mut context as *mut PixelContext).cast(), Some(render_pixel), dest) }
        }
        _ => 0,
    }
}

struct PixelContext {
    gain: f64,
    fixed: Option<ae::PF_Pixel>,
}
unsafe extern "C" fn render_pixel(context: *mut c_void, _x: i32, _y: i32, src: *mut ae::PF_Pixel, dst: *mut ae::PF_Pixel) -> i32 {
    if context.is_null() || src.is_null() || dst.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let context = unsafe { &*context.cast::<PixelContext>() };
    let src = unsafe { *src };
    let scale = |c: u8| (f64::from(c) * context.gain).round().clamp(0.0, 255.0) as u8;
    let mut result = context.fixed.unwrap_or(ae::PF_Pixel { alpha: src.alpha, red: scale(src.red), green: scale(src.green), blue: scale(src.blue) });
    result.alpha = src.alpha;
    unsafe {
        dst.write(result);
    }
    0
}

unsafe extern "C" fn delete_pre_render(data: *mut c_void) {
    if !data.is_null() {
        unsafe {
            drop(Box::from_raw(data.cast::<u64>()));
        }
    }
    println!("Synthetic SmartFX pre-render data disposed");
}
unsafe fn pre_render(input: &ae::PF_InData, extra: *mut c_void) -> i32 {
    if extra.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let extra = unsafe { &mut *extra.cast::<ae::PF_PreRenderExtra>() };
    if extra.input.is_null() || extra.output.is_null() || extra.cb.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let Some(checkout) = (unsafe { (*extra.cb).checkout_layer }) else {
        return ae::PF_Err_INVALID_CALLBACK;
    };
    let mut result: ae::PF_CheckoutResult = unsafe { std::mem::zeroed() };
    let code = unsafe { checkout(input.effect_ref, 0, 0, &(*extra.input).output_request, input.current_time, input.time_step, input.time_scale, &mut result) };
    if code != 0 {
        return code;
    }
    unsafe {
        (*extra.output).result_rect = result.result_rect;
        if result.ref_width != input.width || result.ref_height != input.height {
            return ae::PF_Err_BAD_CALLBACK_PARAM;
        }
        (*extra.output).max_result_rect = result.max_result_rect;
        (*extra.output).pre_render_data = Box::into_raw(Box::new(0xECAE_1234_u64)).cast();
        (*extra.output).delete_pre_render_data_func = Some(delete_pre_render);
    }
    0
}
unsafe fn smart_render(input: &mut ae::PF_InData, output: &mut ae::PF_OutData, extra: *mut c_void) -> i32 {
    use std::ptr;
    if extra.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let extra = unsafe { &mut *extra.cast::<ae::PF_SmartRenderExtra>() };
    if extra.input.is_null() || extra.cb.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let data = unsafe { (*extra.input).pre_render_data };
    if data.is_null() || unsafe { *data.cast::<u64>() } != 0xECAE_1234_u64 {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let cb = unsafe { &*extra.cb };
    let (Some(source), Some(dest), Some(checkin_pixels), Some(param), Some(checkin_param)) =
        (cb.checkout_layer_pixels, cb.checkout_output, cb.checkin_layer_pixels, input.inter.checkout_param, input.inter.checkin_param)
    else {
        return ae::PF_Err_INVALID_CALLBACK;
    };
    let mut src = ptr::null_mut();
    let mut dst = ptr::null_mut();
    let code = unsafe { source(input.effect_ref, 0, &mut src) };
    if code != 0 {
        return code;
    }
    let code = unsafe { dest(input.effect_ref, &mut dst) };
    if code != 0 || src.is_null() || dst.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let mut defs: [ae::PF_ParamDef; 8] = unsafe { std::mem::zeroed() };
    let Some(first) = defs.first_mut() else {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    };
    first.param_type = ae::PF_Param_LAYER;
    first.u.ld = unsafe { *src };
    for index in 1..8 {
        let Some(def) = defs.get_mut(index) else {
            return ae::PF_Err_INVALID_INDEX;
        };
        let code = unsafe { param(input.effect_ref, index as i32, input.current_time, input.time_step, input.time_scale, def) };
        if code != 0 {
            return code;
        }
    }
    let mut pointers: Vec<_> = defs.iter_mut().map(|d| d as *mut _).collect();
    let code = unsafe { EffectMain(ae::PF_Cmd_RENDER, input, output, pointers.as_mut_ptr(), dst, ptr::null_mut()) };
    for def in defs.iter_mut().skip(1) {
        unsafe {
            checkin_param(input.effect_ref, def);
        }
    }
    unsafe {
        checkin_pixels(input.effect_ref, 0);
    }
    code
}

// Exercise the suite tables across a real DLL boundary, using only the public SDK types.
unsafe fn check_layer_services(input: &ae::PF_InData) -> i32 {
    use std::ptr;
    if input.pica_basicP.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let basic = unsafe { &*input.pica_basicP };
    let (Some(acquire), Some(release)) = (basic.AcquireSuite, basic.ReleaseSuite) else {
        return ae::PF_Err_INVALID_CALLBACK;
    };
    let mut interface = ptr::null();
    let mut layer_suite = ptr::null();
    if unsafe { acquire(c"AEGP PF Interface Suite".as_ptr(), 1, &mut interface) } != 0
        || unsafe { acquire(c"AEGP Layer Suite".as_ptr(), 11, &mut layer_suite) } != 0
        || interface.is_null()
        || layer_suite.is_null()
    {
        return ae::PF_Err_INVALID_CALLBACK;
    }
    let interface = unsafe { &*interface.cast::<ae::AEGP_PFInterfaceSuite1>() };
    let layers = unsafe { &*layer_suite.cast::<ae::AEGP_LayerSuite5>() };
    let (Some(get_layer), Some(get_flags), Some(get_camera)) = (interface.AEGP_GetEffectLayer, layers.AEGP_GetLayerFlags, interface.AEGP_GetEffectCamera)
    else {
        return ae::PF_Err_INVALID_CALLBACK;
    };
    let mut layer = ptr::null_mut();
    let mut flags = 0;
    let mut camera = ptr::null_mut();
    let time = ae::A_Time { value: input.current_time, scale: input.time_scale };
    let mut sequence = ptr::null();
    if unsafe { acquire(c"PF Effect Sequence Data Suite".as_ptr(), 1, &mut sequence) } != 0 || sequence.is_null() {
        return ae::PF_Err_INVALID_CALLBACK;
    }
    let Some(get_sequence) = (unsafe { (*sequence.cast::<ae::PF_EffectSequenceDataSuite1>()).PF_GetConstSequenceData }) else {
        return ae::PF_Err_INVALID_CALLBACK;
    };
    let mut readonly = ptr::null();
    let valid = unsafe { get_sequence(input.effect_ref, &mut readonly) } == 0
        && !readonly.is_null()
        && std::ptr::eq(readonly.cast::<c_void>(), input.sequence_data.cast::<c_void>())
        && unsafe { get_layer(input.effect_ref, &mut layer) } == 0
        && !layer.is_null()
        && unsafe { get_flags(layer, &mut flags) } == 0
        && flags & ae::AEGP_LayerFlag_VIDEO_ACTIVE != 0
        && unsafe { get_camera(input.effect_ref, &time, &mut camera) } == 0
        && camera.is_null();
    unsafe {
        release(c"AEGP Layer Suite".as_ptr(), 11);
        release(c"AEGP PF Interface Suite".as_ptr(), 1);
        release(c"PF Effect Sequence Data Suite".as_ptr(), 1);
    }
    if valid { 0 } else { ae::PF_Err_BAD_CALLBACK_PARAM }
}
