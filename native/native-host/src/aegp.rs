//! Partial AEGP tables. Generated from public ABI signatures by tools/generate_aegp.py.
use crate::{aegp_context, event};
use after_effects_sys::*;
use serde_json::json;
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetNewEffectForEffect(_aegp_plugin_id: AEGP_PluginID, _effect_pp_ref: PF_ProgPtr, _effect_refPH: *mut AEGP_EffectRefH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetNewEffectForEffect"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetEffectCameraMatrix(
    _effect_pp_ref: PF_ProgPtr,
    _comp_timePT: *const A_Time,
    _camera_matrixP: *mut A_Matrix4,
    _dist_to_image_planePF: *mut A_FpLong,
    _image_plane_widthPL: *mut A_short,
    _image_plane_heightPL: *mut A_short,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetEffectCameraMatrix"}));
    PF_Err_INVALID_CALLBACK
}
pub static INTERFACE: AEGP_PFInterfaceSuite1 = AEGP_PFInterfaceSuite1 {
    AEGP_GetEffectLayer: Some(aegp_context::effect_layer),
    AEGP_GetNewEffectForEffect: Some(cb_AEGP_GetNewEffectForEffect),
    AEGP_ConvertEffectToCompTime: Some(aegp_context::comp_time),
    AEGP_GetEffectCamera: Some(aegp_context::effect_camera),
    AEGP_GetEffectCameraMatrix: Some(cb_AEGP_GetEffectCameraMatrix),
};
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetCompNumLayers(_compH: AEGP_CompH, _num_layersPL: *mut A_long) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetCompNumLayers"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetCompLayerByIndex(_compH: AEGP_CompH, _layer_indexL: A_long, _layerPH: *mut AEGP_LayerH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetCompLayerByIndex"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetActiveLayer(_layerPH: *mut AEGP_LayerH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetActiveLayer"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetLayerIndex(_layerH: AEGP_LayerH, _layer_indexPL: *mut A_long) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerIndex"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetLayerSourceItem(_layerH: AEGP_LayerH, _source_itemPH: *mut AEGP_ItemH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerSourceItem"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetLayerSourceItemID(_layerH: AEGP_LayerH, _source_item_idPL: *mut A_long) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerSourceItemID"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetLayerName(_layerH: AEGP_LayerH, _layer_nameZ0: *mut A_char, _source_nameZ0: *mut A_char) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerName"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetLayerQuality(_layerH: AEGP_LayerH, _qualityP: *mut AEGP_LayerQuality) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerQuality"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_SetLayerQuality(_layerH: AEGP_LayerH, _quality: AEGP_LayerQuality) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetLayerQuality"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_SetLayerFlag(_layerH: AEGP_LayerH, _single_flag: AEGP_LayerFlags, _valueB: A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetLayerFlag"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_IsLayerVideoReallyOn(_layerH: AEGP_LayerH, _onPB: *mut A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_IsLayerVideoReallyOn"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_IsLayerAudioReallyOn(_layerH: AEGP_LayerH, _onPB: *mut A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_IsLayerAudioReallyOn"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetLayerCurrentTime(_layerH: AEGP_LayerH, _time_mode: AEGP_LTimeMode, _curr_timePT: *mut A_Time) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerCurrentTime"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetLayerInPoint(_layerH: AEGP_LayerH, _time_mode: AEGP_LTimeMode, _in_pointPT: *mut A_Time) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerInPoint"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetLayerDuration(_layerH: AEGP_LayerH, _time_mode: AEGP_LTimeMode, _durationPT: *mut A_Time) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerDuration"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_SetLayerInPointAndDuration(
    _layerH: AEGP_LayerH,
    _time_mode: AEGP_LTimeMode,
    _in_pointPT: *const A_Time,
    _durationPT: *const A_Time,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetLayerInPointAndDuration"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetLayerOffset(_layerH: AEGP_LayerH, _offsetPT: *mut A_Time) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerOffset"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_SetLayerOffset(_layerH: AEGP_LayerH, _offsetPT: *const A_Time) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetLayerOffset"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetLayerStretch(_layerH: AEGP_LayerH, _stretchPRt: *mut A_Ratio) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerStretch"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_SetLayerStretch(_layerH: AEGP_LayerH, _stretchPRt: *const A_Ratio) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetLayerStretch"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetLayerTransferMode(_layerH: AEGP_LayerH, _transfer_modeP: *mut AEGP_LayerTransferMode) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerTransferMode"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_SetLayerTransferMode(_layerH: AEGP_LayerH, _transfer_modeP: *const AEGP_LayerTransferMode) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetLayerTransferMode"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_IsAddLayerValid(_item_to_addH: AEGP_ItemH, _into_compH: AEGP_CompH, _validPB: *mut A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_IsAddLayerValid"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_AddLayer(_item_to_addH: AEGP_ItemH, _into_compH: AEGP_CompH, _added_layerPH0: *mut AEGP_LayerH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_AddLayer"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_ReorderLayer(_layerH: AEGP_LayerH, _layer_indexL: A_long) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_ReorderLayer"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_IsVideoActive(_layerH: AEGP_LayerH, _time_mode: AEGP_LTimeMode, _timePT: *const A_Time, _is_activePB: *mut A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_IsVideoActive"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_IsLayerUsedAsTrackMatte(_layerH: AEGP_LayerH, _fill_must_be_activeB: A_Boolean, _is_track_mattePB: *mut A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_IsLayerUsedAsTrackMatte"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_DoesLayerHaveTrackMatte(_layerH: AEGP_LayerH, _has_track_mattePB: *mut A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_DoesLayerHaveTrackMatte"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_ConvertCompToLayerTime(_layerH: AEGP_LayerH, _comp_timePT: *const A_Time, _layer_timePT: *mut A_Time) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_ConvertCompToLayerTime"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetLayerDancingRandValue(_layerH: AEGP_LayerH, _comp_timePT: *const A_Time, _rand_valuePL: *mut A_long) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerDancingRandValue"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetLayerID(_layerH: AEGP_LayerH, _id_valP: *mut AEGP_LayerIDVal) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerID"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetLayerToWorldXformFromView(
    _aegp_layerH: AEGP_LayerH,
    _view_timeP: *const A_Time,
    _comp_timeP: *const A_Time,
    _transform: *mut A_Matrix4,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerToWorldXformFromView"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_SetLayerName(_aegp_layerH: AEGP_LayerH, _new_nameZ: *const A_char) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetLayerName"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetLayerParent(_layerH: AEGP_LayerH, _parent_layerPH: *mut AEGP_LayerH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerParent"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_SetLayerParent(_layerH: AEGP_LayerH, _parent_layerH0: AEGP_LayerH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetLayerParent"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_DeleteLayer(_layerH: AEGP_LayerH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_DeleteLayer"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_DuplicateLayer(_orig_layerH: AEGP_LayerH, _duplicate_layerPH: *mut AEGP_LayerH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_DuplicateLayer"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetLayerFromLayerID(_parent_compH: AEGP_CompH, _id: AEGP_LayerIDVal, _layerPH: *mut AEGP_LayerH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerFromLayerID"}));
    PF_Err_INVALID_CALLBACK
}
pub static LAYER: AEGP_LayerSuite5 = AEGP_LayerSuite5 {
    AEGP_GetCompNumLayers: Some(cb_AEGP_GetCompNumLayers),
    AEGP_GetCompLayerByIndex: Some(cb_AEGP_GetCompLayerByIndex),
    AEGP_GetActiveLayer: Some(cb_AEGP_GetActiveLayer),
    AEGP_GetLayerIndex: Some(cb_AEGP_GetLayerIndex),
    AEGP_GetLayerSourceItem: Some(cb_AEGP_GetLayerSourceItem),
    AEGP_GetLayerSourceItemID: Some(cb_AEGP_GetLayerSourceItemID),
    AEGP_GetLayerParentComp: Some(aegp_context::parent_comp),
    AEGP_GetLayerName: Some(cb_AEGP_GetLayerName),
    AEGP_GetLayerQuality: Some(cb_AEGP_GetLayerQuality),
    AEGP_SetLayerQuality: Some(cb_AEGP_SetLayerQuality),
    AEGP_GetLayerFlags: Some(aegp_context::layer_flags),
    AEGP_SetLayerFlag: Some(cb_AEGP_SetLayerFlag),
    AEGP_IsLayerVideoReallyOn: Some(cb_AEGP_IsLayerVideoReallyOn),
    AEGP_IsLayerAudioReallyOn: Some(cb_AEGP_IsLayerAudioReallyOn),
    AEGP_GetLayerCurrentTime: Some(cb_AEGP_GetLayerCurrentTime),
    AEGP_GetLayerInPoint: Some(cb_AEGP_GetLayerInPoint),
    AEGP_GetLayerDuration: Some(cb_AEGP_GetLayerDuration),
    AEGP_SetLayerInPointAndDuration: Some(cb_AEGP_SetLayerInPointAndDuration),
    AEGP_GetLayerOffset: Some(cb_AEGP_GetLayerOffset),
    AEGP_SetLayerOffset: Some(cb_AEGP_SetLayerOffset),
    AEGP_GetLayerStretch: Some(cb_AEGP_GetLayerStretch),
    AEGP_SetLayerStretch: Some(cb_AEGP_SetLayerStretch),
    AEGP_GetLayerTransferMode: Some(cb_AEGP_GetLayerTransferMode),
    AEGP_SetLayerTransferMode: Some(cb_AEGP_SetLayerTransferMode),
    AEGP_IsAddLayerValid: Some(cb_AEGP_IsAddLayerValid),
    AEGP_AddLayer: Some(cb_AEGP_AddLayer),
    AEGP_ReorderLayer: Some(cb_AEGP_ReorderLayer),
    AEGP_GetLayerMaskedBounds: Some(aegp_context::masked_bounds),
    AEGP_GetLayerObjectType: Some(aegp_context::object_type),
    AEGP_IsLayer3D: Some(aegp_context::layer_3d),
    AEGP_IsLayer2D: Some(aegp_context::layer_2d),
    AEGP_IsVideoActive: Some(cb_AEGP_IsVideoActive),
    AEGP_IsLayerUsedAsTrackMatte: Some(cb_AEGP_IsLayerUsedAsTrackMatte),
    AEGP_DoesLayerHaveTrackMatte: Some(cb_AEGP_DoesLayerHaveTrackMatte),
    AEGP_ConvertCompToLayerTime: Some(cb_AEGP_ConvertCompToLayerTime),
    AEGP_ConvertLayerToCompTime: Some(aegp_context::layer_time),
    AEGP_GetLayerDancingRandValue: Some(cb_AEGP_GetLayerDancingRandValue),
    AEGP_GetLayerID: Some(cb_AEGP_GetLayerID),
    AEGP_GetLayerToWorldXform: Some(aegp_context::layer_world),
    AEGP_GetLayerToWorldXformFromView: Some(cb_AEGP_GetLayerToWorldXformFromView),
    AEGP_SetLayerName: Some(cb_AEGP_SetLayerName),
    AEGP_GetLayerParent: Some(cb_AEGP_GetLayerParent),
    AEGP_SetLayerParent: Some(cb_AEGP_SetLayerParent),
    AEGP_DeleteLayer: Some(cb_AEGP_DeleteLayer),
    AEGP_DuplicateLayer: Some(cb_AEGP_DuplicateLayer),
    AEGP_GetLayerFromLayerID: Some(cb_AEGP_GetLayerFromLayerID),
};
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetCompNumLayers(_compH: AEGP_CompH, _num_layersPL: *mut A_long) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetCompNumLayers"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetCompLayerByIndex(_compH: AEGP_CompH, _layer_indexL: A_long, _layerPH: *mut AEGP_LayerH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetCompLayerByIndex"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetActiveLayer(_layerPH: *mut AEGP_LayerH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetActiveLayer"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetLayerIndex(_layerH: AEGP_LayerH, _layer_indexPL: *mut A_long) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerIndex"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetLayerSourceItem(_layerH: AEGP_LayerH, _source_itemPH: *mut AEGP_ItemH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerSourceItem"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetLayerSourceItemID(_layerH: AEGP_LayerH, _source_item_idPL: *mut A_long) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerSourceItemID"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetLayerName(
    _pluginID: AEGP_PluginID,
    _layerH: AEGP_LayerH,
    _utf_layer_namePH0: *mut AEGP_MemHandle,
    _utf_source_namePH0: *mut AEGP_MemHandle,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerName"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetLayerQuality(_layerH: AEGP_LayerH, _qualityP: *mut AEGP_LayerQuality) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerQuality"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_SetLayerQuality(_layerH: AEGP_LayerH, _quality: AEGP_LayerQuality) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetLayerQuality"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_SetLayerFlag(_layerH: AEGP_LayerH, _single_flag: AEGP_LayerFlags, _valueB: A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetLayerFlag"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_IsLayerVideoReallyOn(_layerH: AEGP_LayerH, _onPB: *mut A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_IsLayerVideoReallyOn"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_IsLayerAudioReallyOn(_layerH: AEGP_LayerH, _onPB: *mut A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_IsLayerAudioReallyOn"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetLayerCurrentTime(_layerH: AEGP_LayerH, _time_mode: AEGP_LTimeMode, _curr_timePT: *mut A_Time) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerCurrentTime"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetLayerInPoint(_layerH: AEGP_LayerH, _time_mode: AEGP_LTimeMode, _in_pointPT: *mut A_Time) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerInPoint"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetLayerDuration(_layerH: AEGP_LayerH, _time_mode: AEGP_LTimeMode, _durationPT: *mut A_Time) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerDuration"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_SetLayerInPointAndDuration(
    _layerH: AEGP_LayerH,
    _time_mode: AEGP_LTimeMode,
    _in_pointPT: *const A_Time,
    _durationPT: *const A_Time,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetLayerInPointAndDuration"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetLayerOffset(_layerH: AEGP_LayerH, _offsetPT: *mut A_Time) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerOffset"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_SetLayerOffset(_layerH: AEGP_LayerH, _offsetPT: *const A_Time) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetLayerOffset"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetLayerStretch(_layerH: AEGP_LayerH, _stretchPRt: *mut A_Ratio) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerStretch"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_SetLayerStretch(_layerH: AEGP_LayerH, _stretchPRt: *const A_Ratio) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetLayerStretch"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetLayerTransferMode(_layerH: AEGP_LayerH, _transfer_modeP: *mut AEGP_LayerTransferMode) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerTransferMode"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_SetLayerTransferMode(_layerH: AEGP_LayerH, _transfer_modeP: *const AEGP_LayerTransferMode) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetLayerTransferMode"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_IsAddLayerValid(_item_to_addH: AEGP_ItemH, _into_compH: AEGP_CompH, _validPB: *mut A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_IsAddLayerValid"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_AddLayer(_item_to_addH: AEGP_ItemH, _into_compH: AEGP_CompH, _added_layerPH0: *mut AEGP_LayerH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_AddLayer"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_ReorderLayer(_layerH: AEGP_LayerH, _layer_indexL: A_long) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_ReorderLayer"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_IsVideoActive(
    _layerH: AEGP_LayerH,
    _time_mode: AEGP_LTimeMode,
    _timePT: *const A_Time,
    _is_activePB: *mut A_Boolean,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_IsVideoActive"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_IsLayerUsedAsTrackMatte(
    _layerH: AEGP_LayerH,
    _fill_must_be_activeB: A_Boolean,
    _is_track_mattePB: *mut A_Boolean,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_IsLayerUsedAsTrackMatte"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_DoesLayerHaveTrackMatte(_layerH: AEGP_LayerH, _has_track_mattePB: *mut A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_DoesLayerHaveTrackMatte"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_ConvertCompToLayerTime(_layerH: AEGP_LayerH, _comp_timePT: *const A_Time, _layer_timePT: *mut A_Time) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_ConvertCompToLayerTime"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetLayerDancingRandValue(_layerH: AEGP_LayerH, _comp_timePT: *const A_Time, _rand_valuePL: *mut A_long) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerDancingRandValue"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetLayerID(_layerH: AEGP_LayerH, _id_valP: *mut AEGP_LayerIDVal) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerID"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetLayerToWorldXformFromView(
    _aegp_layerH: AEGP_LayerH,
    _view_timeP: *const A_Time,
    _comp_timeP: *const A_Time,
    _transform: *mut A_Matrix4,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerToWorldXformFromView"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_SetLayerName(_aegp_layerH: AEGP_LayerH, _new_nameZ: *const A_UTF16Char) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetLayerName"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetLayerParent(_layerH: AEGP_LayerH, _parent_layerPH: *mut AEGP_LayerH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerParent"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_SetLayerParent(_layerH: AEGP_LayerH, _parent_layerH0: AEGP_LayerH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetLayerParent"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_DeleteLayer(_layerH: AEGP_LayerH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_DeleteLayer"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_DuplicateLayer(_orig_layerH: AEGP_LayerH, _duplicate_layerPH: *mut AEGP_LayerH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_DuplicateLayer"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetLayerFromLayerID(_parent_compH: AEGP_CompH, _id: AEGP_LayerIDVal, _layerPH: *mut AEGP_LayerH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerFromLayerID"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_GetLayerLabel(_layerH: AEGP_LayerH, _labelP: *mut AEGP_LabelID) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerLabel"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_LAYER7_AEGP_SetLayerLabel(_layerH: AEGP_LayerH, _label: AEGP_LabelID) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetLayerLabel"}));
    PF_Err_INVALID_CALLBACK
}
pub static LAYER7: AEGP_LayerSuite7 = AEGP_LayerSuite7 {
    AEGP_GetCompNumLayers: Some(cb_LAYER7_AEGP_GetCompNumLayers),
    AEGP_GetCompLayerByIndex: Some(cb_LAYER7_AEGP_GetCompLayerByIndex),
    AEGP_GetActiveLayer: Some(cb_LAYER7_AEGP_GetActiveLayer),
    AEGP_GetLayerIndex: Some(cb_LAYER7_AEGP_GetLayerIndex),
    AEGP_GetLayerSourceItem: Some(cb_LAYER7_AEGP_GetLayerSourceItem),
    AEGP_GetLayerSourceItemID: Some(cb_LAYER7_AEGP_GetLayerSourceItemID),
    AEGP_GetLayerParentComp: Some(aegp_context::parent_comp),
    AEGP_GetLayerName: Some(cb_LAYER7_AEGP_GetLayerName),
    AEGP_GetLayerQuality: Some(cb_LAYER7_AEGP_GetLayerQuality),
    AEGP_SetLayerQuality: Some(cb_LAYER7_AEGP_SetLayerQuality),
    AEGP_GetLayerFlags: Some(aegp_context::layer_flags),
    AEGP_SetLayerFlag: Some(cb_LAYER7_AEGP_SetLayerFlag),
    AEGP_IsLayerVideoReallyOn: Some(cb_LAYER7_AEGP_IsLayerVideoReallyOn),
    AEGP_IsLayerAudioReallyOn: Some(cb_LAYER7_AEGP_IsLayerAudioReallyOn),
    AEGP_GetLayerCurrentTime: Some(cb_LAYER7_AEGP_GetLayerCurrentTime),
    AEGP_GetLayerInPoint: Some(cb_LAYER7_AEGP_GetLayerInPoint),
    AEGP_GetLayerDuration: Some(cb_LAYER7_AEGP_GetLayerDuration),
    AEGP_SetLayerInPointAndDuration: Some(cb_LAYER7_AEGP_SetLayerInPointAndDuration),
    AEGP_GetLayerOffset: Some(cb_LAYER7_AEGP_GetLayerOffset),
    AEGP_SetLayerOffset: Some(cb_LAYER7_AEGP_SetLayerOffset),
    AEGP_GetLayerStretch: Some(cb_LAYER7_AEGP_GetLayerStretch),
    AEGP_SetLayerStretch: Some(cb_LAYER7_AEGP_SetLayerStretch),
    AEGP_GetLayerTransferMode: Some(cb_LAYER7_AEGP_GetLayerTransferMode),
    AEGP_SetLayerTransferMode: Some(cb_LAYER7_AEGP_SetLayerTransferMode),
    AEGP_IsAddLayerValid: Some(cb_LAYER7_AEGP_IsAddLayerValid),
    AEGP_AddLayer: Some(cb_LAYER7_AEGP_AddLayer),
    AEGP_ReorderLayer: Some(cb_LAYER7_AEGP_ReorderLayer),
    AEGP_GetLayerMaskedBounds: Some(aegp_context::masked_bounds),
    AEGP_GetLayerObjectType: Some(aegp_context::object_type),
    AEGP_IsLayer3D: Some(aegp_context::layer_3d),
    AEGP_IsLayer2D: Some(aegp_context::layer_2d),
    AEGP_IsVideoActive: Some(cb_LAYER7_AEGP_IsVideoActive),
    AEGP_IsLayerUsedAsTrackMatte: Some(cb_LAYER7_AEGP_IsLayerUsedAsTrackMatte),
    AEGP_DoesLayerHaveTrackMatte: Some(cb_LAYER7_AEGP_DoesLayerHaveTrackMatte),
    AEGP_ConvertCompToLayerTime: Some(cb_LAYER7_AEGP_ConvertCompToLayerTime),
    AEGP_ConvertLayerToCompTime: Some(aegp_context::layer_time),
    AEGP_GetLayerDancingRandValue: Some(cb_LAYER7_AEGP_GetLayerDancingRandValue),
    AEGP_GetLayerID: Some(cb_LAYER7_AEGP_GetLayerID),
    AEGP_GetLayerToWorldXform: Some(aegp_context::layer_world),
    AEGP_GetLayerToWorldXformFromView: Some(cb_LAYER7_AEGP_GetLayerToWorldXformFromView),
    AEGP_SetLayerName: Some(cb_LAYER7_AEGP_SetLayerName),
    AEGP_GetLayerParent: Some(cb_LAYER7_AEGP_GetLayerParent),
    AEGP_SetLayerParent: Some(cb_LAYER7_AEGP_SetLayerParent),
    AEGP_DeleteLayer: Some(cb_LAYER7_AEGP_DeleteLayer),
    AEGP_DuplicateLayer: Some(cb_LAYER7_AEGP_DuplicateLayer),
    AEGP_GetLayerFromLayerID: Some(cb_LAYER7_AEGP_GetLayerFromLayerID),
    AEGP_GetLayerLabel: Some(cb_LAYER7_AEGP_GetLayerLabel),
    AEGP_SetLayerLabel: Some(cb_LAYER7_AEGP_SetLayerLabel),
};
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetBlendingTables(_render_contextH: PR_RenderContextH, _blending_tables: *mut PF_EffectBlendingTables) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetBlendingTables"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_DoesViewHaveColorSpaceXform(_viewP: AEGP_ItemViewP, _has_xformPB: *mut A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_DoesViewHaveColorSpaceXform"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_XformWorkingToViewColorSpace(_viewP: AEGP_ItemViewP, _srcH: AEGP_WorldH, _dstH: AEGP_WorldH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_XformWorkingToViewColorSpace"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetNewColorProfileFromICCProfile(
    _aegp_plugin_id: AEGP_PluginID,
    _icc_sizeL: A_long,
    _icc_dataPV: *const ::std::os::raw::c_void,
    _color_profilePP: *mut AEGP_ColorProfileP,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetNewColorProfileFromICCProfile"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetNewICCProfileFromColorProfile(
    _aegp_plugin_id: AEGP_PluginID,
    _color_profileP: AEGP_ConstColorProfileP,
    _icc_profilePH: *mut AEGP_MemHandle,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetNewICCProfileFromColorProfile"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetNewColorProfileDescription(
    _aegp_plugin_id: AEGP_PluginID,
    _color_profileP: AEGP_ConstColorProfileP,
    _unicode_descPH: *mut AEGP_MemHandle,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetNewColorProfileDescription"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_DisposeColorProfile(_color_profileP: AEGP_ColorProfileP) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_DisposeColorProfile"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetColorProfileApproximateGamma(_color_profileP: AEGP_ConstColorProfileP, _approx_gammaP: *mut A_FpShort) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetColorProfileApproximateGamma"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_IsRGBColorProfile(_color_profileP: AEGP_ConstColorProfileP, _is_rgbPB: *mut A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_IsRGBColorProfile"}));
    PF_Err_INVALID_CALLBACK
}
pub static COLOR_SETTINGS: AEGP_ColorSettingsSuite2 = AEGP_ColorSettingsSuite2 {
    AEGP_GetBlendingTables: Some(cb_AEGP_GetBlendingTables),
    AEGP_DoesViewHaveColorSpaceXform: Some(cb_AEGP_DoesViewHaveColorSpaceXform),
    AEGP_XformWorkingToViewColorSpace: Some(cb_AEGP_XformWorkingToViewColorSpace),
    AEGP_GetNewWorkingSpaceColorProfile: Some(aegp_context::working_space),
    AEGP_GetNewColorProfileFromICCProfile: Some(cb_AEGP_GetNewColorProfileFromICCProfile),
    AEGP_GetNewICCProfileFromColorProfile: Some(cb_AEGP_GetNewICCProfileFromColorProfile),
    AEGP_GetNewColorProfileDescription: Some(cb_AEGP_GetNewColorProfileDescription),
    AEGP_DisposeColorProfile: Some(cb_AEGP_DisposeColorProfile),
    AEGP_GetColorProfileApproximateGamma: Some(cb_AEGP_GetColorProfileApproximateGamma),
    AEGP_IsRGBColorProfile: Some(cb_AEGP_IsRGBColorProfile),
};
#[allow(non_snake_case)]
unsafe extern "C" fn cb_composite_rect(
    _effect_ref: PF_ProgPtr,
    _src_rect: *mut PF_Rect,
    _src_opacity: A_long,
    _source_wld: *mut PF_EffectWorld,
    _dest_x: A_long,
    _dest_y: A_long,
    _field_rdr: PF_Field,
    _xfer_mode: PF_XferMode,
    _dest_wld: *mut PF_EffectWorld,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"composite_rect"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_blend(
    _effect_ref: PF_ProgPtr,
    _src1: *const PF_EffectWorld,
    _src2: *const PF_EffectWorld,
    _ratio: PF_Fixed,
    _dst: *mut PF_EffectWorld,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"blend"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_convolve(
    _effect_ref: PF_ProgPtr,
    _src: *mut PF_EffectWorld,
    _area: *const PF_Rect,
    _flags: PF_KernelFlags,
    _kernel_size: A_long,
    _a_kernel: *mut ::std::os::raw::c_void,
    _r_kernel: *mut ::std::os::raw::c_void,
    _g_kernel: *mut ::std::os::raw::c_void,
    _b_kernel: *mut ::std::os::raw::c_void,
    _dst: *mut PF_EffectWorld,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"convolve"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_transform_world(
    _effect_ref: PF_ProgPtr,
    _quality: PF_Quality,
    _m_flags: PF_ModeFlags,
    _field: PF_Field,
    _src_world: *const PF_EffectWorld,
    _comp_mode: *const PF_CompositeMode,
    _mask_world0: *const PF_MaskWorld,
    _matrices: *const PF_FloatMatrix,
    _num_matrices: A_long,
    _src2dst_matrix: PF_Boolean,
    _dest_rect: *const PF_Rect,
    _dst_world: *mut PF_EffectWorld,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"transform_world"}));
    PF_Err_INVALID_CALLBACK
}
pub static WORLD_TRANSFORM: PF_WorldTransformSuite1 = PF_WorldTransformSuite1 {
    composite_rect: Some(cb_composite_rect),
    blend: Some(cb_blend),
    convolve: Some(cb_convolve),
    copy: Some(crate::worlds::copy),
    copy_hq: Some(crate::worlds::copy),
    transfer_rect: Some(crate::pixels::transfer),
    transform_world: Some(cb_transform_world),
};
#[allow(non_snake_case)]
unsafe extern "C" fn cb_iterate_origin(
    _in_data: *mut PF_InData,
    _progress_base: A_long,
    _progress_final: A_long,
    _src: *mut PF_EffectWorld,
    _area: *const PF_Rect,
    _origin: *const PF_Point,
    _refcon: *mut ::std::os::raw::c_void,
    _pix_fn: ::std::option::Option<
        unsafe extern "C" fn(_refcon: *mut ::std::os::raw::c_void, _x: A_long, _y: A_long, _in_: *mut PF_Pixel, _out: *mut PF_Pixel) -> PF_Err,
    >,
    _dst: *mut PF_EffectWorld,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"iterate_origin"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_iterate_lut(
    _in_data: *mut PF_InData,
    _progress_base: A_long,
    _progress_final: A_long,
    _src: *mut PF_EffectWorld,
    _area: *const PF_Rect,
    _a_lut0: *mut A_u_char,
    _r_lut0: *mut A_u_char,
    _g_lut0: *mut A_u_char,
    _b_lut0: *mut A_u_char,
    _dst: *mut PF_EffectWorld,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"iterate_lut"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_iterate_origin_non_clip_src(
    _in_data: *mut PF_InData,
    _progress_base: A_long,
    _progress_final: A_long,
    _src: *mut PF_EffectWorld,
    _area: *const PF_Rect,
    _origin: *const PF_Point,
    _refcon: *mut ::std::os::raw::c_void,
    _pix_fn: ::std::option::Option<
        unsafe extern "C" fn(_refcon: *mut ::std::os::raw::c_void, _x: A_long, _y: A_long, _in_: *mut PF_Pixel, _out: *mut PF_Pixel) -> PF_Err,
    >,
    _dst: *mut PF_EffectWorld,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"iterate_origin_non_clip_src"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_iterate_generic(
    _iterationsL: A_long,
    _refconPV: *mut ::std::os::raw::c_void,
    _fn_func: ::std::option::Option<
        unsafe extern "C" fn(_refconPV: *mut ::std::os::raw::c_void, _thread_indexL: A_long, _i: A_long, _iterationsL: A_long) -> PF_Err,
    >,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"iterate_generic"}));
    PF_Err_INVALID_CALLBACK
}
pub static ITERATE8: PF_Iterate8Suite1 = PF_Iterate8Suite1 {
    iterate: Some(crate::pixels::iterate),
    iterate_origin: Some(cb_iterate_origin),
    iterate_lut: Some(cb_iterate_lut),
    iterate_origin_non_clip_src: Some(cb_iterate_origin_non_clip_src),
    iterate_generic: Some(cb_iterate_generic),
};
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_IsStreamLegal(_layerH: AEGP_LayerH, _which_stream: AEGP_LayerStream, _is_legalP: *mut A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_IsStreamLegal"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_CanVaryOverTime(_streamH: AEGP_StreamRefH, _can_varyPB: *mut A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_CanVaryOverTime"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetNewLayerStream(
    _aegp_plugin_id: AEGP_PluginID,
    _layerH: AEGP_LayerH,
    _which_stream: AEGP_LayerStream,
    _streamPH: *mut AEGP_StreamRefH,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetNewLayerStream"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetEffectNumParamStreams(_effect_refH: AEGP_EffectRefH, _num_paramsPL: *mut A_long) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetEffectNumParamStreams"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetNewEffectStreamByIndex(
    _aegp_plugin_id: AEGP_PluginID,
    _effect_refH: AEGP_EffectRefH,
    _param_index: PF_ParamIndex,
    _streamPH: *mut AEGP_StreamRefH,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetNewEffectStreamByIndex"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetNewMaskStream(
    _aegp_plugin_id: AEGP_PluginID,
    _mask_refH: AEGP_MaskRefH,
    _which_stream: AEGP_MaskStream,
    _mask_streamPH: *mut AEGP_StreamRefH,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetNewMaskStream"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_DisposeStream(_streamH: AEGP_StreamRefH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_DisposeStream"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetStreamName(_streamH: AEGP_StreamRefH, _force_englishB: A_Boolean, _nameZ: *mut A_char) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetStreamName"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetStreamUnitsText(_streamH: AEGP_StreamRefH, _force_englishB: A_Boolean, _unitsZ: *mut A_char) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetStreamUnitsText"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetStreamProperties(
    _streamH: AEGP_StreamRefH,
    _flagsP: *mut AEGP_StreamFlags,
    _minP0: *mut A_FpLong,
    _maxP0: *mut A_FpLong,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetStreamProperties"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_IsStreamTimevarying(_streamH: AEGP_StreamRefH, _is_timevaryingPB: *mut A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_IsStreamTimevarying"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetStreamType(_streamH: AEGP_StreamRefH, _stream_typeP: *mut AEGP_StreamType) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetStreamType"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetNewStreamValue(
    _aegp_plugin_id: AEGP_PluginID,
    _streamH: AEGP_StreamRefH,
    _time_mode: AEGP_LTimeMode,
    _timePT: *const A_Time,
    _pre_expressionB: A_Boolean,
    _valueP: *mut AEGP_StreamValue,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetNewStreamValue"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_DisposeStreamValue(_valueP: *mut AEGP_StreamValue) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_DisposeStreamValue"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_SetStreamValue(_aegp_plugin_id: AEGP_PluginID, _streamH: AEGP_StreamRefH, _valueP: *mut AEGP_StreamValue) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetStreamValue"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetLayerStreamValue(
    _layerH: AEGP_LayerH,
    _which_stream: AEGP_LayerStream,
    _time_mode: AEGP_LTimeMode,
    _timePT: *const A_Time,
    _pre_expressionB: A_Boolean,
    _stream_valP: *mut AEGP_StreamVal,
    _stream_typeP0: *mut AEGP_StreamType,
) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetLayerStreamValue"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetExpressionState(_aegp_plugin_id: AEGP_PluginID, _streamH: AEGP_StreamRefH, _enabledPB: *mut A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetExpressionState"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_SetExpressionState(_aegp_plugin_id: AEGP_PluginID, _streamH: AEGP_StreamRefH, _enabledB: A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetExpressionState"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetExpression(_aegp_plugin_id: AEGP_PluginID, _streamH: AEGP_StreamRefH, _expressionHZ: *mut AEGP_MemHandle) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetExpression"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_SetExpression(_aegp_plugin_id: AEGP_PluginID, _streamH: AEGP_StreamRefH, _expressionP: *const A_char) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetExpression"}));
    PF_Err_INVALID_CALLBACK
}
pub static STREAM: AEGP_StreamSuite1 = AEGP_StreamSuite1 {
    AEGP_IsStreamLegal: Some(cb_AEGP_IsStreamLegal),
    AEGP_CanVaryOverTime: Some(cb_AEGP_CanVaryOverTime),
    AEGP_GetNewLayerStream: Some(cb_AEGP_GetNewLayerStream),
    AEGP_GetEffectNumParamStreams: Some(cb_AEGP_GetEffectNumParamStreams),
    AEGP_GetNewEffectStreamByIndex: Some(cb_AEGP_GetNewEffectStreamByIndex),
    AEGP_GetNewMaskStream: Some(cb_AEGP_GetNewMaskStream),
    AEGP_DisposeStream: Some(cb_AEGP_DisposeStream),
    AEGP_GetStreamName: Some(cb_AEGP_GetStreamName),
    AEGP_GetStreamUnitsText: Some(cb_AEGP_GetStreamUnitsText),
    AEGP_GetStreamProperties: Some(cb_AEGP_GetStreamProperties),
    AEGP_IsStreamTimevarying: Some(cb_AEGP_IsStreamTimevarying),
    AEGP_GetStreamType: Some(cb_AEGP_GetStreamType),
    AEGP_GetNewStreamValue: Some(cb_AEGP_GetNewStreamValue),
    AEGP_DisposeStreamValue: Some(cb_AEGP_DisposeStreamValue),
    AEGP_SetStreamValue: Some(cb_AEGP_SetStreamValue),
    AEGP_GetLayerStreamValue: Some(cb_AEGP_GetLayerStreamValue),
    AEGP_GetExpressionState: Some(cb_AEGP_GetExpressionState),
    AEGP_SetExpressionState: Some(cb_AEGP_SetExpressionState),
    AEGP_GetExpression: Some(cb_AEGP_GetExpression),
    AEGP_SetExpression: Some(cb_AEGP_SetExpression),
};
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetCamera(_render_contextH: PR_RenderContextH, _comp_timeP: *const A_Time, _camera_layerPH: *mut AEGP_LayerH) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetCamera"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetCameraType(_camera_layerH: AEGP_LayerH, _camera_typeP: *mut AEGP_CameraType) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetCameraType"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetDefaultCameraDistanceToImagePlane(_compH: AEGP_CompH, _dist_to_planePF: *mut A_FpLong) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetDefaultCameraDistanceToImagePlane"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetCameraFilmSize(_camera_layerH: AEGP_LayerH, _film_size_unitsP: *mut AEGP_FilmSizeUnits, _film_sizePF0: *mut A_FpLong) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetCameraFilmSize"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_SetCameraFilmSize(_camera_layerH: AEGP_LayerH, _film_size_units: AEGP_FilmSizeUnits, _film_sizePF0: *mut A_FpLong) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_SetCameraFilmSize"}));
    PF_Err_INVALID_CALLBACK
}
pub static CAMERA: AEGP_CameraSuite2 = AEGP_CameraSuite2 {
    AEGP_GetCamera: Some(cb_AEGP_GetCamera),
    AEGP_GetCameraType: Some(cb_AEGP_GetCameraType),
    AEGP_GetDefaultCameraDistanceToImagePlane: Some(cb_AEGP_GetDefaultCameraDistanceToImagePlane),
    AEGP_GetCameraFilmSize: Some(cb_AEGP_GetCameraFilmSize),
    AEGP_SetCameraFilmSize: Some(cb_AEGP_SetCameraFilmSize),
};
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_ReportInfo(_aegp_plugin_id: AEGP_PluginID, _info_stringZ: *const A_char) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_ReportInfo"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetDriverPluginInitFuncVersion(_major_versionPS: *mut A_short, _minor_versionPS: *mut A_short) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetDriverPluginInitFuncVersion"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetDriverImplementationVersion(_major_versionPS: *mut A_short, _minor_versionPS: *mut A_short) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetDriverImplementationVersion"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_StartQuietErrors(_err_stateP: *mut AEGP_ErrReportState) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_StartQuietErrors"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_EndQuietErrors(_report_quieted_errorsB: A_Boolean, _err_stateP: *mut AEGP_ErrReportState) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_EndQuietErrors"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_StartUndoGroup(_undo_nameZ: *const A_char) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_StartUndoGroup"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_EndUndoGroup() -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_EndUndoGroup"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_GetMainHWND(_main_hwnd: *mut ::std::os::raw::c_void) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_GetMainHWND"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_ShowHideAllFloaters(_include_tool_palB: A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_ShowHideAllFloaters"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_PaintPalGetForeColor(_fore_colorP: *mut AEGP_ColorVal) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_PaintPalGetForeColor"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_PaintPalGetBackColor(_back_colorP: *mut AEGP_ColorVal) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_PaintPalGetBackColor"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_PaintPalSetForeColor(_fore_colorP: *const AEGP_ColorVal) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_PaintPalSetForeColor"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_PaintPalSetBackColor(_back_colorP: *const AEGP_ColorVal) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_PaintPalSetBackColor"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_CharPalGetFillColor(_is_fill_color_definedPB: *mut A_Boolean, _fill_colorP: *mut AEGP_ColorVal) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_CharPalGetFillColor"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_CharPalGetStrokeColor(_is_stroke_color_definedPB: *mut A_Boolean, _stroke_colorP: *mut AEGP_ColorVal) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_CharPalGetStrokeColor"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_CharPalSetFillColor(_fill_colorP: *const AEGP_ColorVal) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_CharPalSetFillColor"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_CharPalSetStrokeColor(_stroke_colorP: *const AEGP_ColorVal) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_CharPalSetStrokeColor"}));
    PF_Err_INVALID_CALLBACK
}
#[allow(non_snake_case)]
unsafe extern "C" fn cb_AEGP_CharPalIsFillColorUIFrontmost(_is_fill_color_selectedPB: *mut A_Boolean) -> A_Err {
    event(json!({"stage":"unsupported_callback","name":"AEGP_CharPalIsFillColorUIFrontmost"}));
    PF_Err_INVALID_CALLBACK
}
pub static UTILITY: AEGP_UtilitySuite2 = AEGP_UtilitySuite2 {
    AEGP_ReportInfo: Some(cb_AEGP_ReportInfo),
    AEGP_GetDriverPluginInitFuncVersion: Some(cb_AEGP_GetDriverPluginInitFuncVersion),
    AEGP_GetDriverImplementationVersion: Some(cb_AEGP_GetDriverImplementationVersion),
    AEGP_StartQuietErrors: Some(cb_AEGP_StartQuietErrors),
    AEGP_EndQuietErrors: Some(cb_AEGP_EndQuietErrors),
    AEGP_StartUndoGroup: Some(cb_AEGP_StartUndoGroup),
    AEGP_EndUndoGroup: Some(cb_AEGP_EndUndoGroup),
    AEGP_RegisterWithAEGP: Some(aegp_context::register),
    AEGP_GetMainHWND: Some(cb_AEGP_GetMainHWND),
    AEGP_ShowHideAllFloaters: Some(cb_AEGP_ShowHideAllFloaters),
    AEGP_PaintPalGetForeColor: Some(cb_AEGP_PaintPalGetForeColor),
    AEGP_PaintPalGetBackColor: Some(cb_AEGP_PaintPalGetBackColor),
    AEGP_PaintPalSetForeColor: Some(cb_AEGP_PaintPalSetForeColor),
    AEGP_PaintPalSetBackColor: Some(cb_AEGP_PaintPalSetBackColor),
    AEGP_CharPalGetFillColor: Some(cb_AEGP_CharPalGetFillColor),
    AEGP_CharPalGetStrokeColor: Some(cb_AEGP_CharPalGetStrokeColor),
    AEGP_CharPalSetFillColor: Some(cb_AEGP_CharPalSetFillColor),
    AEGP_CharPalSetStrokeColor: Some(cb_AEGP_CharPalSetStrokeColor),
    AEGP_CharPalIsFillColorUIFrontmost: Some(cb_AEGP_CharPalIsFillColorUIFrontmost),
};
