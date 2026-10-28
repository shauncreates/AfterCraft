use super::*;
use crate::tests_timeline::{animate, prop, setup};

fn select(s: &mut Session, layer: u64, path: &str, times: &[f64]) {
    s.execute("keys.select", json!({"keys": times.iter().map(|time| json!({"layer":layer,"path":path,"time":time})).collect::<Vec<_>>() })).unwrap();
}

#[test]
fn original_motion_functions_keep_endpoints_and_produce_distinct_physics() {
    let mut samples = Vec::new();
    for mode in [PhysicsMode::Spring, PhysicsMode::Elastic, PhysicsMode::Bounce] {
        let curve = Physics { mode, ..Physics::default() };
        assert_eq!(curve.sample(0.0), 0.0);
        assert_eq!(curve.sample(1.0), 1.0);
        assert_eq!(curve.sample(f64::NAN), 0.0);
        let values: Vec<_> = (0..=1000).map(|i| curve.sample(i as f64 / 1000.0)).collect();
        assert!(values.iter().all(|v| v.is_finite()));
        if mode == PhysicsMode::Bounce {
            assert!(values.iter().all(|v| (0.0..=1.0).contains(v)));
            assert!(values.windows(2).any(|p| p[1] < p[0]), "bounce must contain actual rebounds");
        } else {
            assert!(values.iter().any(|v| *v > 1.05), "spring and elastic must overshoot");
        }
        samples.push(values);
    }
    assert_ne!(samples[0], samples[1], "elastic and spring are different functions");
    for time in [0.0, 0.1, 0.5, 0.8, 1.0] {
        assert_eq!(Physics { amplitude: 0.0, ..Physics::default() }.sample(time), time);
    }
}

#[test]
fn physics_validation_bounds_even_extreme_preview_inputs() {
    for (frequency, damping, amplitude) in [(f64::NAN, 6.0, 1.0), (f64::INFINITY, 6.0, 1.0), (0.0, 6.0, 1.0), (3.0, 0.0, 1.0), (3.0, 6.0, 4.0)] {
        let settings = Physics { frequency, damping, amplitude, ..Physics::default() };
        assert!(settings.validate().is_err());
        assert_eq!(settings.sample(0.375), 0.375);
    }
    for frequency in [0.5, 12.0] {
        for damping in [0.1, 20.0] {
            for mode in [PhysicsMode::Spring, PhysicsMode::Elastic, PhysicsMode::Bounce] {
                let settings = Physics { frequency, damping, mode, amplitude: 3.0 };
                assert!(settings.validate().is_ok());
                assert!((0..=1000).all(|i| settings.sample(i as f64 / 1000.0).is_finite()));
            }
        }
    }
}

#[test]
fn physics_baking_evaluates_the_motion_at_inserted_times_and_undoes_exactly() {
    for (from, to) in [(20.0, 200.0), (200.0, 20.0)] {
        let (mut s, layer) = setup();
        animate(&mut s, layer, "transform/rotation", &[(0.0, json!(from)), (1.0, json!(to)), (2.0, json!(500.0))]);
        select(&mut s, layer, "transform/rotation", &[0.0, 1.0]);
        let before = prop(&s, layer, "transform/rotation");
        let undo = s.history.undo.len();
        let result = s.execute(APPLY, json!({"mode":"spring","density":2})).unwrap();
        assert_eq!(result["segments"], 1);
        assert!(result["generatedKeys"].as_u64().unwrap() >= 47);
        let after = prop(&s, layer, "transform/rotation");
        assert_eq!(after.keys.first().unwrap().time, Tick::ZERO);
        assert_eq!(after.keys.last().unwrap(), before.keys.last().unwrap());
        let settings = Physics::default();
        for key in after.keys.iter().filter(|k| k.time > Tick::ZERO && k.time < Tick::from_seconds_f64(1.0)) {
            let expected = from + (to - from) * settings.sample(key.time.seconds());
            let actual = effectcraft_keyframe::evaluate(&after.keys, key.time, false).unwrap().as_f64();
            assert!((actual - expected).abs() < 1e-9);
        }
        let boundary = after.keys.iter().find(|k| k.time == Tick::from_seconds_f64(1.0)).unwrap();
        assert_eq!(boundary.value, before.keys[1].value);
        assert_eq!(boundary.out_interp, before.keys[1].out_interp);
        assert_eq!(s.history.undo.len(), undo + 1);
        s.execute("edit.undo", json!({})).unwrap();
        assert_eq!(prop(&s, layer, "transform/rotation"), before);
        s.execute("edit.redo", json!({})).unwrap();
        assert_eq!(prop(&s, layer, "transform/rotation"), after);
    }
}

