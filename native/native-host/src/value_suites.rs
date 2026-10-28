//! Typed public-SDK access to the color values owned by this 8-bit host.
use after_effects_sys as ae;

unsafe extern "C" fn color(_effect: ae::PF_ProgPtr, def: *const ae::PF_ParamDef, out: *mut ae::PF_PixelFloat) -> ae::PF_Err {
    if def.is_null() || out.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let def = unsafe { &*def };
    if def.param_type != ae::PF_Param_COLOR {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let p = unsafe { def.u.cd.value };
    unsafe {
        *out = ae::PF_PixelFloat {
            alpha: f32::from(p.alpha) / 255.0,
            red: f32::from(p.red) / 255.0,
            green: f32::from(p.green) / 255.0,
            blue: f32::from(p.blue) / 255.0,
        };
    }
    0
}

pub const COLOR: ae::PF_ColorParamSuite1 = ae::PF_ColorParamSuite1 { PF_GetFloatingPointColorFromColorDef: Some(color) };

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn color_queries_return_the_current_value_and_validate_the_tag() {
        let mut def: ae::PF_ParamDef = unsafe { std::mem::zeroed() };
        let mut out: ae::PF_PixelFloat = unsafe { std::mem::zeroed() };
        assert_ne!(unsafe { color(std::ptr::null_mut(), &def, &mut out) }, 0);
        def.param_type = ae::PF_Param_COLOR;
        def.u.cd.value = ae::PF_Pixel { alpha: 255, red: 128, green: 64, blue: 0 };
        assert_eq!(unsafe { color(std::ptr::null_mut(), &def, &mut out) }, 0);
        assert_eq!([out.alpha, out.red, out.green, out.blue], [1.0, 128.0 / 255.0, 64.0 / 255.0, 0.0]);
        assert_ne!(unsafe { color(std::ptr::null_mut(), &def, std::ptr::null_mut()) }, 0);
    }
}
