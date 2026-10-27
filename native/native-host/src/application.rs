//! Headless application services, version 6 (SDK PFAppSuite4).
use crate::event;
use after_effects_sys as ae;
use serde_json::json;
#[allow(non_snake_case)]
unsafe extern "C" fn cb_PF_AppGetBgColor(_bg_colorP: *mut ae::PF_App_Color) -> ae::PF_Err {
    event(json!({"stage":"app_callback","name":"PF_AppGetBgColor"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_PF_AppGetColor(_color_type: ae::PF_App_ColorType, _app_colorP: *mut ae::PF_App_Color) -> ae::PF_Err {
    event(json!({"stage":"app_callback","name":"PF_AppGetColor"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_PF_GetPersonalInfo(_ptiP: *mut ae::PF_AppPersonalTextInfo) -> ae::PF_Err {
    event(json!({"stage":"app_callback","name":"PF_GetPersonalInfo"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_PF_GetFontStyleSheet(
    _sheet: ae::PF_FontStyleSheet,
    _font_nameP0: *mut ae::PF_FontName,
    _font_numPS0: *mut ae::A_short,
    _sizePS0: *mut ae::A_short,
    _stylePS0: *mut ae::A_short,
) -> ae::PF_Err {
    event(json!({"stage":"app_callback","name":"PF_GetFontStyleSheet"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_PF_SetCursor(_cursor: ae::PF_CursorType) -> ae::PF_Err {
    event(json!({"stage":"app_callback","name":"PF_SetCursor"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_PF_IsRenderEngine(_render_enginePB: *mut ae::PF_Boolean) -> ae::PF_Err {
    event(json!({"stage":"app_callback","name":"PF_IsRenderEngine"}));
    if _render_enginePB.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    unsafe {
        *_render_enginePB = 1;
    }
    0
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_PF_AppColorPickerDialog(
    _dialog_titleZ0: *const ae::A_char,
    _sample_colorP: *const ae::PF_PixelFloat,
    _use_ws_to_monitor_xformB: ae::PF_Boolean,
    _new_colorP: *mut ae::PF_PixelFloat,
) -> ae::PF_Err {
    event(json!({"stage":"app_callback","name":"PF_AppColorPickerDialog"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_PF_GetMouse(_pointP: *mut ae::PF_Point) -> ae::PF_Err {
    event(json!({"stage":"app_callback","name":"PF_GetMouse"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_PF_InvalidateRect(_contextH: ae::PF_ContextH, _rectP0: *const ae::PF_Rect) -> ae::PF_Err {
    event(json!({"stage":"app_callback","name":"PF_InvalidateRect"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_PF_ConvertLocalToGlobal(_localP: *const ae::PF_Point, _globalP: *mut ae::PF_Point) -> ae::PF_Err {
    event(json!({"stage":"app_callback","name":"PF_ConvertLocalToGlobal"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_PF_GetColorAtGlobalPoint(
    _globalP: *const ae::PF_Point,
    _eyeSize: ae::A_short,
    _mode: ae::PF_EyeDropperSampleMode,
    _outColorP: *mut ae::PF_PixelFloat,
) -> ae::PF_Err {
    event(json!({"stage":"app_callback","name":"PF_GetColorAtGlobalPoint"}));
    ae::PF_Err_INVALID_CALLBACK
}
pub static SUITE: ae::PFAppSuite4 = ae::PFAppSuite4 {
    PF_AppGetBgColor: Some(cb_PF_AppGetBgColor),
    PF_AppGetColor: Some(cb_PF_AppGetColor),
    PF_GetPersonalInfo: Some(cb_PF_GetPersonalInfo),
    PF_GetFontStyleSheet: Some(cb_PF_GetFontStyleSheet),
    PF_SetCursor: Some(cb_PF_SetCursor),
    PF_IsRenderEngine: Some(cb_PF_IsRenderEngine),
    PF_AppColorPickerDialog: Some(cb_PF_AppColorPickerDialog),
    PF_GetMouse: Some(cb_PF_GetMouse),
    PF_InvalidateRect: Some(cb_PF_InvalidateRect),
    PF_ConvertLocalToGlobal: Some(cb_PF_ConvertLocalToGlobal),
    PF_GetColorAtGlobalPoint: Some(cb_PF_GetColorAtGlobalPoint),
};
