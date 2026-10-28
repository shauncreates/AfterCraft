use super::*;
use crate::tests_timeline::{animate, prop, setup};

fn select(s: &mut Session, layer: u64, path: &str, times: &[f64]) {
    let keys: Vec<_> = times.iter().map(|time| json!({"layer": layer, "path": path, "time": time})).collect();
    s.execute("keys.select", json!({"keys": keys})).unwrap();
}

fn cubic(a: f64, b: f64, u: f64) -> f64 {
    3.0 * (1.0 - u).powi(2) * u * a + 3.0 * (1.0 - u) * u * u * b + u.powi(3)
}

#[test]
fn normalized_curve_renders_ascending_descending_and_overshooting_values_and_undoes() {
    for (from, to) in [(10.0, 210.0), (210.0, 10.0)] {
        let (mut s, layer) = setup();
        animate(&mut s, layer, "transform/rotation", &[(0.0, json!(from)), (2.0, json!(to))]);
        select(&mut s, layer, "transform/rotation", &[0.0, 2.0]);
        let before = prop(&s, layer, "transform/rotation");
        let curve = [0.35, -0.45, 0.65, 1.45];
        let response = s.execute(APPLY, json!({"curve": curve})).unwrap();
        assert_eq!(response["segments"], 1);
        let keys = prop(&s, layer, "transform/rotation").keys;
        for u in [0.1, 0.3, 0.6, 0.9] {
            let x = cubic(curve[0], curve[2], u);
            let progress = cubic(curve[1], curve[3], u);
            let actual = effectcraft_keyframe::evaluate(&keys, Tick::from_seconds_f64(2.0 * x), false).unwrap().components()[0];
            assert!((actual - (from + (to - from) * progress)).abs() < 1e-6, "{actual}, {progress}");
        }
        let read = s.execute("keys.currentRead", json!({})).unwrap();
        let actual: [f64; 4] = serde_json::from_value(read["curve"].clone()).unwrap();
        assert!(curve.iter().zip(actual).all(|(a, b)| (a - b).abs() < 1e-10));
        s.execute("edit.undo", json!({})).unwrap();
        assert_eq!(prop(&s, layer, "transform/rotation"), before);
        s.execute("edit.redo", json!({})).unwrap();
        assert_eq!(prop(&s, layer, "transform/rotation").keys, keys);
    }
}

#[test]
fn current_changes_only_selected_adjacent_intervals_without_bridging_other_keys() {
    let (mut s, layer) = setup();
    animate(&mut s, layer, "transform/rotation", &[(0.0, json!(0)), (1.0, json!(100)), (2.0, json!(0)), (3.0, json!(100))]);
    select(&mut s, layer, "transform/rotation", &[0.0, 1.0, 3.0]);
    let before = prop(&s, layer, "transform/rotation").keys;
    let reply = s.execute(APPLY, json!({"curve": Curve::DEFAULT.0})).unwrap();
    assert_eq!(reply["segments"], 1);
    let after = prop(&s, layer, "transform/rotation").keys;
    assert_eq!(after[0].in_interp, before[0].in_interp);
    assert_eq!(after[1].out_interp, before[1].out_interp);
    assert_eq!(&after[2..], &before[2..]);
    for (a, b) in before.iter().zip(&after) {
        assert_eq!((&a.value, a.time), (&b.value, b.time));
    }
}

#[test]
fn applying_next_interval_preserves_the_untouched_auto_bezier_side() {
    let (mut s, layer) = setup();
    animate(&mut s, layer, "transform/rotation", &[(0.0, json!(0)), (1.0, json!(60)), (2.0, json!(200))]);
    let cid = s.active_comp_id().unwrap();
    let pr = std::sync::Arc::make_mut(&mut s.project).comp_mut(cid).unwrap().layer_mut(LayerId(layer)).unwrap().props.prop_mut("transform/rotation").unwrap();
    pr.keys[1].auto_bezier = true;
    pr.keys[1].in_interp = Interp::Bezier;
    pr.keys[1].out_interp = Interp::Bezier;
    let before = pr.keys.clone();
    select(&mut s, layer, "transform/rotation", &[1.0, 2.0]);
    s.execute(APPLY, json!({"curve": [0.7, 0.0, 0.3, 1.0]})).unwrap();
    let after = prop(&s, layer, "transform/rotation").keys;
    for time in [0.1, 0.4, 0.7, 0.9] {
        assert_eq!(
            effectcraft_keyframe::evaluate(&before, Tick::from_seconds_f64(time), false),
            effectcraft_keyframe::evaluate(&after, Tick::from_seconds_f64(time), false)
        );
    }
}

