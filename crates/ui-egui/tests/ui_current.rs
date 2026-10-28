//! Current's visible controls apply a real curve through the undoable engine command.

use std::sync::Arc;

use effectcraft_engine::{Session, config::MemoryConfig, keyframe::Interp};
use effectcraft_ui_egui::{EffectcraftApp, dock::PanelKind};
use egui::{Event, pos2, vec2};
use egui_kittest::Harness;
use serde_json::json;

fn harness() -> (Harness<'static, EffectcraftApp>, u64) {
    harness_size(vec2(1100.0, 1300.0))
}

fn harness_size(size: egui::Vec2) -> (Harness<'static, EffectcraftApp>, u64) {
    let mut session = Session { config: Some(Arc::new(MemoryConfig::default())), ..Session::default() };
    session.execute("comp.new", json!({"name":"Current", "width":320, "height":180, "duration":2})).unwrap();
    let layer = session.execute("layer.newSolid", json!({"name":"Plate", "color":"#406080"})).unwrap()["layer"].as_u64().unwrap();
    for (time, value) in [(0, 0), (1, 100)] {
        session.execute("prop.addKey", json!({"layer":layer, "path":"transform/rotation", "time":time, "value":value})).unwrap();
    }
    session
        .execute("keys.select", json!({"keys":[{"layer":layer,"path":"transform/rotation","time":0},{"layer":layer,"path":"transform/rotation","time":1}]}))
        .unwrap();
    let mut app = EffectcraftApp::new(session);
    app.show_panel(PanelKind::Current);
    app.ui.maximized = Some(PanelKind::Current);
    let mut h = Harness::builder().with_size(size).build_eframe(|_| app);
    h.run_steps(3);
    (h, layer)
}

fn keys(h: &Harness<'_, EffectcraftApp>, layer: u64) -> Vec<effectcraft_engine::keyframe::Keyframe> {
    h.state().session.active_comp().unwrap().layer(effectcraft_engine::project::LayerId(layer)).unwrap().props.prop("transform/rotation").unwrap().keys.clone()
}

