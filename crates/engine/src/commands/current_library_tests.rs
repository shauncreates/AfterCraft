use super::*;
use crate::config::{ConfigStore, MemoryConfig};
use crate::tests_timeline::{animate, prop, setup};
use std::sync::Arc;

fn session() -> (Session, Arc<MemoryConfig>) {
    let config = Arc::new(MemoryConfig::default());
    (Session { config: Some(config.clone()), ..Default::default() }, config)
}
fn custom(s: &mut Session, name: &str) {
    s.execute("current.createGraphList", json!({"name":name})).unwrap();
}
fn save(s: &mut Session, list: &str, name: &str, config: GraphConfig) {
    s.execute("current.saveGraph", json!({"list":list,"name":name,"config":config})).unwrap();
}

#[test]
fn default_families_are_original_read_only_named_curves_and_export_without_duplicates() {
    let (mut s, _) = session();
    let defaults = s.execute("current.graphLists", json!({})).unwrap()["defaults"].clone();
    let names: Vec<_> = defaults["graphs"].as_array().unwrap().iter().map(|g| g["name"].as_str().unwrap()).collect();
    for family in ["Ease", "Quad", "Cubic", "Quart", "Quint", "Sine", "Expo", "Circ", "Back"] {
        assert!(names.contains(&family));
        assert!(names.contains(&format!("{family} In").as_str()));
        assert!(names.contains(&format!("{family} Out").as_str()));
    }
    assert_eq!(names.iter().map(|s| s.to_lowercase()).collect::<BTreeSet<_>>().len(), names.len());
    assert!(s.execute("current.saveGraph", json!({"list":"Defaults","name":"No","config":GraphConfig::default()})).is_err());
    assert!(s.execute("current.deleteGraphList", json!({"list":"Defaults"})).is_err());
    let result = s.execute("current.exportGraphLists", json!({"list":"Defaults"})).unwrap();
    let doc = GraphLists::parse(result["json"].as_str().unwrap()).unwrap();
    assert_eq!(doc.lists[0].name, "Defaults Copy");
    assert_eq!(doc.lists[0].graphs.len(), 41);
}

#[test]
fn standard_polynomials_directions_and_ease_follow_exact_mathematical_definitions() {
    for (family, n) in [(StandardFamily::Quad, 2), (StandardFamily::Cubic, 3), (StandardFamily::Quart, 4), (StandardFamily::Quint, 5)] {
        for t in [0.1f64, 0.25, 0.5, 0.75, 0.9] {
            assert!((Standard { family, direction: Direction::In }.sample(t) - t.powi(n)).abs() < 1e-12);
            assert!((Standard { family, direction: Direction::Out }.sample(t) - (1.0 - (1.0 - t).powi(n))).abs() < 1e-12);
        }
    }
    for t in [0.1f64, 0.3, 0.5, 0.8] {
        let ease = Standard { family: StandardFamily::Ease, direction: Direction::InOut };
        assert!((ease.sample(t) - (3.0 * t * t - 2.0 * t * t * t)).abs() < 1e-12);
        assert!((Standard { family: StandardFamily::Ease, direction: Direction::In }.sample(t) - (2.0 * t * t - t * t * t)).abs() < 1e-12);
        assert!((Standard { family: StandardFamily::Ease, direction: Direction::Out }.sample(t) - (t + t * t - t * t * t)).abs() < 1e-12);
    }
    for family in StandardFamily::ALL {
        for direction in Direction::ALL {
            let curve = Standard { family, direction };
            assert_eq!(curve.sample(0.0), 0.0);
            assert_eq!(curve.sample(1.0), 1.0);
            assert_eq!(curve.sample(f64::NAN), 0.0);
            assert!((0..=1000).all(|i| curve.sample(i as f64 / 1000.0).is_finite()));
        }
    }
    assert!(Standard { family: StandardFamily::Back, direction: Direction::In }.sample(0.2) < 0.0);
    assert!(Standard { family: StandardFamily::Back, direction: Direction::Out }.sample(0.8) > 1.0);
}

#[test]
fn standard_baking_evaluates_quad_and_shared_refinement_then_undoes_original_keys() {
    let (mut s, layer) = setup();
    animate(&mut s, layer, "transform/rotation", &[(0.0, json!(10)), (1.0, json!(210))]);
    s.execute("keys.select", json!({"keys":[{"layer":layer,"path":"transform/rotation","time":0},{"layer":layer,"path":"transform/rotation","time":1}]}))
        .unwrap();
    let before = prop(&s, layer, "transform/rotation");
    let undo = s.history.undo.len();
    let result = s.execute("keys.currentStandard", json!({"family":"quad","direction":"in","density":2})).unwrap();
    let keys = prop(&s, layer, "transform/rotation").keys;
    for key in keys.iter() {
        assert!((key.value.as_f64() - (10.0 + 200.0 * key.time.seconds().powi(2))).abs() < 1e-9);
    }
    s.execute("keys.currentStandard", json!({"family":"quint","direction":"out","density":2,"token":result["token"]})).unwrap();
    assert_eq!(prop(&s, layer, "transform/rotation").keys.len(), keys.len());
    assert_eq!(s.history.undo.len(), undo + 1);
    s.execute("edit.undo", json!({})).unwrap();
    assert_eq!(prop(&s, layer, "transform/rotation"), before);
}