#[test]
fn motion_reapplication_replaces_generated_keys_and_coalesces_one_undo_step() {
    let (mut s, layer) = setup();
    animate(&mut s, layer, "transform/rotation", &[(0.0, json!(0)), (1.0, json!(100))]);
    select(&mut s, layer, "transform/rotation", &[0.0, 1.0]);
    let before = prop(&s, layer, "transform/rotation");
    let undo = s.history.undo.len();
    let first = s.execute(APPLY, json!({"mode":"spring","density":2})).unwrap();
    let n = prop(&s, layer, "transform/rotation").keys.len();
    let second = s.execute(APPLY, json!({"mode":"elastic","amplitude":2.0,"density":2,"token":first["token"]})).unwrap();
    assert_eq!(first["token"], second["token"]);
    assert_eq!(prop(&s, layer, "transform/rotation").keys.len(), n, "live editing must not accumulate keys");
    assert_eq!(s.history.undo.len(), undo + 1, "one motion recipe is one undo step");
    s.execute("edit.undo", json!({})).unwrap();
    assert_eq!(prop(&s, layer, "transform/rotation"), before);
    let snapshot = s.project.clone();
    assert!(s.execute(APPLY, json!({"mode":"bounce","token":first["token"]})).unwrap_err().to_string().contains("expired"));
    assert_eq!(s.project, snapshot);
}

#[test]
fn vector_components_follow_one_progress_without_changing_constant_dimensions() {
    let (mut s, layer) = setup();
    animate(&mut s, layer, "transform/scale", &[(0.0, json!([50, 300, 100])), (1.0, json!([150, 100, 100]))]);
    select(&mut s, layer, "transform/scale", &[0.0, 1.0]);
    s.execute(APPLY, json!({"mode":"elastic"})).unwrap();
    let keys = prop(&s, layer, "transform/scale").keys;
    let settings = Physics { mode: PhysicsMode::Elastic, ..Physics::default() };
    for key in keys {
        let p = settings.sample(key.time.seconds());
        let value = key.value.components();
        assert!((value[0] - (50.0 + 100.0 * p)).abs() < 1e-10);
        assert!((value[1] - (300.0 - 200.0 * p)).abs() < 1e-10);
        assert_eq!(value[2], 100.0);
    }
}

#[test]
fn baking_density_tracks_fractional_composition_rate_and_layer_stretch() {
    for stretch in [100.0, 200.0, -200.0] {
        let (mut s, layer) = setup();
        animate(&mut s, layer, "transform/rotation", &[(0.0, json!(0)), (1.0, json!(100))]);
        let cid = s.active_comp_id().unwrap();
        let comp = Arc::make_mut(&mut s.project).comp_mut(cid).unwrap();
        comp.frame_rate = effectcraft_time::FrameRate::FPS_29_97;
        comp.layer_mut(LayerId(layer)).unwrap().stretch = stretch;
        select(&mut s, layer, "transform/rotation", &[0.0, 1.0]);
        let result = s.execute(APPLY, json!({"mode":"bounce","density":2})).unwrap();
        assert_eq!(result["generatedKeys"], if stretch == 100.0 { 59 } else { 119 });
        let keys = prop(&s, layer, "transform/rotation").keys;
        assert!(keys.windows(2).all(|p| p[0].time < p[1].time));
        assert_eq!(keys.first().unwrap().time, Tick::ZERO);
        assert_eq!(keys.last().unwrap().time, Tick::from_seconds_f64(1.0));
    }
}

