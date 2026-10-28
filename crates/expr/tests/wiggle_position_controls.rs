//! Original control-binding fixture: no Adobe preset implementation is included.
use effectcraft_color::Label;
use effectcraft_keyframe::{Keyframe, Value};
use effectcraft_project::{
    Comp, Expression, ItemKind, LayerSource, Project, Solid,
    build::{self, Ids},
};
use effectcraft_time::{FrameRate, Tick};

#[test]
fn imported_wiggle_controls_drive_an_adjacent_effect_position_expression() {
    let mut project = Project::default();
    let comp = Comp::new(64, 64, FrameRate::FPS_30, Tick::from_seconds_f64(2.0));
    let item =
        project.add_item("Original plate", Label::Red, None, ItemKind::Solid(Solid { color: [1.0, 0.0, 0.0], width: 64, height: 64, pixel_aspect: 1.0 }));
    let mut layer = build::layer(&mut project, &comp, "Plate", LayerSource::Solid { item }, (64, 64), None);
    let layer_id = layer.id;
    let mut next = project.next_id;
    for (id, name) in [("ec.distort.transform", "Original Transform"), ("ec.control.wiggleposition", "Motion Controls")] {
        let spec = effectcraft_effects::find(id).unwrap();
        let group = effectcraft_effects::instantiate(spec, &mut Ids(&mut next), name, [64.0, 64.0]);
        layer.props.sub_mut("effects").unwrap().children.push(group.into());
    }
    project.next_id = next;
    let position = "effects/#1/position";
    let expression = "wiggle(effect('Motion Controls')('Wiggle Speed (wigs/sec)'), effect('Motion Controls')('Wiggle Amount (pixels)'))";
    layer.props.prop_mut(position).unwrap().expr = Some(Expression { text: expression.into(), enabled: true });
    let cid = project.add_item("Binding", Label::Sandstone, None, ItemKind::Comp(comp.into()));
    project.comp_mut(cid).unwrap().layers.push(layer);

    let by_label = effectcraft_expr::eval_property(&project, cid, layer_id, position, 0.137).unwrap();
    let literal = effectcraft_expr::eval_standalone(&project, cid, layer_id, position, "wiggle(2, 25)", 0.137).unwrap();
    assert_eq!(by_label, literal, "the imported display labels bind to their original typed controls");
    let by_index =
        effectcraft_expr::eval_standalone(&project, cid, layer_id, position, "wiggle(effect('Motion Controls')(1), effect('Motion Controls')(2))", 0.137)
            .unwrap();
    assert_eq!(by_index, literal, "the source control order remains usable by expressions");

    let layer = project.comp_mut(cid).unwrap().layer_mut(layer_id).unwrap();
    layer.props.prop_mut("effects/#2/amount").unwrap().value = Value::Scalar(0.0);
    let base = layer.props.prop(position).unwrap().value.clone();
    assert_ne!(literal, base, "nonzero controls must actually move the evaluated position");
    assert_eq!(effectcraft_expr::eval_property(&project, cid, layer_id, position, 0.137).unwrap(), base);

    let layer = project.comp_mut(cid).unwrap().layer_mut(layer_id).unwrap();
    layer.props.prop_mut("effects/#2/amount").unwrap().keys =
        vec![Keyframe::new(Tick::ZERO, Value::Scalar(0.0)), Keyframe::new(Tick::from_seconds_f64(1.0), Value::Scalar(50.0))];
    let animated = effectcraft_expr::eval_property(&project, cid, layer_id, position, 0.5).unwrap();
    let literal = effectcraft_expr::eval_standalone(&project, cid, layer_id, position, "wiggle(2, 25)", 0.5).unwrap();
    assert_eq!(animated, literal, "animated control values are read at the evaluation time");
}
