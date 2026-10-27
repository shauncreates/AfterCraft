//! Premiere-compatible utility suite: unsupported queries return SDK callback errors.
use crate::event;
use after_effects_sys as ae;
use serde_json::json;
thread_local! {static TICKS_PER_FRAME: std::cell::Cell<i64> = const {std::cell::Cell::new(10_160_640_000)};}
pub fn set_ticks_per_frame(value: i64) {
    TICKS_PER_FRAME.with(|v| v.set(value));
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetFilterInstanceID(_effect_ref: ae::PF_ProgPtr, _outFilterInstanceID: *mut ae::A_long) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetFilterInstanceID"}));
    if _outFilterInstanceID.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    unsafe {
        *_outFilterInstanceID = 1;
    }
    0
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetMediaTimecode(
    _effect_ref: ae::PF_ProgPtr,
    _outCurrentFrame: *mut ae::A_long,
    _outTimeDisplay: *mut ae::PF_TimeDisplay,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetMediaTimecode"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetClipSpeed(_effect_ref: ae::PF_ProgPtr, _speed: *mut f64) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetClipSpeed"}));
    if _speed.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    unsafe {
        *_speed = 1.0;
    }
    0
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetClipDuration(_effect_ref: ae::PF_ProgPtr, _frameDuration: *mut ae::A_long) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetClipDuration"}));
    if _frameDuration.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    unsafe {
        *_frameDuration = 25;
    }
    0
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetClipStart(_effect_ref: ae::PF_ProgPtr, _frameDuration: *mut ae::A_long) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetClipStart"}));
    if _frameDuration.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    unsafe {
        *_frameDuration = 0;
    }
    0
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetUnscaledClipDuration(_effect_ref: ae::PF_ProgPtr, _frameDuration: *mut ae::A_long) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetUnscaledClipDuration"}));
    if _frameDuration.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    unsafe {
        *_frameDuration = 25;
    }
    0
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetUnscaledClipStart(_effect_ref: ae::PF_ProgPtr, _frameDuration: *mut ae::A_long) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetUnscaledClipStart"}));
    if _frameDuration.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    unsafe {
        *_frameDuration = 0;
    }
    0
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetTrackItemStart(_effect_ref: ae::PF_ProgPtr, _frameDuration: *mut ae::A_long) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetTrackItemStart"}));
    if _frameDuration.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    unsafe {
        *_frameDuration = 0;
    }
    0
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetMediaFieldType(_effect_ref: ae::PF_ProgPtr, _outFieldType: *mut ae::prFieldType) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetMediaFieldType"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetMediaFrameRate(_effect_ref: ae::PF_ProgPtr, _outTicksPerFrame: *mut ae::PrTime) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetMediaFrameRate"}));
    if _outTicksPerFrame.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    unsafe {
        *_outTicksPerFrame = TICKS_PER_FRAME.with(|v| v.get());
    }
    0
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetContainingTimelineID(_effect_ref: ae::PF_ProgPtr, _outTimelineID: *mut ae::PrTimelineID) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetContainingTimelineID"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetClipName(_effect_ref: ae::PF_ProgPtr, _inGetMasterClipName: ae::A_Boolean, _outSDKString: *mut ae::PrSDKString) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetClipName"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_EffectWantsCheckedOutFramesToMatchRenderPixelFormat(_effect_ref: ae::PF_ProgPtr) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"EffectWantsCheckedOutFramesToMatchRenderPixelFormat"}));
    0
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_EffectDependsOnClipName(_effect_ref: ae::PF_ProgPtr, _inDependsOnClipName: ae::A_Boolean) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"EffectDependsOnClipName"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_SetEffectInstanceName(_effect_ref: ae::PF_ProgPtr, _inSDKString: *const ae::PrSDKString) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"SetEffectInstanceName"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetFileName(_effect_ref: ae::PF_ProgPtr, _outSDKString: *mut ae::PrSDKString) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetFileName"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetOriginalClipFrameRate(_effect_ref: ae::PF_ProgPtr, _outTicksPerFrame: *mut ae::PrTime) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetOriginalClipFrameRate"}));
    if _outTicksPerFrame.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    unsafe {
        *_outTicksPerFrame = TICKS_PER_FRAME.with(|v| v.get());
    }
    0
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetSourceTrackMediaTimecode(
    _effect_ref: ae::PF_ProgPtr,
    _inLayerParamIndex: ae::csSDK_uint32,
    _inApplyTransform: bool,
    _inAddStartTimeOffset: bool,
    _outCurrentFrame: *mut ae::A_long,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetSourceTrackMediaTimecode"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetSourceTrackClipName(
    _effect_ref: ae::PF_ProgPtr,
    _inLayerParamIndex: ae::csSDK_uint32,
    _inGetMasterClipName: ae::A_Boolean,
    _outSDKString: *mut ae::PrSDKString,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetSourceTrackClipName"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetSourceTrackFileName(
    _effect_ref: ae::PF_ProgPtr,
    _inLayerParamIndex: ae::csSDK_uint32,
    _outSDKString: *mut ae::PrSDKString,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetSourceTrackFileName"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_EffectDependsOnClipName2(
    _effect_ref: ae::PF_ProgPtr,
    _inDependsOnClipName: ae::A_Boolean,
    _inLayerParamIndex: ae::csSDK_uint32,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"EffectDependsOnClipName2"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetMediaTimecode2(
    _effect_ref: ae::PF_ProgPtr,
    _inApplyTrim: bool,
    _outCurrentFrame: *mut ae::A_long,
    _outTimeDisplay: *mut ae::PF_TimeDisplay,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetMediaTimecode2"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetSourceTrackMediaTimecode2(
    _effect_ref: ae::PF_ProgPtr,
    _inLayerParamIndex: ae::csSDK_uint32,
    _inApplyTransform: bool,
    _inAddStartTimeOffset: bool,
    _inSequenceTime: ae::PrTime,
    _outCurrentFrame: *mut ae::A_long,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetSourceTrackMediaTimecode2"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetSourceTrackClipName2(
    _effect_ref: ae::PF_ProgPtr,
    _inLayerParamIndex: ae::csSDK_uint32,
    _inGetMasterClipName: ae::A_Boolean,
    _outSDKString: *mut ae::PrSDKString,
    _inSequenceTime: ae::PrTime,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetSourceTrackClipName2"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetSourceTrackFileName2(
    _effect_ref: ae::PF_ProgPtr,
    _inLayerParamIndex: ae::csSDK_uint32,
    _outSDKString: *mut ae::PrSDKString,
    _inSequenceTime: ae::PrTime,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetSourceTrackFileName2"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetCommentString(
    _inEffectRef: ae::PF_ProgPtr,
    _inSourceTrack: i32,
    _inSequenceTime: ae::PrTime,
    _outSDKString: *mut ae::PrSDKString,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetCommentString"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetLogNoteString(
    _inEffectRef: ae::PF_ProgPtr,
    _inSourceTrack: i32,
    _inSequenceTime: ae::PrTime,
    _outSDKString: *mut ae::PrSDKString,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetLogNoteString"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetCameraRollString(
    _inEffectRef: ae::PF_ProgPtr,
    _inSourceTrack: i32,
    _inSequenceTime: ae::PrTime,
    _outSDKString: *mut ae::PrSDKString,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetCameraRollString"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetClientMetadataString(
    _inEffectRef: ae::PF_ProgPtr,
    _inSourceTrack: i32,
    _inSequenceTime: ae::PrTime,
    _outSDKString: *mut ae::PrSDKString,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetClientMetadataString"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetDailyRollString(
    _inEffectRef: ae::PF_ProgPtr,
    _inSourceTrack: i32,
    _inSequenceTime: ae::PrTime,
    _outSDKString: *mut ae::PrSDKString,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetDailyRollString"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetDescriptionString(
    _inEffectRef: ae::PF_ProgPtr,
    _inSourceTrack: i32,
    _inSequenceTime: ae::PrTime,
    _outSDKString: *mut ae::PrSDKString,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetDescriptionString"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetLabRollString(
    _inEffectRef: ae::PF_ProgPtr,
    _inSourceTrack: i32,
    _inSequenceTime: ae::PrTime,
    _outSDKString: *mut ae::PrSDKString,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetLabRollString"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetSceneString(
    _inEffectRef: ae::PF_ProgPtr,
    _inSourceTrack: i32,
    _inSequenceTime: ae::PrTime,
    _outSDKString: *mut ae::PrSDKString,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetSceneString"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetShotString(
    _inEffectRef: ae::PF_ProgPtr,
    _inSourceTrack: i32,
    _inSequenceTime: ae::PrTime,
    _outSDKString: *mut ae::PrSDKString,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetShotString"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetTapeNameString(
    _inEffectRef: ae::PF_ProgPtr,
    _inSourceTrack: i32,
    _inSequenceTime: ae::PrTime,
    _outSDKString: *mut ae::PrSDKString,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetTapeNameString"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetVideoCodecString(
    _inEffectRef: ae::PF_ProgPtr,
    _inSourceTrack: i32,
    _inSequenceTime: ae::PrTime,
    _outSDKString: *mut ae::PrSDKString,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetVideoCodecString"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetGoodMetadataString(
    _inEffectRef: ae::PF_ProgPtr,
    _inSourceTrack: i32,
    _inSequenceTime: ae::PrTime,
    _outSDKString: *mut ae::PrSDKString,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetGoodMetadataString"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetSoundRollString(
    _inEffectRef: ae::PF_ProgPtr,
    _inSourceTrack: i32,
    _inSequenceTime: ae::PrTime,
    _outSDKString: *mut ae::PrSDKString,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetSoundRollString"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetSequenceTime(_inEffectRef: ae::PF_ProgPtr, _outSequenceTime: *mut ae::PrTime) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetSequenceTime"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetSoundTimecode(
    _inEffectRef: ae::PF_ProgPtr,
    _inSourceTrack: i32,
    _inSequenceTime: ae::PrTime,
    _outCurrentFrame: *mut ae::A_long,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetSoundTimecode"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetOriginalClipFrameRateForSourceTrack(
    _inEffectRef: ae::PF_ProgPtr,
    _inSourceTrack: i32,
    _outTicksPerFrame: *mut ae::PrTime,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetOriginalClipFrameRateForSourceTrack"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetMediaFrameRateForSourceTrack(
    _inEffectRef: ae::PF_ProgPtr,
    _inSourceTrack: i32,
    _inSequenceTime: ae::PrTime,
    _outTicksPerFrame: *mut ae::PrTime,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetMediaFrameRateForSourceTrack"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_GetSourceTrackMediaActualStartTime(
    _inEffectRef: ae::PF_ProgPtr,
    _inLayerParamIndex: ae::csSDK_uint32,
    _inSequenceTime: ae::PrTime,
    _outClipActualStartTime: *mut ae::PrTime,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"GetSourceTrackMediaActualStartTime"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_IsSourceTrackMediaTrimmed(
    _inEffectRef: ae::PF_ProgPtr,
    _inLayerParamIndex: ae::csSDK_uint32,
    _inSequenceTime: ae::PrTime,
    _outTrimApplied: *mut bool,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"IsSourceTrackMediaTrimmed"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_IsMediaTrimmed(_inEffectRef: ae::PF_ProgPtr, _inSequenceTime: ae::PrTime, _outTrimApplied: *mut bool) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"IsMediaTrimmed"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_IsTrackEmpty(
    _inEffectRef: ae::PF_ProgPtr,
    _inLayerParamIndex: ae::csSDK_uint32,
    _inSequenceTime: ae::PrTime,
    _outIsTrackEmpty: *mut bool,
) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"IsTrackEmpty"}));
    ae::PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_IsTrackItemEffectAppliedToSynthetic(_inEffectRef: ae::PF_ProgPtr, _outIsTrackItemEffectAppliedToSynthetic: *mut bool) -> ae::PF_Err {
    event(json!({"stage":"utility_callback","name":"IsTrackItemEffectAppliedToSynthetic"}));
    ae::PF_Err_INVALID_CALLBACK
}
pub static SUITE: ae::PF_UtilitySuite = ae::PF_UtilitySuite {
    GetFilterInstanceID: Some(cb_GetFilterInstanceID),
    GetMediaTimecode: Some(cb_GetMediaTimecode),
    GetClipSpeed: Some(cb_GetClipSpeed),
    GetClipDuration: Some(cb_GetClipDuration),
    GetClipStart: Some(cb_GetClipStart),
    GetUnscaledClipDuration: Some(cb_GetUnscaledClipDuration),
    GetUnscaledClipStart: Some(cb_GetUnscaledClipStart),
    GetTrackItemStart: Some(cb_GetTrackItemStart),
    GetMediaFieldType: Some(cb_GetMediaFieldType),
    GetMediaFrameRate: Some(cb_GetMediaFrameRate),
    GetContainingTimelineID: Some(cb_GetContainingTimelineID),
    GetClipName: Some(cb_GetClipName),
    EffectWantsCheckedOutFramesToMatchRenderPixelFormat: Some(cb_EffectWantsCheckedOutFramesToMatchRenderPixelFormat),
    EffectDependsOnClipName: Some(cb_EffectDependsOnClipName),
    SetEffectInstanceName: Some(cb_SetEffectInstanceName),
    GetFileName: Some(cb_GetFileName),
    GetOriginalClipFrameRate: Some(cb_GetOriginalClipFrameRate),
    GetSourceTrackMediaTimecode: Some(cb_GetSourceTrackMediaTimecode),
    GetSourceTrackClipName: Some(cb_GetSourceTrackClipName),
    GetSourceTrackFileName: Some(cb_GetSourceTrackFileName),
    EffectDependsOnClipName2: Some(cb_EffectDependsOnClipName2),
    GetMediaTimecode2: Some(cb_GetMediaTimecode2),
    GetSourceTrackMediaTimecode2: Some(cb_GetSourceTrackMediaTimecode2),
    GetSourceTrackClipName2: Some(cb_GetSourceTrackClipName2),
    GetSourceTrackFileName2: Some(cb_GetSourceTrackFileName2),
    GetCommentString: Some(cb_GetCommentString),
    GetLogNoteString: Some(cb_GetLogNoteString),
    GetCameraRollString: Some(cb_GetCameraRollString),
    GetClientMetadataString: Some(cb_GetClientMetadataString),
    GetDailyRollString: Some(cb_GetDailyRollString),
    GetDescriptionString: Some(cb_GetDescriptionString),
    GetLabRollString: Some(cb_GetLabRollString),
    GetSceneString: Some(cb_GetSceneString),
    GetShotString: Some(cb_GetShotString),
    GetTapeNameString: Some(cb_GetTapeNameString),
    GetVideoCodecString: Some(cb_GetVideoCodecString),
    GetGoodMetadataString: Some(cb_GetGoodMetadataString),
    GetSoundRollString: Some(cb_GetSoundRollString),
    GetSequenceTime: Some(cb_GetSequenceTime),
    GetSoundTimecode: Some(cb_GetSoundTimecode),
    GetOriginalClipFrameRateForSourceTrack: Some(cb_GetOriginalClipFrameRateForSourceTrack),
    GetMediaFrameRateForSourceTrack: Some(cb_GetMediaFrameRateForSourceTrack),
    GetSourceTrackMediaActualStartTime: Some(cb_GetSourceTrackMediaActualStartTime),
    IsSourceTrackMediaTrimmed: Some(cb_IsSourceTrackMediaTrimmed),
    IsMediaTrimmed: Some(cb_IsMediaTrimmed),
    IsTrackEmpty: Some(cb_IsTrackEmpty),
    IsTrackItemEffectAppliedToSynthetic: Some(cb_IsTrackItemEffectAppliedToSynthetic),
};
