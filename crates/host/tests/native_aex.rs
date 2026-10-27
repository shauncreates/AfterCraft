//! Actual Session -> layer effect -> native worker -> composition render.
#![cfg(windows)]
use effectcraft_engine::time::Tick;
use serde_json::json;

#[test]
fn edition_dialog_is_first_run_only_and_can_be_reopened_explicitly() {
    let mut s = effectcraft_engine::Session {
        plugin_installer: Some(|_, _| Ok(json!({"edition":null,"effectiveEdition":"ofx","needsChoice":true}))),
        ..Default::default()
    };
    s.execute("effect.plugins.preference", json!({"first_launch":true})).unwrap();
    assert!(s.events.iter().any(|event| matches!(event,effectcraft_engine::Event::Frontend {command,..} if command=="effect.plugins.chooseEdition")));
    s.events.clear();
    s.plugin_installer = Some(|_, _| Ok(json!({"edition":"ofx","effectiveEdition":"ofx","needsChoice":false})));
    s.execute("effect.plugins.preference", json!({"first_launch":true})).unwrap();
    assert!(s.events.is_empty());
    s.execute("effect.plugins.preference", json!({})).unwrap();
    assert!(s.events.iter().any(|event| matches!(event,effectcraft_engine::Event::Frontend {command,..} if command=="effect.plugins.chooseEdition")));
    s.events.clear();
    s.plugin_installer = Some(|_, _| Ok(json!({"edition":null,"effectiveEdition":"ofx","needsChoice":false})));
    s.execute("effect.plugins.preference", json!({"first_launch":true})).unwrap();
    assert!(s.events.is_empty());
}

#[test]
fn native_installer_rejects_malformed_arguments_and_records_background_results() {
    let mut s = effectcraft_engine::Session::default();
    for command in ["effect.plugins.install", "effect.plugins.scanInstalled", "effect.plugins.addLibrary"] {
        for value in [json!([]), json!("bad"), json!(1)] {
            assert!(s.execute(command, value).is_err());
        }
    }
    s.plugin_loader = Some(|_, _| Ok(json!({})));
    s.plugin_installer = Some(|request, progress| {
        assert_eq!(request["installed"], true);
        assert!(progress(1, 2, "Discovering test effects"));
        Ok(json!({"profiles":[],"errors":[{"path":"test.aex","error":"unsupported host service"}]}))
    });
    let report = s.execute("effect.plugins.scanInstalled", json!({"wait":true})).unwrap();
    assert_eq!(report["errors"][0]["error"], "unsupported host service");
    assert_eq!(s.job_log.last().unwrap().status, "done");
}

