//! Actual parser -> isolated scripting session -> one transactional project merge.
//! No native commercial effect is rendered in this test.

#[test]
fn scripted_subframe_timing_keeps_distinct_keys_and_bezier_handles() {
    use effectcraft_engine::time::Tick;
    use serde_json::json;
    let mut session = effectcraft_host::session();
    let result = session
        .execute(
            "script.run",
            json!({"code":r#"
        var c=app.project.items.addComp('Subframe import',32,16,1,25,60);
        var l=c.layers.addSolid([1,1,1],'Original fixture',32,16,1,25);
        l.startTime=17.18045351473923; l.stretch=125;
        var p=l.property('ADBE Transform Group').property('ADBE Opacity');
        p.setValuesAtTimes([19.182486979166666,19.185],[20,80]);
        p.setInterpolationTypeAtKey(1,KeyframeInterpolationType.BEZIER,KeyframeInterpolationType.BEZIER);
        p.setTemporalEaseAtKey(1,[new KeyframeEase(12,70)],[new KeyframeEase(24,80)]);
        c.openInViewer();
    "#}),
        )
        .unwrap();
    assert_eq!(result["ok"], true, "{result}");
    let layer = &session.active_comp().unwrap().layers[0];
    assert_eq!(layer.start_time, Tick::from_seconds_f64(17.18045351473923));
    let keys = &layer.props.prop("transform/opacity").unwrap().keys;
    assert_eq!(keys.len(), 2, "two subframe keys must not collapse onto a frame");
    for (key, expected) in keys.iter().zip([19.182486979166666, 19.185]) {
        assert!((layer.comp_time(key.time).seconds() - expected).abs() < 1e-9);
    }
    assert!((keys[0].in_ease[0].speed - 12.0).abs() < 1e-9);
    assert!((keys[0].out_ease[0].influence - 0.8).abs() < 1e-9);
    // Normal engine/UI calls still snap unless explicitly opting out.
    session.execute("layer.timing", json!({"start":0.182486979166666})).unwrap();
    assert!((session.active_comp().unwrap().layers[0].start_time.seconds() - 11.0 / 60.0).abs() < 1e-9);
}

#[test]
fn audio_placeholders_retain_animatable_audio_levels() {
    use serde_json::json;
    let mut session = effectcraft_host::session();
    session.execute("comp.new", json!({"name":"Audio placeholder","width":32,"height":16,"duration":2})).unwrap();
    let item =
        session.execute("file.importPlaceholder", json!({"name":"Missing original audio","hasAudio":true,"hasVideo":false,"duration":2})).unwrap()["item"]
            .as_u64()
            .unwrap();
    session.execute("layer.addItem", json!({"item":item})).unwrap();
    let result = session.execute("script.run", json!({"code":"var p=app.project.activeItem.layer(1).property('ADBE Audio Group').property('ADBE Audio Levels');p.setValuesAtTimes([0,1],[[-48,-48],[0,0]]);"})).unwrap();
    assert_eq!(result["ok"], true, "{result}");
    let levels = session.active_comp().unwrap().layers[0].props.prop("audio/levels").unwrap();
    assert_eq!(levels.keys.len(), 2);
}

#[test]
#[ignore = "requires AFTERCRAFT_AEP_TEST_PROJECT, Python and the pinned private py-aep runtime"]
fn aep_import_builds_a_staging_project_and_one_undo_restores_the_open_document() {
    use serde_json::json;
    let path = std::env::var("AFTERCRAFT_AEP_TEST_PROJECT").unwrap();
    let mut session = effectcraft_host::session();
    session.execute("comp.new", json!({"name":"Existing project","width":32,"height":16,"duration":2})).unwrap();
    let before = session.project.clone();
    let result = session.execute("file.importAep", json!({"path":path,"edition":"ofx"})).unwrap();
    assert!(!result["comps"].as_array().unwrap().is_empty(), "{result}");
    let actual = &result["report"]["actualImported"];
    assert!(actual["comps"].as_u64().unwrap() > 0);
    assert_eq!(session.project.comps().count(), before.comps().count() + actual["comps"].as_u64().unwrap() as usize);
    let artifact = std::path::PathBuf::from(result["report"]["artifactDirectory"].as_str().unwrap());
    assert!(artifact.join("report.json").is_file());
    assert!(artifact.join("project.jsx").is_file());
    assert!(!result["report"]["fidelityVerified"].as_bool().unwrap(), "structural import is not a render-fidelity claim");
    session.execute("edit.undo", json!({})).unwrap();
    assert_eq!(session.project, before);
}