#[test]
fn spatial_easing_preserves_motion_path_and_uses_distance_while_overshoot_reports_limitation() {
    let (mut s, layer) = setup();
    animate(&mut s, layer, "transform/position", &[(0.0, json!([10, 20])), (2.0, json!([110, 220]))]);
    select(&mut s, layer, "transform/position", &[0.0, 2.0]);
    let before = prop(&s, layer, "transform/position").keys;
    s.execute(APPLY, json!({"curve": [0.6, 0.0, 0.8, 0.4]})).unwrap();
    let after = prop(&s, layer, "transform/position").keys;
    for (a, b) in before.iter().zip(&after) {
        assert_eq!(
            (a.time, a.value.clone(), a.spatial_in, a.spatial_out, a.spatial_auto),
            (b.time, b.value.clone(), b.spatial_in, b.spatial_out, b.spatial_auto)
        );
    }
    let u = 0.4;
    let x = cubic(0.6, 0.8, u);
    let f = cubic(0.0, 0.4, u);
    let value = effectcraft_keyframe::evaluate(&after, Tick::from_seconds_f64(x * 2.0), true).unwrap().components();
    assert!((value[0] - (10.0 + 100.0 * f)).abs() < 1e-6);
    let err = s.execute(APPLY, json!({"curve": [0.3, -0.4, 0.7, 1.0]})).unwrap_err();
    assert!(err.to_string().contains("Separate Position"));
    assert_eq!(prop(&s, layer, "transform/position").keys, after);
}

#[test]
fn bad_curves_nonadjacent_selection_and_locked_layers_leave_project_and_history_unchanged() {
    let (mut s, layer) = setup();
    animate(&mut s, layer, "transform/rotation", &[(0.0, json!(0)), (1.0, json!(100)), (2.0, json!(0))]);
    select(&mut s, layer, "transform/rotation", &[0.0, 2.0]);
    let before = s.project.clone();
    let undo = s.history.undo.len();
    assert!(s.execute(APPLY, json!({"curve": Curve::DEFAULT.0})).is_err());
    select(&mut s, layer, "transform/rotation", &[0.0, 1.0]);
    for curve in [json!([0.3, 0, 0.7]), json!([0.3, 11, 0.7, 1])] {
        assert!(s.execute(APPLY, json!({"curve": curve})).is_err());
    }
    assert_eq!(s.project, before);
    assert_eq!(s.history.undo.len(), undo);
    let cid = s.active_comp_id().unwrap();
    std::sync::Arc::make_mut(&mut s.project).comp_mut(cid).unwrap().layer_mut(LayerId(layer)).unwrap().switches.locked = true;
    assert!(s.execute(APPLY, json!({"curve": Curve::DEFAULT.0})).unwrap_err().to_string().contains("Unlock"));
    assert_eq!(s.history.undo.len(), undo);
}

#[test]
fn current_library_is_persistent_and_separate_from_project_undo() {
    let (mut s, _) = setup();
    let config = std::sync::Arc::new(crate::config::MemoryConfig::default());
    s.config = Some(config.clone());
    let undo = s.history.undo.len();
    s.execute("current.savePreset", json!({"name": "My curve", "curve": Curve::DEFAULT.0})).unwrap();
    let presets = s.execute("current.presets", json!({})).unwrap();
    assert_eq!(presets["builtins"].as_array().unwrap().len(), 7);
    assert_eq!(presets["user"][0]["name"], "My curve");
    let mut reopened = Session { config: Some(config), ..Session::default() };
    assert_eq!(reopened.execute("current.presets", json!({})).unwrap()["user"], presets["user"]);
    s.execute("current.deletePreset", json!({"name": "My curve"})).unwrap();
    assert_eq!(s.execute("current.presets", json!({})).unwrap()["user"], json!([]));
    assert_eq!(s.history.undo.len(), undo);
}

#[test]
fn vertical_endpoint_apply_is_approximated_with_notice_without_changing_saved_graph_data() {
    for curve in [[0.0, 0.4, 1.0, 0.6], [0.2, 0.0, 1.0, 0.7]] {
        let (mut s, layer) = setup();
        s.config = Some(std::sync::Arc::new(crate::config::MemoryConfig::default()));
        s.execute("current.createGraphList", json!({"name":"Original data"})).unwrap();
        s.execute("current.saveGraph", json!({"list":"Original data","name":"Vertical","config":{"curve":curve}})).unwrap();
        let saved = s.execute("current.exportGraphLists", json!({})).unwrap();
        animate(&mut s, layer, "transform/rotation", &[(0.0, json!(0)), (1.0, json!(100))]);
        select(&mut s, layer, "transform/rotation", &[0.0, 1.0]);
        let before = prop(&s, layer, "transform/rotation");
        let undo = s.history.undo.len();
        let result = s.execute(APPLY, json!({"curve":curve})).unwrap();
        assert!(!result["notices"].as_array().unwrap().is_empty());
        let applied: [f64; 4] = serde_json::from_value(result["appliedCurve"].clone()).unwrap();
        assert_eq!(applied[2], 0.999);
        if curve[0] == 0.0 {
            assert_eq!(applied[0], 0.001);
        }
        let after = prop(&s, layer, "transform/rotation");
        assert_eq!(after.keys.len(), 2);
        assert!(after.keys.iter().zip(&before.keys).all(|(a, b)| a.time == b.time && a.value == b.value));
        assert_eq!(s.execute("current.exportGraphLists", json!({})).unwrap(), saved, "applying cannot mutate stored curve coordinates");
        assert_eq!(s.history.undo.len(), undo + 1);
        s.execute("edit.undo", json!({})).unwrap();
        assert_eq!(prop(&s, layer, "transform/rotation"), before);
    }
}
