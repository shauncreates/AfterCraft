"""Stable identities for original built-in effects and bounded popup translation.

The identities describe project metadata, not an Adobe plugin implementation.
Names alone are ambiguous: Shatter has two forces with Position/Depth/Radius/
Strength, and Key Light has several Replace Method, Saturation and Invert controls.
Unknown identities and popup choices deliberately have no replacement.
"""
import math
import re


KEYLIGHT = {
    2: "view", 3: "unpremultiplyResult", 4: "screenColour", 5: "screenGain",
    6: "screenBalance", 7: "despillBias", 8: "alphaBias", 9: "lockBiasesTogether",
    10: "screenPreblur", 12: "screenMatte/clipBlack", 13: "screenMatte/clipWhite",
    14: "screenMatte/clipRollback", 15: "screenMatte/screenShrinkGrow",
    16: "screenMatte/screenSoftness", 17: "screenMatte/screenDespotBlack",
    18: "screenMatte/screenDespotWhite", 19: "screenMatte/replaceMethod",
    20: "screenMatte/replaceColour", 23: "insideMask/insideMask",
    24: "insideMask/insideMaskSoftness", 25: "insideMask/invertInsideMask",
    26: "insideMask/insideReplaceMethod", 27: "insideMask/insideReplaceColour",
    28: "insideMask/sourceAlpha", 31: "outsideMask/outsideMask",
    32: "outsideMask/outsideMaskSoftness", 33: "outsideMask/invertOutsideMask",
    36: "foregroundColourCorrection/enableColourCorrection",
    37: "foregroundColourCorrection/saturation", 38: "foregroundColourCorrection/contrast",
    39: "foregroundColourCorrection/brightness", 41: "foregroundColourCorrection/colourSuppression",
    42: "foregroundColourCorrection/suppressionBalance", 43: "foregroundColourCorrection/suppressionAmount",
    46: "foregroundColourCorrection/colourBalanceHue", 47: "foregroundColourCorrection/colourBalanceSaturation",
    52: "edgeColourCorrection/enableEdgeColourCorrection", 53: "edgeColourCorrection/edgeHardness",
    54: "edgeColourCorrection/edgeSoftness", 55: "edgeColourCorrection/edgeGrow",
    56: "edgeColourCorrection/edgeSaturation", 57: "edgeColourCorrection/edgeContrast",
    58: "edgeColourCorrection/edgeBrightness", 60: "edgeColourCorrection/edgeColourSuppression",
    61: "edgeColourCorrection/edgeSuppressionBalance", 62: "edgeColourCorrection/edgeSuppressionAmount",
    65: "edgeColourCorrection/edgeColourBalanceHue", 66: "edgeColourCorrection/edgeColourBalanceSaturation",
    71: "sourceCrops/xMethod", 72: "sourceCrops/yMethod", 73: "sourceCrops/edgeColour",
    74: "sourceCrops/edgeColourAlpha", 75: "sourceCrops/cropLeft", 76: "sourceCrops/cropRight",
    77: "sourceCrops/cropTop", 78: "sourceCrops/cropBottom",
}

SHATTER = {
    2: "view", 4: "render", 6: "shape/pattern", 8: "shape/customShatterMap",
    10: "shape/repetitions", 12: "shape/direction", 14: "shape/origin",
    16: "shape/extrusionDepth", 18: "gradient/shatterThreshold", 20: "gradient/gradientLayer",
    22: "gradient/invertGradient", 24: "force1/force1Position", 26: "force1/force1Depth",
    28: "force1/force1Radius", 30: "force1/force1Strength", 32: "force2/force2Position",
    34: "force2/force2Depth", 36: "force2/force2Radius", 38: "force2/force2Strength",
    40: "physics/rotationSpeed", 42: "physics/tumbleAxis", 44: "physics/randomness",
    46: "physics/viscosity", 48: "physics/massVariance", 50: "physics/gravity",
    52: "physics/gravityDirection", 54: "physics/gravityInclination", 56: "textures/color",
    58: "textures/opacity", 60: "textures/frontMode", 62: "textures/frontLayer",
    64: "textures/sideMode", 66: "textures/sideLayer", 68: "textures/backMode",
    70: "textures/backLayer", 72: "cameraSystem", 74: "cameraPosition/xRotation",
    76: "cameraPosition/yRotation", 78: "cameraPosition/zRotation", 80: "cameraPosition/xyPosition",
    82: "cameraPosition/cameraZ", 84: "cameraPosition/focalLength", 86: "cameraPosition/transformOrder",
    88: "cornerPins/upperLeftCorner", 90: "cornerPins/upperRightCorner",
    92: "cornerPins/lowerLeftCorner", 94: "cornerPins/lowerRightCorner", 96: "cornerPins/autoFocalLength",
    98: "cornerPins/focalLength", 100: "lighting/lightIntensity", 102: "lighting/lightColor",
    104: "lighting/lightPosition", 106: "lighting/lightDepth", 108: "lighting/lightType",
    110: "lighting/ambientLight", 112: "material/diffuse", 114: "material/specular",
    116: "material/highlightSharpness", 144: "shape/whiteTilesFixed",
}