#[test]
fn live_token_rejects_selection_changes_and_unrelated_project_edits_without_lost_work() {
    let (mut s, layer) = setup();
    animate(&mut s, layer, "transform/rotation", &[(0.0, json!(0)), (1.0, json!(100)), (2.0, json!(200))]);
    select(&mut s, layer, "transform/rotation", &[0.0, 1.0]);
    let first = s.execute(APPLY, json!({"mode":"spring"})).unwrap();
    select(&mut s, layer, "transform/rotation", &[1.0, 2.0]);
    let project = s.project.clone();
    assert!(s.execute(APPLY, json!({"mode":"bounce","token":first["token"]})).is_err());
    assert_eq!(s.project, project);
    select(&mut s, layer, "transform/rotation", &[0.0, 1.0]);
    s.execute("prop.set", json!({"layer":layer,"path":"transform/opacity","value":60})).unwrap();
    let project = s.project.clone();
    let undo = s.history.undo.len();
    assert!(s.execute(APPLY, json!({"mode":"bounce","token":first["token"]})).is_err());
    assert_eq!(s.project, project);
    assert_eq!(s.history.undo.len(), undo);
}

#[test]
fn baking_preserves_unselected_auto_bezier_intervals_and_separate_dimensions() {
    let (mut s, layer) = setup();
    animate(&mut s, layer, "transform/rotation", &[(0.0, json!(0)), (1.0, json!(60)), (2.0, json!(200)), (3.0, json!(500))]);
    let cid = s.active_comp_id().unwrap();
    let pr = Arc::make_mut(&mut s.project).comp_mut(cid).unwrap().layer_mut(LayerId(layer)).unwrap().props.prop_mut("transform/rotation").unwrap();
    for key in pr.keys.iter_mut() {
        key.auto_bezier = true;
        key.in_interp = Interp::Bezier;
        key.out_interp = Interp::Bezier;
    }
    let before = pr.keys.clone();
    select(&mut s, layer, "transform/rotation", &[1.0, 2.0]);
    s.execute(APPLY, json!({"mode":"bounce"})).unwrap();
    let after = prop(&s, layer, "transform/rotation").keys;
    for time in [0.1, 0.5, 0.9, 2.1, 2.5, 2.9] {
        assert_eq!(
            effectcraft_keyframe::evaluate(&before, Tick::from_seconds_f64(time), false),
            effectcraft_keyframe::evaluate(&after, Tick::from_seconds_f64(time), false)
        );
    }
}

#[test]
fn unsafe_or_unbounded_bakes_leave_project_and_undo_unchanged() {
    let (mut s, layer) = setup();
    animate(&mut s, layer, "transform/rotation", &[(0.0, json!(0)), (3600.0, json!(100))]);
    select(&mut s, layer, "transform/rotation", &[0.0, 3600.0]);
    let before = s.project.clone();
    let undo = s.history.undo.len();
    for params in [json!({"density":8}), json!({"mode":"noise"}), json!({"density":1.5}), json!({"frequency":1e100}), json!({"token":-1})] {
        assert!(s.execute(APPLY, params).is_err());
        assert_eq!(s.project, before);
        assert_eq!(s.history.undo.len(), undo);
    }
    animate(&mut s, layer, "transform/position", &[(0.0, json!([0, 0])), (1.0, json!([100, 100]))]);
    select(&mut s, layer, "transform/position", &[0.0, 1.0]);
    let before = s.project.clone();
    assert!(s.execute(APPLY, json!({})).unwrap_err().to_string().contains("Separate Position"));
    assert_eq!(s.project, before);
}

#[test]
fn physics_library_persists_original_settings_independently_of_project_undo() {
    let (mut s, _) = setup();
    let config = Arc::new(crate::config::MemoryConfig::default());
    s.config = Some(config.clone());
    let undo = s.history.undo.len();
    let physics = Physics { mode: PhysicsMode::Bounce, damping: 2.5, ..Physics::default() };
    s.execute("current.savePhysicsPreset", json!({"name":"My rebound","physics":physics})).unwrap();
    let library = s.execute("current.physicsPresets", json!({})).unwrap();
    assert_eq!(library["builtins"].as_array().unwrap().len(), 3);
    assert_eq!(library["user"][0]["physics"], json!(physics));
    let mut reopened = Session { config: Some(config), ..Session::default() };
    assert_eq!(reopened.execute("current.physicsPresets", json!({})).unwrap()["user"], library["user"]);
    s.execute("current.deletePhysicsPreset", json!({"name":"My rebound"})).unwrap();
    assert_eq!(s.execute("current.physicsPresets", json!({})).unwrap()["user"], json!([]));
    assert!(s.execute("current.savePhysicsPreset", json!({"name":"Invalid","physics":{"amplitude":100}})).is_err());
    assert_eq!(s.history.undo.len(), undo);
}