#[test]
fn multiple_lists_isolate_graphs_rename_delete_and_persist_without_project_undo() {
    let (mut s, config) = session();
    let undo = s.history.undo.len();
    custom(&mut s, "Editing");
    custom(&mut s, "Motion");
    save(&mut s, "Editing", "Same name", GraphConfig::default());
    save(&mut s, "Motion", "Same name", GraphConfig { mode: GraphMode::Bounce, ..Default::default() });
    s.execute("current.renameGraphList", json!({"list":"Motion","name":"Rebounds"})).unwrap();
    let result = s.execute("current.graphLists", json!({})).unwrap();
    assert_eq!(result["lists"][0]["graphs"][0]["config"]["mode"], "bezier");
    assert_eq!(result["lists"][1]["graphs"][0]["config"]["mode"], "bounce");
    let mut reopened = Session { config: Some(config), ..Default::default() };
    assert_eq!(reopened.execute("current.graphLists", json!({})).unwrap()["lists"], result["lists"]);
    s.execute("current.deleteGraph", json!({"list":"Editing","name":"Same name"})).unwrap();
    s.execute("current.deleteGraphList", json!({"list":"Rebounds"})).unwrap();
    assert_eq!(s.execute("current.graphLists", json!({})).unwrap()["lists"], json!([{"name":"Editing","graphs":[]} ]));
    assert_eq!(s.history.undo.len(), undo);
}

#[test]
fn canonical_export_import_preserves_all_graph_modes_and_existing_lists() {
    let (mut s, _) = session();
    custom(&mut s, "Set");
    for (i, mode) in [GraphMode::Bezier, GraphMode::Spring, GraphMode::Elastic, GraphMode::Bounce, GraphMode::Standard].into_iter().enumerate() {
        save(
            &mut s,
            "Set",
            &format!("Curve {i}"),
            GraphConfig { mode, curve: [0.0, -9.0, 1.0, 10.0], family: StandardFamily::Quint, direction: Direction::Out, ..Default::default() },
        );
    }
    let text = s.execute("current.exportGraphLists", json!({"list":"Set"})).unwrap()["json"].as_str().unwrap().to_string();
    let original = GraphLists::parse(&text).unwrap();
    let result = s.execute("current.importGraphLists", json!({"text":text})).unwrap();
    assert_eq!(result["imported"], json!(["Set (2)"]));
    let lists: Vec<GraphList> = serde_json::from_value(result["lists"].clone()).unwrap();
    assert_eq!(lists[0], original.lists[0]);
    assert_eq!(lists[0].graphs, lists[1].graphs);
}

#[test]
fn hostile_libraries_are_rejected_atomically_including_duplicate_keys_and_invalid_configs() {
    let (mut s, config) = session();
    custom(&mut s, "Existing");
    let before = config.read(FILE).unwrap();
    let valid = GraphLists {
        lists: vec![GraphList { name: "Input".into(), graphs: vec![Graph { name: "Curve".into(), config: GraphConfig::default() }] }],
        ..Default::default()
    };
    let mut docs = Vec::new();
    let mut v = json!(valid);
    v["version"] = json!(2);
    docs.push(v);
    let mut v = json!(valid);
    v["lists"][0]["graphs"][0]["config"]["curve"] = json!([0.3, 11, 0.7, 1]);
    docs.push(v);
    let mut v = json!(valid);
    v["lists"][0]["graphs"][0]["config"]["expression"] = json!("untrusted()");
    docs.push(v);
    let mut v = json!(valid);
    v["lists"][0]["name"] = json!("Defaults");
    docs.push(v);
    let mut v = json!(valid);
    v["lists"] = json!([{"name":"A","graphs":[]},{"name":"a","graphs":[]}]);
    docs.push(v);
    let mut v = json!(valid);
    v["lists"] = json!((0..33).map(|i| json!({"name":format!("List{i}"),"graphs":[]})).collect::<Vec<_>>());
    docs.push(v);
    for doc in docs {
        assert!(s.execute("current.importGraphLists", json!({"document":doc})).is_err());
        assert_eq!(config.read(FILE).unwrap(), before);
    }
    let duplicate = r#"{"format":"current.graph-lists","version":1,"version":1,"lists":[]}"#;
    assert!(s.execute("current.importGraphLists", json!({"text":duplicate})).is_err());
    assert!(s.execute("current.importGraphLists", json!({"text":" ".repeat(MAX_BYTES+1)})).is_err());
    assert_eq!(config.read(FILE).unwrap(), before);
}