#[test]
#[ignore = "Requires EFFECTCRAFT_OFX_TEST_WORKER and EFFECTCRAFT_OFX_TEST_BUNDLE"]
fn ofx_installation_renders_and_changes_controls_in_a_full_session() {
    let worker = std::path::PathBuf::from(std::env::var("EFFECTCRAFT_OFX_TEST_WORKER").unwrap());
    let bundle = std::path::PathBuf::from(std::env::var("EFFECTCRAFT_OFX_TEST_BUNDLE").unwrap());
    let settings = worker.parent().unwrap().join(format!("session-fixture-{}", std::process::id()));
    let mut s = effectcraft_host::session();
    s.plugin_installer = Some(|request, progress| {
        let mut request = request.clone();
        request["worker"] = json!(std::env::var("EFFECTCRAFT_AEX_TEST_WORKER").unwrap());
        request["ofx_worker"] = json!(std::env::var("EFFECTCRAFT_OFX_TEST_WORKER").unwrap());
        effectcraft_plugin::installer(&request, progress)
    });
    let report = s.execute("effect.plugins.scan", json!({"folder":bundle,"settings_folder":settings,"wait":true})).unwrap();
    assert_eq!(report["errors"], json!([]));
    assert_eq!(report["loadErrors"], json!([]));
    assert!(report["loaded"].as_array().unwrap().len() >= 5);
    s.execute("comp.new", json!({"name":"OFX Session","width":32,"height":16,"duration":1,"frameRate":30000.0/1001.0})).unwrap();
    s.execute("layer.newSolid", json!({"color":[0.2,0.3,0.4],"width":32,"height":16})).unwrap();
    let comp = s.active_comp_id().unwrap();
    let original = s.render(comp, Tick::ZERO, Default::default()).get(16, 8);
    s.execute("effect.apply", json!({"layer":"#1","effect":"ofx.org.effectcraft.test.invert"})).unwrap();
    let pixel = s.render(comp, Tick::ZERO, Default::default()).get(16, 8);
    for (actual, expected) in pixel.into_iter().zip([original[3] - original[0], original[3] - original[1], original[3] - original[2], original[3]]) {
        assert!((actual - expected).abs() < 0.0001, "{pixel:?}");
    }
    s.execute("prop.set", json!({"layer":"#1","path":"effects/#1/amount","value":0})).unwrap();
    let pixel = s.render(comp, Tick::ZERO, Default::default()).get(16, 8);
    for (actual, expected) in pixel.into_iter().zip(original) {
        assert!((actual - expected).abs() < 0.0001, "{pixel:?}");
    }
    // A temporal effect must see the animated output of earlier effects, including
    // another OFX effect. Holding the helper-pool lock here deadlocked the checkout.
    s.execute("effect.apply", json!({"layer":"#1","effect":"ec.generate.fill"})).unwrap();
    s.edit("Animate temporal fixture", None, |project, _| {
        // Compare the actual float average, without a second 8-bpc quantization step.
        project.settings.bit_depth = effectcraft_project::BitDepth::Bpc32;
        let group = project.comp_mut(comp).unwrap().layers.first_mut().unwrap().props.group_mut("effects/#2").unwrap();
        group.prop_mut("color").unwrap().keys = vec![
            effectcraft_engine::keyframe::Keyframe::new(Tick::ZERO, effectcraft_engine::keyframe::Value::Color([0.1, 0.2, 0.3, 1.0])),
            effectcraft_engine::keyframe::Keyframe::new(Tick::from_seconds_f64(1.0), effectcraft_engine::keyframe::Value::Color([0.8, 0.6, 0.4, 1.0])),
        ];
        Ok(())
    })
    .unwrap();
    let time = 0.75;
    let previous = time - 1001.0 / 30000.0;
    let mut expected = Vec::new();
    for scale in [1.0, 0.5, 0.25] {
        let opts = effectcraft_engine::render::RenderOpts { scale, ..Default::default() };
        let now = s.render(comp, Tick::from_seconds_f64(time), opts);
        let before = s.render(comp, Tick::from_seconds_f64(previous), opts);
        let a = now.get(i64::from(now.width / 2), i64::from(now.height / 2));
        let b = before.get(i64::from(before.width / 2), i64::from(before.height / 2));
        assert!((a[0] - b[0]).abs() > 0.001, "fixture must change with time");
        expected.push((scale, std::array::from_fn::<_, 4, _>(|i| 0.5 * (a[i] + b[i]))));
    }
    s.execute("effect.apply", json!({"layer":"#1","effect":"ofx.org.effectcraft.test.echo"})).unwrap();
    for (scale, expected) in expected {
        let image = s.render(comp, Tick::from_seconds_f64(time), effectcraft_engine::render::RenderOpts { scale, ..Default::default() });
        let actual = image.get(i64::from(image.width / 2), i64::from(image.height / 2));
        for (a, b) in actual.into_iter().zip(expected) {
            assert!((a - b).abs() < 0.0001, "scale={scale}: {actual:?} != {expected:?}");
        }
    }
}

