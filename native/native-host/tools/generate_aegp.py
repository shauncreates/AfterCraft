"""Generate typed error callbacks from public after-effects-sys ABI bindings, never plugin code."""
import pathlib
import argparse
import re

root = pathlib.Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("bindings", type=pathlib.Path, help="Public after-effects-sys Windows ABI bindings")
args = parser.parse_args()
source = args.bindings.read_text(encoding="utf-8")
supported = {
    "AEGP_GetEffectLayer": "effect_layer",
    "AEGP_GetEffectCamera": "effect_camera",
    "AEGP_ConvertEffectToCompTime": "comp_time",
    "AEGP_GetLayerFlags": "layer_flags",
    "AEGP_IsLayer3D": "layer_3d",
    "AEGP_IsLayer2D": "layer_2d",
    "AEGP_GetLayerObjectType": "object_type",
    "AEGP_GetLayerMaskedBounds": "masked_bounds",
    "AEGP_ConvertLayerToCompTime": "layer_time",
    "AEGP_GetLayerToWorldXform": "layer_world",
    "AEGP_GetLayerParentComp": "parent_comp",
    "AEGP_GetNewWorkingSpaceColorProfile": "working_space",
    "copy": "@crate::worlds::copy",
    "copy_hq": "@crate::worlds::copy",
    "transfer_rect": "@crate::pixels::transfer",
    "iterate": "@crate::pixels::iterate",
    "AEGP_RegisterWithAEGP": "register",
}
out = ['//! Partial AEGP tables. Generated from public ABI signatures by tools/generate_aegp.py.\n',
       'use after_effects_sys::*;\nuse crate::{event, aegp_context};\nuse serde_json::json;\n']
for struct, table in [("AEGP_PFInterfaceSuite1", "INTERFACE"), ("AEGP_LayerSuite5", "LAYER"),
                      ("AEGP_LayerSuite7", "LAYER7"),
                      ("AEGP_ColorSettingsSuite2", "COLOR_SETTINGS"),
                      ("PF_WorldTransformSuite1", "WORLD_TRANSFORM"),
                      ("PF_Iterate8Suite1", "ITERATE8"),
                      ("AEGP_StreamSuite1", "STREAM"), ("AEGP_CameraSuite2", "CAMERA"),
                      ("AEGP_UtilitySuite2", "UTILITY")]:
    body = re.search(r'pub struct ' + struct + r' \{(.*?)\n\}', source, re.S).group(1)
    # A callback can itself take function pointers. Match the outer argument list
    # with balanced parentheses rather than stopping at an inner callback's return.
    members = list(re.finditer(r'^    pub (\w+):', body, re.M))
    fields = []
    for index, member in enumerate(members):
        stop = members[index + 1].start() if index + 1 < len(members) else len(body)
        declaration = body[member.end():stop]
        signature = re.search(r'unsafe extern "C" fn\(', declaration)
        if not signature:
            raise ValueError(f'unsupported public ABI field: {struct}.{member.group(1)}')
        start = signature.end()
        depth, end = 1, start
        while end < len(declaration) and depth:
            depth += (declaration[end] == '(') - (declaration[end] == ')')
            end += 1
        if depth or not re.match(r'\s*-> (?:A_Err|PF_Err)', declaration[end:]):
            raise ValueError(f'unsupported public ABI signature: {struct}.{member.group(1)}')
        fields.append((member.group(1), declaration[start:end - 1]))
    table_fields = []
    for field, args in fields:
        if field in supported:
            callback = supported[field]
            callback = callback[1:] if callback.startswith('@') else 'aegp_context::' + callback
            table_fields.append(f'{field}: Some({callback}),')
            continue
        args = re.sub(r'\b(\w+):(?!:)', r'_\1:', args)
        name = 'cb_' + (table + '_' if table == 'LAYER7' else '') + field
        out.append(f'#[allow(non_snake_case)]\nunsafe extern "C" fn {name}({args}) -> A_Err {{\n'
                   f' event(json!({{"stage":"unsupported_callback","name":"{field}"}}));\n'
                   ' PF_Err_INVALID_CALLBACK\n}\n')
        table_fields.append(f'{field}: Some({name}),')
    out.append(f'pub static {table}: {struct} = {struct} {{\n' + '\n'.join(table_fields) + '\n};\n')
(root / 'src/aegp.rs').write_text(''.join(out), encoding='utf-8', newline='\n')