#[test]
fn old_libraries_migrate_lazily_without_overwriting_originals() {
    let (mut s, config) = session();
    s.execute("current.savePreset", json!({"name":"Mine","curve":[0.2,0,0.8,1]})).unwrap();
    s.execute("current.savePreset", json!({"name":"mine","curve":[0.3,0,0.7,1]})).unwrap();
    s.execute("current.savePhysicsPreset", json!({"name":"Bounce","physics":{"mode":"bounce","frequency":4}})).unwrap();
    assert!(config.read(FILE).is_none());
    let bezier = config.read("current-curves.json").unwrap();
    let motion = config.read("current-physics-curves.json").unwrap();
    let result = s.execute("current.graphLists", json!({})).unwrap();
    assert_eq!(result["lists"].as_array().unwrap().len(), 2);
    assert_eq!(result["lists"][0]["graphs"][1]["name"], "mine (2)");
    assert_eq!(config.read("current-curves.json").unwrap(), bezier);
    assert_eq!(config.read("current-physics-curves.json").unwrap(), motion);
    let stored = config.read(FILE).unwrap();
    s.execute("current.graphLists", json!({})).unwrap();
    assert_eq!(config.read(FILE).unwrap(), stored);
}

#[test]
fn flow_synthetic_records_strings_legacy_maps_and_overshoot_round_trip_without_runtime_execution() {
    let (mut s, _) = session();
    let result = s
        .execute(
            "current.importGraphLists",
            json!({"text":r#"[{"_id":"own-fixture","name":"Lift","value":[0,-9,1,10]},{"name":"lift","curve":"0.2, 0, 0.8, 1"}]"#}),
        )
        .unwrap();
    assert_eq!(serde_json::from_value::<[f64; 4]>(result["lists"][0]["graphs"][0]["config"]["curve"].clone()).unwrap(), [0.0, -9.0, 1.0, 10.0]);
    assert_eq!(result["lists"][0]["graphs"][1]["name"], "lift (2)");
    let exported = s.execute("current.exportGraphLists", json!({"list":"Imported Flow","format":"flow"})).unwrap()["json"].as_str().unwrap().to_string();
    let flow: Value = serde_json::from_str(&exported).unwrap();
    assert!(flow[0].get("_id").is_none());
    s.execute("current.importGraphLists", json!({"text":exported})).unwrap();
    let result = s.execute("current.importGraphLists", json!({"text":r#"{"Original map":"0.1,0,0.9,1"}"#})).unwrap();
    assert_eq!(result["imported"], json!(["Imported Flow (3)"]));
    assert_eq!(result["lists"][2]["graphs"][0]["name"], "Original map");
}

#[test]
fn flow_export_refuses_lossy_physics_and_flow_import_rejects_unsafe_data_atomically() {
    let (mut s, config) = session();
    custom(&mut s, "Physics");
    save(&mut s, "Physics", "Spring", GraphConfig { mode: GraphMode::Spring, ..Default::default() });
    let before = config.read(FILE).unwrap();
    assert!(s.execute("current.exportGraphLists", json!({"list":"Physics","format":"flow"})).unwrap_err().to_string().contains("no Bezier graphs"));
    for text in [
        r#"[{"name":"Bad","value":[0,0,1]}]"#,
        r#"[{"name":"Bad","value":"0,NaN,1,1"}]"#,
        r#"[{"name":"Bad","value":[0,0,1,1],"expression":"untrusted()"}]"#,
        r#"{"Same":[0,0,1,1],"Same":[0.2,0,0.8,1]}"#,
    ] {
        assert!(s.execute("current.importGraphLists", json!({"text":text})).is_err());
        assert_eq!(config.read(FILE).unwrap(), before);
    }
}

#[test]
fn flow_export_skips_non_bezier_modes_and_reports_every_skipped_name() {
    let (mut s, _) = session();
    custom(&mut s, "Mixed");
    save(&mut s, "Mixed", "Bezier", GraphConfig::default());
    for (name, mode) in [("Spring", GraphMode::Spring), ("Standard", GraphMode::Standard), ("Elastic", GraphMode::Elastic), ("Bounce", GraphMode::Bounce)] {
        save(&mut s, "Mixed", name, GraphConfig { mode, ..Default::default() });
    }
    let result = s.execute("current.exportGraphLists", json!({"list":"Mixed","format":"flow"})).unwrap();
    assert_eq!(result["exportedCount"], 1);
    assert_eq!(result["skippedCount"], 4);
    assert_eq!(result["skipped"], json!(["Spring", "Standard", "Elastic", "Bounce"]));
    let exported: Value = serde_json::from_str(result["json"].as_str().unwrap()).unwrap();
    assert_eq!(exported.as_array().unwrap().len(), 1);
    assert_eq!(exported[0]["name"], "Bezier");
    let lossless = s.execute("current.exportGraphLists", json!({"list":"Mixed"})).unwrap();
    assert_eq!(lossless["exportedCount"], 5);
    assert_eq!(lossless["skippedCount"], 0);
}