#[test]
#[ignore = "Requires EFFECTCRAFT_AEX_TEST_WORKER and EFFECTCRAFT_AEX_TEST_PLUGIN"]
fn native_aex_renders_in_effectcraft_session() {
    let dir = std::env::temp_dir().join(format!("effectcraft-native-session-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let descriptor = dir.join("gain.ecaex.json");
    let config = json!({"experimental_aex":true,"worker":std::env::var("EFFECTCRAFT_AEX_TEST_WORKER").unwrap(),
        "plugin":std::env::var("EFFECTCRAFT_AEX_TEST_PLUGIN").unwrap(),"parameter_indices":[1],
        "manifest":{"api":1,"id":"org.test.session.native_gain","name":"Experimental Native Gain",
        "category":"Experimental AEX","params":[{"id":"gain","name":"Gain","type":"slider","default":2,"min":0,"max":4}]}});
    std::fs::write(&descriptor, serde_json::to_vec(&config).unwrap()).unwrap();
    let mut s = effectcraft_host::session();
    // Reproduce a plugin loaded after the UI has already built its menus.
    let _ = effectcraft_engine::menus::menu_bar();
    s.execute("effect.plugins.load", json!({"path":descriptor})).unwrap();
    assert!(
        effectcraft_engine::menus::entries()
            .iter()
            .any(|(path, entry)| path.iter().any(|part| part == "Experimental AEX") && entry.params["effect"] == "org.test.session.native_gain")
    );
    s.execute("comp.new", json!({"name":"Native AEX integration","width":40,"height":20,"duration":1})).unwrap();
    let layer = s.execute("layer.newSolid", json!({"color":[0.2,0.3,0.4],"width":40,"height":20})).unwrap()["layer"].as_u64().unwrap();
    s.execute("effect.apply", json!({"layer":layer,"effect":"org.test.session.native_gain"})).unwrap();
    let comp = s.active_comp_id().unwrap();
    let doubled = s.render(comp, Tick::ZERO, Default::default()).get(20, 10);
    assert!((doubled[0] - 0.4).abs() < 0.01 && (doubled[1] - 0.6).abs() < 0.01 && (doubled[2] - 0.8).abs() < 0.01, "{doubled:?}");
    s.execute("prop.set", json!({"layer":layer,"path":"effects/#1/gain","value":0.5})).unwrap();
    let halved = s.render(comp, Tick::ZERO, Default::default()).get(20, 10);
    assert!((halved[0] - 0.1).abs() < 0.01 && (halved[1] - 0.15).abs() < 0.01 && (halved[2] - 0.2).abs() < 0.01, "{halved:?}");
    for factor in [2u32, 3, 4, 5, 8] {
        let preview = s.render(comp, Tick::ZERO, effectcraft_engine::render::RenderOpts { scale: 1.0 / f64::from(factor), ..Default::default() });
        let pixel = preview.get(i64::from(preview.width / 2), i64::from(preview.height / 2));
        for (actual, expected) in pixel.into_iter().zip(halved) {
            assert!((actual - expected).abs() < 0.01, "1/{factor} Session preview changed native gain: {pixel:?}");
        }
    }
    let mut clock_config = config.clone();
    clock_config["manifest"]["id"] = json!("org.test.session.native_clock");
    clock_config["manifest"]["name"] = json!("Native Clock Probe");
    clock_config["manifest"]["params"] = json!([{"id":"clock","name":"Encode Clock","type":"checkbox","default":true}]);
    clock_config["parameter_indices"] = json!([2]);
    let clock_descriptor = dir.join("clock.ecaex.json");
    std::fs::write(&clock_descriptor, serde_json::to_vec(&clock_config).unwrap()).unwrap();
    s.execute("effect.plugins.load", json!({"path":clock_descriptor})).unwrap();
    s.execute("comp.new", json!({"name":"NTSC Clock","width":40,"height":20,"duration":400,"frameRate":30000.0/1001.0})).unwrap();
    let layer = s.execute("layer.newSolid", json!({"color":[0.2,0.3,0.4],"width":40,"height":20})).unwrap()["layer"].as_u64().unwrap();
    s.execute("effect.apply", json!({"layer":layer,"effect":"org.test.session.native_clock"})).unwrap();
    let actual = s.render(s.active_comp_id().unwrap(), Tick::from_seconds_f64(10000.0 * 1001.0 / 30000.0), Default::default()).get(20, 10);
    for (value, expected) in actual.into_iter().zip([(1001 % 256) as f32 / 255.0, (30000 % 256) as f32 / 255.0, (10010000 % 256) as f32 / 255.0, 1.0]) {
        assert!((value - expected).abs() < 0.01, "Native clock did not match composition: {actual:?}");
    }
    std::fs::remove_file(clock_descriptor).unwrap();
    let mut typed_config = config.clone();
    typed_config["manifest"]["id"] = json!("org.test.session.native_typed");
    typed_config["manifest"]["name"] = json!("Native Typed Probe");
    typed_config["manifest"]["params"] = json!([
        {"id":"mode","name":"Mode","type":"popup","options":["Gain","Typed Controls"],"default":1},
        {"id":"color","name":"Color","type":"color","default":[0.25,0.5,0.75,1]},
        {"id":"point","name":"Point","type":"point","default":[0.25,0.5]},
        {"id":"angle","name":"Angle","type":"angle","default":91}]);
    typed_config["parameter_indices"] = json!([6, 3, 4, 5]);
    let typed_descriptor = dir.join("typed.ecaex.json");
    std::fs::write(&typed_descriptor, serde_json::to_vec(&typed_config).unwrap()).unwrap();
    s.execute("effect.plugins.load", json!({"path":typed_descriptor})).unwrap();
    s.execute("comp.new", json!({"name":"Typed Controls","width":40,"height":20,"duration":1})).unwrap();
    let layer = s.execute("layer.newSolid", json!({"color":[0.2,0.3,0.4],"width":40,"height":20})).unwrap()["layer"].as_u64().unwrap();
    s.execute("effect.apply", json!({"layer":layer,"effect":"org.test.session.native_typed"})).unwrap();
    s.execute("prop.set", json!({"layer":layer,"path":"effects/#1/point","value":[12,9]})).unwrap();
    let typed = s.render(s.active_comp_id().unwrap(), Tick::ZERO, Default::default()).get(20, 10);
    for (value, expected) in typed.into_iter().zip([64.0 / 255.0, 21.0 / 255.0, 91.0 / 255.0, 1.0]) {
        assert!((value - expected).abs() < 0.01, "Typed native controls changed across the host: {typed:?}");
    }
    std::fs::remove_file(typed_descriptor).unwrap();
    std::fs::remove_file(descriptor).unwrap();
    std::fs::remove_dir(dir).unwrap();
}