fn click(h: &mut Harness<'_, EffectcraftApp>, id: &str) {
    let r = h.state().auto.find(id).unwrap_or_else(|| panic!("missing control {id}")).rect;
    let at = pos2(r[0] + r[2] * 0.5, r[1] + r[3] * 0.5);
    h.input_mut().events.push(Event::PointerMoved(at));
    h.input_mut().events.push(Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
    h.step();
    h.input_mut().events.push(Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
    h.run_steps(2);
}

fn scroll_library_footer_into_view(h: &mut Harness<'_, EffectcraftApp>) {
    // Saving adds another row of thumbnails. Reach the footer as a user would,
    // rather than sending a click to a control below the clipped viewport.
    h.input_mut().events.push(Event::PointerMoved(pos2(500.0, 500.0)));
    h.input_mut().events.push(Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(0.0, -600.0),
        modifiers: Default::default(),
        phase: egui::TouchPhase::Move,
    });
    h.run_steps(12);
}

fn create_list(h: &mut Harness<'_, EffectcraftApp>, name: &str) {
    h.state_mut().ui.current.list_name = name.into();
    h.run_steps(2);
    scroll_library_footer_into_view(h);
    click(h, "current.newList");
    assert_eq!(h.state().ui.current.selected_list, name);
}

#[test]
fn current_preset_applies_native_easing_undoes_and_saves_a_curve() {
    let (mut h, layer) = harness();
    if let Some(path) = std::env::var_os("AFTERCRAFT_CURRENT_SNAPSHOT") {
        h.render().unwrap().save(path).unwrap();
    }
    for id in ["current.graph", "current.handle1", "current.handle2", "current.values", "current.apply", "current.read", "current.name"] {
        assert!(h.state().auto.find(id).is_some(), "missing {id}");
    }
    click(&mut h, "current.preset.4");
    assert_eq!(h.state().ui.current.curve, [0.78, 0.0, 0.22, 1.0]);
    click(&mut h, "current.apply");
    let prop = h.state().session.active_comp().unwrap().layer(effectcraft_engine::project::LayerId(layer)).unwrap().props.prop("transform/rotation").unwrap();
    assert_eq!(prop.keys[0].out_interp, Interp::Bezier);
    assert!((prop.keys[0].out_ease[0].influence - 0.78).abs() < 1e-10);
    click(&mut h, "current.read");
    assert!(h.state().ui.current.curve.iter().zip([0.78, 0.0, 0.22, 1.0]).all(|(a, b)| (a - b).abs() < 1e-10));
    h.state_mut().session.execute("edit.undo", json!({})).unwrap();
    let prop = h.state().session.active_comp().unwrap().layer(effectcraft_engine::project::LayerId(layer)).unwrap().props.prop("transform/rotation").unwrap();
    assert_eq!(prop.keys[0].out_interp, Interp::Linear);
    h.state_mut().ui.current.name = "My snap".into();
    h.run_steps(2);
    create_list(&mut h, "My curves");
    click(&mut h, "current.save");
    assert_eq!(h.state().ui.current.lists[0].graphs.len(), 1);
    assert_eq!(h.state().ui.current.lists[0].graphs[0].name, "My snap");
    scroll_library_footer_into_view(&mut h);
    click(&mut h, "current.delete");
    assert!(h.state().ui.current.lists[0].graphs.is_empty());
}

#[test]
fn physics_modes_preview_bake_refine_undo_and_save_real_motion() {
    for mode in ["spring", "elastic", "bounce"] {
        let (mut h, layer) = harness();
        let before = keys(&h, layer);
        let undo = h.state().session.history.undo.len();
        click(&mut h, &format!("current.mode.{mode}"));
        for id in ["current.frequency", "current.damping", "current.amplitude", "current.density", "current.motionPreview", "current.liveApply"] {
            assert!(h.state().auto.find(id).is_some(), "missing {id}");
        }
        assert_eq!(keys(&h, layer), before, "choosing a mode only previews it");
        click(&mut h, "current.previewPlay");
        h.run_steps(4);
        assert!(h.state().ui.current.preview_playing);
        assert_eq!(keys(&h, layer), before, "preview playback must not mutate project keys");
        assert_eq!(h.state().session.history.undo.len(), undo);
        click(&mut h, "current.previewPlay");
        assert!(!h.state().ui.current.preview_playing);
        click(&mut h, "current.apply");
        let baked = keys(&h, layer);
        assert!(baked.len() > 2, "{mode} must create actual intermediate keys");
        assert_eq!((baked.first().unwrap().time, &baked.first().unwrap().value), (before[0].time, &before[0].value));
        assert_eq!((baked.last().unwrap().time, &baked.last().unwrap().value), (before[1].time, &before[1].value));
        if mode == "bounce" {
            assert!(baked.iter().all(|key| (0.0..=100.0).contains(&key.value.as_f64())));
        } else {
            assert!(baked.iter().any(|key| key.value.as_f64() > 100.0), "{mode} must visibly overshoot");
        }
        assert_eq!(h.state().session.history.undo.len(), undo + 1);
        h.state_mut().ui.current.physics.frequency = 5.0;
        h.run_steps(2);
        click(&mut h, "current.apply");
        assert_ne!(keys(&h, layer), baked, "refinement changes the generated motion");
        assert_eq!(keys(&h, layer).len(), baked.len(), "refinement replaces generated samples instead of accumulating them");
        assert_eq!(h.state().session.history.undo.len(), undo + 1, "one refinement session remains one undo step");
        if mode == "spring"
            && let Some(path) = std::env::var_os("AFTERCRAFT_CURRENT_PHYSICS_SNAPSHOT")
        {
            h.render().unwrap().save(path).unwrap();
        }
        h.state_mut().session.execute("edit.undo", json!({})).unwrap();
        assert_eq!(keys(&h, layer), before);
        h.state_mut().ui.current.name = format!("My {mode}");
        h.run_steps(2);
        create_list(&mut h, "My motion");
        click(&mut h, "current.save");
        assert_eq!(h.state().ui.current.lists[0].graphs.len(), 1);
        assert_eq!(h.state().ui.current.lists[0].graphs[0].config.frequency, 5.0);
        scroll_library_footer_into_view(&mut h);
        click(&mut h, "current.delete");
        assert!(h.state().ui.current.lists[0].graphs.is_empty());
    }
}

#[test]
fn compact_current_keeps_physics_controls_and_apply_accessible() {
    let (mut h, _) = harness_size(vec2(380.0, 780.0));
    h.state_mut().ui.current.compact = true;
    h.run_steps(2);
    click(&mut h, "current.mode.spring");
    for id in ["current.graph", "current.frequency", "current.density", "current.apply", "current.liveApply", "current.name"] {
        let rect = h.state().auto.find(id).unwrap_or_else(|| panic!("missing {id}")).rect;
        assert!(rect[0] >= 0.0 && rect[0] + rect[2] <= 380.5, "{id} exceeds compact panel width: {rect:?}");
    }
    if let Some(path) = std::env::var_os("AFTERCRAFT_CURRENT_COMPACT_SNAPSHOT") {
        h.render().unwrap().save(path).unwrap();
    }
}

#[test]
fn live_motion_waits_for_a_finished_gesture_and_refines_one_undo_step() {
    let (mut h, layer) = harness();
    click(&mut h, "current.mode.spring");
    let before = keys(&h, layer);
    let undo = h.state().session.history.undo.len();
    click(&mut h, "current.liveApply");
    assert!(h.state().ui.current.live_apply);
    assert_eq!(keys(&h, layer), before, "opting in does not apply motion");
    for time in [100.0, 101.0] {
        let rect = h.state().auto.find("current.frequency").unwrap().rect;
        let from = pos2(rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5);
        let to = from + vec2(28.0, 0.0);
        h.input_mut().time = Some(time);
        h.input_mut().events.push(Event::PointerMoved(from));
        h.input_mut().events.push(Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
        h.step();
        h.input_mut().time = Some(time);
        h.input_mut().events.push(Event::PointerMoved(to));
        h.step();
        h.input_mut().time = Some(time + 0.1);
        h.input_mut().events.push(Event::PointerButton { pos: to, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
        h.step();
        if time == 100.0 {
            assert_eq!(keys(&h, layer), before, "the 250ms settle interval has not elapsed");
        }
        h.input_mut().time = Some(time + 0.5);
        h.step();
        assert!(keys(&h, layer).len() > 2, "settled live gesture must bake actual motion");
        assert_eq!(h.state().session.history.undo.len(), undo + 1, "live refinement keeps one undo step");
    }
    h.input_mut().time = None;
    h.state_mut().session.execute("edit.undo", json!({})).unwrap();
    assert_eq!(keys(&h, layer), before);
}

#[test]
fn standard_mode_previews_and_bakes_exact_quad_with_undo() {
    let (mut h, layer) = harness();
    let before = keys(&h, layer);
    click(&mut h, "current.mode.standard");
    click(&mut h, "current.direction.in");
    assert_eq!(format!("{:?}", h.state().ui.current.standard.family), "Quad");
    assert_eq!(format!("{:?}", h.state().ui.current.standard.direction), "In");
    assert_eq!(keys(&h, layer), before, "family selection is only a preview");
    assert!(h.state().auto.find("current.standardFamily").is_some());
    click(&mut h, "current.apply");
    let baked = keys(&h, layer);
    assert!(baked.len() > 2);
    let start = before[0].time.seconds();
    let duration = before[1].time.seconds() - start;
    for key in &baked {
        let t = (key.time.seconds() - start) / duration;
        assert!((key.value.as_f64() - 100.0 * t.powi(2)).abs() < 1e-8);
    }
    h.state_mut().session.execute("edit.undo", json!({})).unwrap();
    assert_eq!(keys(&h, layer), before);
}

#[test]
fn graph_lists_switch_independently_and_import_export_through_native_dialog_hooks() {
    let (mut h, layer) = harness();
    let project = h.state().session.project.clone();
    click(&mut h, "current.preset.4");
    create_list(&mut h, "First");
    click(&mut h, "current.save");
    assert_eq!(h.state().ui.current.lists[0].graphs[0].name, "Snap");
    create_list(&mut h, "Second");
    h.state_mut().ui.current.name = "Other".into();
    h.run_steps(2);
    click(&mut h, "current.save");
    click(&mut h, "current.listSelector");
    click(&mut h, "current.list.1");
    assert_eq!(h.state().ui.current.selected_list, "First");
    click(&mut h, "current.saved.0");
    assert_eq!(h.state().ui.current.name, "Snap");
    let path = std::env::temp_dir().join(format!(
        "AfterCraft-Current-test-{}-{}.current",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    let export = path.to_string_lossy().to_string();
    h.state_mut().hooks.pick_save_file = Some(Box::new(move |_, _| Some(export.clone())));
    click(&mut h, "current.export");
    let data = std::fs::read_to_string(&path).unwrap();
    let doc = effectcraft_engine::commands::current::GraphLists::parse(&data).unwrap();
    assert_eq!(doc.lists.len(), 1);
    assert_eq!(doc.lists[0].name, "First");
    let import = path.to_string_lossy().to_string();
    h.state_mut().hooks.pick_files = Some(Box::new(move |_| vec![import.clone()]));
    click(&mut h, "current.import");
    assert_eq!(h.state().ui.current.selected_list, path.file_stem().unwrap().to_string_lossy());
    assert_eq!(h.state().ui.current.lists.len(), 3);
    assert_eq!(h.state().ui.current.lists[0].graphs, h.state().ui.current.lists[2].graphs);
    h.state_mut().ui.current.list_name = "First copy".into();
    h.run_steps(2);
    click(&mut h, "current.renameList");
    assert_eq!(h.state().ui.current.selected_list, "First copy");
    click(&mut h, "current.deleteList");
    assert_eq!(h.state().ui.current.lists.len(), 3, "list deletion must be confirmed");
    click(&mut h, "current.confirmDeleteList");
    assert_eq!(h.state().ui.current.lists.len(), 2);
    assert_eq!(h.state().session.project, project, "library work must not edit project data");
    assert_eq!(keys(&h, layer).len(), 2);
    std::fs::remove_file(path).unwrap();
}
