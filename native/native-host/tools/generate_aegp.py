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
    "AEGP_RegisterWithAEGP": "register",
}
out = ['//! Partial AEGP tables. Generated from public ABI signatures by tools/generate_aegp.py.\n',
       'use after_effects_sys::*;\nuse crate::{event, aegp_context};\nuse serde_json::json;\n']
for struct, table in [("AEGP_PFInterfaceSuite1", "INTERFACE"), ("AEGP_LayerSuite5", "LAYER"),
                      ("AEGP_StreamSuite1", "STREAM"), ("AEGP_CameraSuite2", "CAMERA"),
                      ("AEGP_UtilitySuite2", "UTILITY")]:
    body = re.search(r'pub struct ' + struct + r' \{(.*?)\n\}', source, re.S).group(1)
    fields = re.findall(r'pub (\w+):\s*::std::option::Option<\s*unsafe extern "C" fn\((.*?)\) -> A_Err,?\s*>', body, re.S)
    table_fields = []
    for field, args in fields:
        if field in supported:
            table_fields.append(f'{field}: Some(aegp_context::{supported[field]}),')
            continue
        args = re.sub(r'\b(\w+):(?!:)', r'_\1:', args)
        name = 'cb_' + field
        out.append(f'#[allow(non_snake_case)]\nunsafe extern "C" fn {name}({args}) -> A_Err {{\n'
                   f' event(json!({{"stage":"unsupported_callback","name":"{field}"}}));\n'
                   ' PF_Err_INVALID_CALLBACK\n}\n')
        table_fields.append(f'{field}: Some({name}),')
    out.append(f'pub static {table}: {struct} = {struct} {{\n' + '\n'.join(table_fields) + '\n};\n')
(root / 'src/aegp.rs').write_text(''.join(out), encoding='utf-8', newline='\n')