PARAMETERS = {
    "Keylight 906": KEYLIGHT,
    "APC Shatter": SHATTER,
    "ADBE Aud BT": {1: "bass", 2: "treble"},
    "ADBE CM WigglePosition": {1: "speed", 2: "amount"},
    "ADBE Fill": {1: "fillMask", 2: "color", 3: "horizontalFeather", 4: "verticalFeather",
                  5: "opacity", 6: "invert", 7: "allMasks"},
    "ADBE Drop Shadow": {1: "color", 2: "opacity", 3: "direction", 4: "distance", 5: "softness", 6: "shadowOnly"},
    "ADBE Tile": {1: "tileCenter", 2: "tileWidth", 3: "tileHeight", 4: "outputWidth",
                  5: "outputHeight", 6: "mirrorEdges", 7: "phase", 8: "horizontalPhaseShift"},
    "ADBE Turbulent Displace": {1: "displacement", 2: "amount", 3: "size", 4: "offset", 5: "complexity",
                                6: "evolution", 8: "evolutionOptions/cycleEvolution", 9: "evolutionOptions/cycle",
                                10: "evolutionOptions/randomSeed", 12: "pinning", 13: "resizeLayer"},
    "ADBE Geometry2": {1: "anchor", 2: "position", 3: "scaleHeight", 4: "scaleWidth", 5: "skew",
                       6: "skewAxis", 7: "rotation", 8: "opacity", 9: "useCompositionShutterAngle",
                       10: "shutterAngle", 11: "uniform", 12: "sampling"},
    "ADBE Solid Composite": {1: "sourceOpacity", 2: "color", 3: "opacity", 4: "blendingMode"},
    "ADBE Channel Combiner": {2: "useSecondLayer", 3: "sourceLayer", 5: "from", 6: "to", 7: "invert", 8: "solidAlpha"},
    "ADBE Tint": {1: "black", 2: "white", 3: "amount"},
    "ADBE Linear Wipe": {1: "completion", 2: "angle", 3: "feather"},
}


def parameter_id(effect_match_name, property_match_name):
    """Return a known engine parameter path; never infer one from an ordinal alone."""
    if not isinstance(effect_match_name, str):
        return None
    table = PARAMETERS.get(effect_match_name)
    if table is None or not isinstance(property_match_name, str):
        return None
    prefix = effect_match_name + "-"
    if not property_match_name.startswith(prefix):
        return None
    suffix = property_match_name[len(prefix):]
    if not re.fullmatch(r"[0-9]{4}", suffix):
        return None
    return table.get(int(suffix))


def scalar_scale(effect_match_name, property_match_name, source_min, source_max):
    """Return the scale for known stored scalar encodings (plus scalar_offset).

    AE project slider values need not use their displayed units. Fill's stored
    opacity can be 0..1, while Drop Shadow's can be 0..255. Restrict normalization
    to those explicit identities and verified source bounds, not the current value
    (where e.g. 1 could also mean one percent). Callers scale key values and ease
    speeds together. Unknown bounds refuse the property instead of guessing.
    """
    ranges = {
        ("ADBE Fill", "ADBE Fill-0005"): (1, 100),
        ("ADBE Drop Shadow", "ADBE Drop Shadow-0002"): (100, 255),
    }
    if not isinstance(effect_match_name, str) or not isinstance(property_match_name, str):
        return None
    if effect_match_name == "Keylight 906" and property_match_name in ("Keylight 906-0076", "Keylight 906-0078"):
        # Source crops are kept-boundary positions; the engine stores amounts cut
        # from the right/bottom. A full-frame 100 boundary therefore becomes zero.
        return -1.0 if source_min == 0 and source_max == 100 else None
    allowed = ranges.get((effect_match_name, property_match_name))
    if allowed is None:
        return 1.0
    if isinstance(source_min, bool) or isinstance(source_max, bool) or source_min != 0 or source_max not in allowed:
        return None
    return 100.0 / source_max


def scalar_offset(effect_match_name, property_match_name, source_min, source_max):
    """Value-only affine offset; never add this to a derivative/ease speed."""
    if effect_match_name == "Keylight 906" and property_match_name in ("Keylight 906-0076", "Keylight 906-0078"):
        return 100.0 if source_min == 0 and source_max == 100 else None
    return 0.0


def _option_label(option):
    label = option.get("label") if isinstance(option, dict) else option
    if not isinstance(label, str) or len(label) > 1024:
        return None
    # AE can include popup separators such as "(-"; these are not choices.
    normalized = re.sub(r"[^a-z0-9]", "", label.casefold())
    return normalized or None


def remap_enum(value, source_options, target_options):
    """Translate a selected one-based scripting popup value by its unique label.

    Compare the selected choice rather than requiring entire menus to be equal:
    the built-in Shatter supports a subset of AE's patterns and orders light types
    differently. A caller must validate every animated key before applying a curve;
    ``None`` preserves that property as unsupported instead of choosing a default.
    """
    if (not isinstance(source_options, (list, tuple)) or not isinstance(target_options, (list, tuple))
            or not source_options or not target_options or len(source_options) > 2048 or len(target_options) > 2048):
        return None
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return None
    if isinstance(value, float) and not math.isfinite(value):
        return None
    index = int(value)
    if value != index or index < 1 or index > len(source_options):
        return None
    label = _option_label(source_options[index - 1])
    if label is None:
        return None
    # Repeated labels are not an identity, even when one instance is selected.
    if sum(_option_label(option) == label for option in source_options) != 1:
        return None
    matches = [i + 1 for i, option in enumerate(target_options) if _option_label(option) == label]
    return matches[0] if len(matches) == 1 else None
