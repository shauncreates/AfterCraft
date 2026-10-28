//! Expression Controls: parameter-only effects whose values expressions read. They pass pixels
//! through unchanged.

use effectcraft_keyframe::Value;
use effectcraft_project::ParamUi;

use crate::{Buf, EffectCtx, EffectSpec, col, num, p, popup, slider};

/// Id of the Stereo 3D Controls effect (Layer ▸ Camera ▸ Create Stereo 3D Rig).
pub const STEREO_CONTROLS: &str = "ec.control.stereo3d";
/// Two expression inputs; position changes only when an expression reads them.
pub const WIGGLE_POSITION: &str = "ec.control.wiggleposition";
/// Stereo 3D Controls ▸ Configuration: which eyes are offset from the master camera.
pub const STEREO_CONFIGURATIONS: [&str; 3] = ["Stereo Pair (Left & Right)", "Center & Right", "Center & Left"];
/// Stereo 3D Controls ▸ Convergence Of: the zero-parallax distance.
pub const STEREO_CONVERGENCE: [&str; 2] = ["Camera Point of Interest", "Camera Zoom Plane"];

fn passthrough(_: &EffectCtx, b: Buf) -> Buf {
    b
}

fn spec(id: &'static str, name: &'static str, params: Vec<crate::ParamSpec>) -> EffectSpec {
    EffectSpec { id, name, category: "Expression Controls", params, render: passthrough, gpu: true, float: true }
}

pub fn specs() -> Vec<EffectSpec> {
    vec![
        spec("ec.control.slider", "Slider Control", vec![p("slider", "Slider", num(0.0), slider(-1_000_000.0, 1_000_000.0, 0.0, 100.0, 2))]),
        spec("ec.control.angle", "Angle Control", vec![p("angle", "Angle", num(0.0), ParamUi::Angle)]),
        spec("ec.control.checkbox", "Checkbox Control", vec![p("checkbox", "Checkbox", Value::Bool(false), ParamUi::Checkbox)]),
        spec("ec.control.color", "Color Control", vec![p("color", "Color", col(1.0, 0.0, 0.0), ParamUi::Color)]),
        spec("ec.control.point", "Point Control", vec![p("point", "Point", Value::Vec2([0.5, 0.5]), ParamUi::Point)]),
        spec("ec.control.point3d", "3D Point Control", vec![p("point", "3D Point", Value::Vec3([0.0, 0.0, 0.0]), ParamUi::Point3)]),
        spec("ec.control.layer", "Layer Control", vec![p("layer", "Layer", Value::Layer(None), ParamUi::Layer)]),
        spec("ec.control.dropdown", "Dropdown Menu Control", vec![p("menu", "Menu", Value::Enum(0), popup(&["Item 1", "Item 2", "Item 3"]))]),
        spec(
            WIGGLE_POSITION,
            "Wiggle - Position",
            vec![
                p("speed", "Wiggle Speed (wigs/sec)", num(2.0), slider(0.0, 100.0, 0.0, 10.0, 2)),
                p("amount", "Wiggle Amount (pixels)", num(25.0), slider(0.0, 10_000.0, 0.0, 100.0, 2)),
            ],
        ),
        // Layer ▸ Camera ▸ Create Stereo 3D Rig: the rig's eye cameras read these (expressions).
        spec(
            STEREO_CONTROLS,
            "Stereo 3D Controls",
            vec![
                p("configuration", "Configuration", Value::Enum(0), popup(&STEREO_CONFIGURATIONS)),
                p("sceneDepth", "Stereo Scene Depth", num(3.0), slider(0.0, 100.0, 0.0, 10.0, 2)),
                p("convergence", "Enable Convergence", Value::Bool(false), ParamUi::Checkbox),
                p("convergenceOf", "Convergence Of", Value::Enum(0), popup(&STEREO_CONVERGENCE)),
                p("zOffset", "Convergence Z Offset", num(0.0), slider(-100_000.0, 100_000.0, -2000.0, 2000.0, 1)),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wiggle_controls_supply_expression_inputs_without_modifying_pixels() {
        let spec = crate::find(WIGGLE_POSITION).unwrap();
        assert_eq!(spec.params.iter().map(|p| p.id).collect::<Vec<_>>(), ["speed", "amount"]);
        assert_eq!(spec.params.iter().map(|p| p.default.as_f64()).collect::<Vec<_>>(), [2.0, 25.0]);
        let image = crate::Image::filled(8, 8, [0.2, 0.1, 0.05, 0.5]);
        let output = crate::run_fx(WIGGLE_POSITION, &[("speed", num(9.0)), ("amount", num(500.0))], image.clone(), 0.7, Default::default());
        assert_eq!(output.img, image, "a control group without a consuming expression is not a pixel shake");
    }
}
