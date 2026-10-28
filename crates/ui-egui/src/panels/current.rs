//! Current: an original, built-in normalized easing editor. No CEP runtime is required.

use effectcraft_engine::commands::current::{
    Curve, Direction, Graph, GraphConfig, GraphList, GraphMode, Physics, PhysicsMode, Standard, StandardFamily, default_graphs,
};
use egui::{Align2, Rect, RichText, Sense, Stroke, pos2, vec2};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::EffectcraftApp;
use crate::theme::Tokens;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurrentMode {
    #[default]
    Bezier,
    Spring,
    Elastic,
    Bounce,
    Standard,
}

impl CurrentMode {
    fn physics(self) -> Option<PhysicsMode> {
        match self {
            Self::Bezier | Self::Standard => None,
            Self::Spring => Some(PhysicsMode::Spring),
            Self::Elastic => Some(PhysicsMode::Elastic),
            Self::Bounce => Some(PhysicsMode::Bounce),
        }
    }
    fn from_physics(mode: PhysicsMode) -> Self {
        match mode {
            PhysicsMode::Spring => Self::Spring,
            PhysicsMode::Elastic => Self::Elastic,
            PhysicsMode::Bounce => Self::Bounce,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CurrentState {
    pub mode: CurrentMode,
    pub physics: Physics,
    pub density: u8,
    pub compact: bool,
    pub curve: [f64; 4],
    pub text: String,
    pub name: String,
    pub lists: Vec<GraphList>,
    pub selected_list: String,
    pub list_name: String,
    pub standard: Standard,
    pub search: String,
    pub export_all: bool,
    pub export_flow: bool,
    pub confirm_delete_list: bool,
    pub message: String,
    #[serde(skip)]
    pub loaded: bool,
    #[serde(skip)]
    pub live_apply: bool,
    #[serde(skip)]
    pub preview_playing: bool,
    #[serde(skip)]
    pub preview_time: f64,
    #[serde(skip)]
    preview_last_time: Option<f64>,
    #[serde(skip)]
    pending_apply: Option<f64>,
    #[serde(skip)]
    physics_token: Option<serde_json::Value>,
    #[serde(skip)]
    applied_project: usize,
    #[serde(skip)]
    applied_selection: Vec<effectcraft_engine::KeyRef>,
}

impl Default for CurrentState {
    fn default() -> Self {
        Self {
            mode: CurrentMode::Bezier,
            physics: Physics::default(),
            density: 2,
            compact: false,
            curve: Curve::DEFAULT.0,
            text: curve_text(Curve::DEFAULT.0),
            name: String::new(),
            lists: Vec::new(),
            selected_list: "Defaults".into(),
            list_name: String::new(),
            standard: Standard::default(),
            search: String::new(),
            export_all: false,
            export_flow: false,
            confirm_delete_list: false,
            message: String::new(),
            loaded: false,
            live_apply: false,
            preview_playing: false,
            preview_time: 0.0,
            preview_last_time: None,
            pending_apply: None,
            physics_token: None,
            applied_project: 0,
            applied_selection: Vec::new(),
        }
    }
}

fn curve_text(c: [f64; 4]) -> String {
    format!("cubic-bezier({}, {}, {}, {})", c[0], c[1], c[2], c[3])
}

pub fn parse_curve(text: &str) -> Result<[f64; 4], String> {
    if text.len() > 256 {
        return Err("Paste four cubic-Bezier values, not a complete stylesheet.".into());
    }
    let text = text.trim().trim_end_matches(';').trim();
    let text = text.strip_prefix("cubic-bezier(").and_then(|s| s.strip_suffix(')')).unwrap_or(text);
    let mut result = [0.0; 4];
    let mut values = text.split(',');
    for n in &mut result {
        *n = values.next().ok_or("Use four comma-separated values: x1, y1, x2, y2.")?.trim().parse::<f64>().map_err(|_| "Curve values must be numbers.")?;
    }
    if values.next().is_some() {
        return Err("Use exactly four curve values.".into());
    }
    Curve(result).for_apply().map(|_| result)
}

fn load_library(app: &mut EffectcraftApp, state: &mut CurrentState) {
    match app.session.execute("current.graphLists", json!({})) {
        Ok(result) => match serde_json::from_value(result["lists"].clone()) {
            Ok(lists) => {
                state.lists = lists;
                if state.selected_list != "Defaults" && !state.lists.iter().any(|l| l.name == state.selected_list) {
                    state.selected_list = "Defaults".into();
                }
            }
            Err(e) => state.message = format!("Cannot display graph lists: {e}"),
        },
        Err(e) => state.message = e.to_string(),
    }
    state.loaded = true;
}

fn current_config(state: &CurrentState) -> GraphConfig {
    GraphConfig {
        mode: match state.mode {
            CurrentMode::Bezier => GraphMode::Bezier,
            CurrentMode::Standard => GraphMode::Standard,
            CurrentMode::Spring => GraphMode::Spring,
            CurrentMode::Elastic => GraphMode::Elastic,
            CurrentMode::Bounce => GraphMode::Bounce,
        },
        curve: state.curve,
        frequency: state.physics.frequency,
        damping: state.physics.damping,
        amplitude: state.physics.amplitude,
        density: state.density,
        family: state.standard.family,
        direction: state.standard.direction,
    }
}
fn choose_graph(state: &mut CurrentState, graph: &Graph) {
    state.mode = match graph.config.mode {
        GraphMode::Bezier => CurrentMode::Bezier,
        GraphMode::Standard => CurrentMode::Standard,
        _ => CurrentMode::from_physics(graph.config.physics().mode),
    };
    state.curve = graph.config.curve;
    state.text = curve_text(state.curve);
    state.physics = graph.config.physics();
    state.density = graph.config.density;
    state.standard = Standard { family: graph.config.family, direction: graph.config.direction };
    state.name.clone_from(&graph.name);
}
fn library_command(app: &mut EffectcraftApp, state: &mut CurrentState, command: &str, params: serde_json::Value, message: &str) -> bool {
    match app.session.execute(command, params) {
        Ok(_) => {
            load_library(app, state);
            state.message = message.into();
            true
        }
        Err(e) => {
            state.message = e.to_string();
            false
        }
    }
}
fn library(app: &mut EffectcraftApp, ui: &mut egui::Ui, state: &mut CurrentState, ready: bool, valid: bool) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Graph lists").strong());
        let response = egui::ComboBox::from_id_salt("current.listSelector").selected_text(&state.selected_list).width(150.0).show_ui(ui, |ui| {
            for (index, name) in std::iter::once("Defaults").chain(state.lists.iter().map(|l| l.name.as_str())).enumerate() {
                let r = ui.selectable_value(&mut state.selected_list, name.to_string(), name);
                app.auto.add(&format!("current.list.{index}"), r.rect, name);
                if r.changed() {
                    state.search.clear();
                    state.confirm_delete_list = false;
                }
            }
        });
        app.auto.add("current.listSelector", response.response.rect, "Graph list");
        if button(app, ui, "current.import", "Import...", true) {
            match app.hooks.pick_files.as_ref() {
                Some(pick) => {
                    if let Some(path) = pick(&["current", "json", "flow"]).into_iter().next() {
                        match app.session.execute("current.importGraphLists", json!({"path":path})) {
                            Ok(result) => {
                                let last = result["imported"].as_array().and_then(|v| v.last()).and_then(|v| v.as_str()).map(str::to_string);
                                load_library(app, state);
                                if let Some(name) = last {
                                    state.selected_list = name;
                                }
                                state.message = "Graph lists imported. Existing lists were preserved.".into();
                            }
                            Err(e) => state.message = e.to_string(),
                        }
                    }
                }
                None => state.message = "Use current.importGraphLists with a JSON document in this host.".into(),
            }
        }
        if button(app, ui, "current.export", "Export...", true) {
            match app.hooks.pick_save_file.as_ref() {
                Some(pick) => {
                    if let Some(path) = pick(
                        if state.export_flow { "Current-curves.flow" } else { "Current-graph-lists.current" },
                        if state.export_flow { "flow" } else { "current" },
                    ) {
                        let mut params = json!({"path":path, "list":state.selected_list});
                        if state.export_flow {
                            params["format"] = json!("flow");
                        }
                        match app.session.execute("current.exportGraphLists", params) {
                            Ok(result) => {
                                state.message = if state.export_flow {
                                    let skipped = result["skipped"]
                                        .as_array()
                                        .map(|v| v.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                                        .unwrap_or_default();
                                    if skipped.is_empty() {
                                        format!("Exported {} Bezier graphs as Flow data.", result["exportedCount"])
                                    } else {
                                        format!(
                                            "Exported {} Bezier graphs; skipped {}: {skipped}. Use .current to retain every graph mode.",
                                            result["exportedCount"], result["skippedCount"]
                                        )
                                    }
                                } else {
                                    "Graph lists exported as .current JSON.".into()
                                }
                            }
                            Err(e) => state.message = e.to_string(),
                        }
                    }
                }
                None => state.message = "Use current.exportGraphLists to get Current JSON in this host.".into(),
            }
        }
        let r = ui
            .checkbox(&mut state.export_flow, "Flow (.flow)")
            .on_hover_text("Flow exports Bezier graphs and reports skipped modes. A .current file preserves every graph in the selected list.");
        app.auto.add("current.exportFlow", r.rect, "Export Flow Bezier data");
    });
    ui.horizontal_wrapped(|ui| {
        let r = ui.add(egui::TextEdit::singleline(&mut state.list_name).hint_text("List name").char_limit(80).desired_width(130.0));
        app.auto.add("current.listName", r.rect, "List name");
        if button(app, ui, "current.newList", "New list", !state.list_name.trim().is_empty()) {
            let name = state.list_name.trim().to_string();
            if library_command(app, state, "current.createGraphList", json!({"name":name}), "Custom graph list created.") {
                state.selected_list = name;
            }
        }
        let custom = state.selected_list != "Defaults";
        if button(app, ui, "current.renameList", "Rename", custom && !state.list_name.trim().is_empty()) {
            let name = state.list_name.trim().to_string();
            let params = json!({"list":state.selected_list,"name":name});
            if library_command(app, state, "current.renameGraphList", params, "Graph list renamed.") {
                state.selected_list = name;
            }
        }
        if button(app, ui, "current.deleteList", "Delete list...", custom) {
            state.confirm_delete_list = true;
        }
    });
    if state.confirm_delete_list {
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Delete {} and its saved graphs?", state.selected_list));
            if button(app, ui, "current.confirmDeleteList", "Delete list", true) {
                let params = json!({"list":state.selected_list});
                library_command(app, state, "current.deleteGraphList", params, "Graph list deleted.");
                state.confirm_delete_list = false;
            }
            if button(app, ui, "current.cancelDeleteList", "Cancel", true) {
                state.confirm_delete_list = false;
            }
        });
    }
    let r = ui.add(egui::TextEdit::singleline(&mut state.search).hint_text("Find a graph in this list").char_limit(80).desired_width(f32::INFINITY));
    app.auto.add("current.search", r.rect, "Find graph");
    let graphs = if state.selected_list == "Defaults" {
        default_graphs().graphs
    } else {
        state.lists.iter().find(|l| l.name == state.selected_list).map(|l| l.graphs.clone()).unwrap_or_default()
    };
    let filter = state.search.to_lowercase();
    let active = current_config(state);
    let mut apply = false;
    egui::ScrollArea::vertical().id_salt("current.graphListScroll").max_height(if state.compact { 130.0 } else { 180.0 }).show(ui, |ui| {
        if graphs.is_empty() {
            ui.label("This list is empty. Choose or edit a graph, then save it here.");
        }
        ui.horizontal_wrapped(|ui| {
            for (index, graph) in graphs.iter().enumerate().filter(|(_, g)| g.name.to_lowercase().contains(&filter)) {
                let id = if state.selected_list == "Defaults" { format!("current.preset.{index}") } else { format!("current.saved.{index}") };
                let r =
                    preset_card(app, ui, &id, &graph.name, graph.name == state.name && graph.config == active, state.compact, |time| graph.config.sample(time));
                if r.clicked() || r.double_clicked() {
                    choose_graph(state, graph);
                    changed = true;
                    apply |= r.double_clicked();
                }
            }
        });
    });
    if apply && ready && valid {
        apply_current(app, state);
    }
    let custom = state.selected_list != "Defaults";
    if !custom {
        ui.label(RichText::new("Defaults is read-only. Choose a custom list to save a copy.").small().color(app.tokens.text_dim));
    }
    let r = ui.add(egui::TextEdit::singleline(&mut state.name).hint_text("Graph name").char_limit(80).desired_width(f32::INFINITY));
    app.auto.add("current.name", r.rect, "Graph name");
    ui.horizontal_wrapped(|ui| {
        if button(app, ui, "current.save", "Save graph", custom && !state.name.trim().is_empty()) {
            let params = json!({"list":state.selected_list,"name":state.name,"config":current_config(state)});
            library_command(app, state, "current.saveGraph", params, "Graph saved in this list.");
        }
        let saved = custom && graphs.iter().any(|g| g.name.to_lowercase() == state.name.trim().to_lowercase());
        if button(app, ui, "current.delete", "Delete graph", saved) {
            let params = json!({"list":state.selected_list,"name":state.name});
            if library_command(app, state, "current.deleteGraph", params, "Saved graph deleted.") {
                state.name.clear();
            }
        }
    });
    changed
}

fn cubic(a: f64, b: f64, u: f64) -> f64 {
    3.0 * (1.0 - u).powi(2) * u * a + 3.0 * (1.0 - u) * u * u * b + u.powi(3)
}

/// Invert time, rather than treating the Bezier parameter as elapsed time.
fn bezier_progress(curve: [f64; 4], time: f64) -> f64 {
    let time = time.clamp(0.0, 1.0);
    let mut low = 0.0;
    let mut high = 1.0;
    for _ in 0..40 {
        let middle = (low + high) * 0.5;
        if cubic(curve[0], curve[2], middle) < time {
            low = middle;
        } else {
            high = middle;
        }
    }
    cubic(curve[1], curve[3], (low + high) * 0.5)
}

fn progress(state: &CurrentState, time: f64) -> f64 {
    match state.mode {
        CurrentMode::Bezier => bezier_progress(state.curve, time),
        CurrentMode::Standard => state.standard.sample(time),
        _ => state.physics.sample(time),
    }
}

fn apply_current(app: &mut EffectcraftApp, state: &mut CurrentState) {
    let physics = state.mode != CurrentMode::Bezier;
    let result = if physics {
        let mut params = if state.mode == CurrentMode::Standard {
            json!({"family":state.standard.family,"direction":state.standard.direction,"density":state.density})
        } else {
            json!({"mode":state.physics.mode,"frequency":state.physics.frequency,
            "damping":state.physics.damping,"amplitude":state.physics.amplitude,"density":state.density})
        };
        if let Some(token) = &state.physics_token {
            params["token"] = token.clone();
        }
        app.session.execute(if state.mode == CurrentMode::Standard { "keys.currentStandard" } else { "keys.currentPhysics" }, params)
    } else {
        app.session.execute("keys.currentApply", json!({"curve":state.curve}))
    };
    match result {
        Ok(result) => {
            state.physics_token = result.get("token").cloned();
            state.applied_project = std::sync::Arc::as_ptr(&app.session.project) as usize;
            state.applied_selection.clone_from(&app.session.state.selected_keys);
            state.message = if physics {
                format!("Baked {} keys across {} intervals. Undo restores the original keys.", result["generatedKeys"], result["segments"])
            } else {
                format!("Applied to {} intervals. Undo restores the previous easing.", result["segments"])
            };
            if let Some(notices) = result["notices"].as_array() {
                for notice in notices.iter().filter_map(|v| v.as_str()) {
                    state.message.push(' ');
                    state.message.push_str(notice);
                }
            }
        }
        Err(e) => {
            state.message = e.to_string();
            // A failed opt-in update must not repeat every repaint.
            state.live_apply = false;
            state.physics_token = None;
        }
    }
    state.pending_apply = None;
}

fn preset_card(
    app: &mut EffectcraftApp,
    ui: &mut egui::Ui,
    id: &str,
    name: &str,
    selected: bool,
    compact: bool,
    sample: impl Fn(f64) -> f64,
) -> egui::Response {
    let t = app.tokens;
    let size = vec2(if compact { 78.0 } else { 96.0 }, if compact { 54.0 } else { 70.0 });
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let response = response.on_hover_text(format!("{name}\nClick to preview. Double-click to apply to selected keys."));
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), selected, name));
    let paint = ui.painter().with_clip_rect(rect);
    paint.rect_filled(
        rect,
        3.0,
        if selected {
            t.row_selected
        } else if response.hovered() {
            t.hover
        } else {
            t.field_bg
        },
    );
    if selected || response.has_focus() {
        paint.rect_stroke(rect, 3.0, Stroke::new(1.0, t.accent), egui::StrokeKind::Inside);
    }
    let plot = Rect::from_min_max(rect.min + vec2(9.0, 7.0), rect.max - vec2(9.0, 22.0));
    let samples: Vec<_> = (0..=48).map(|i| (f64::from(i) / 48.0, sample(f64::from(i) / 48.0))).collect();
    let min = samples.iter().map(|(_, y)| *y).fold(0.0, f64::min);
    let max = samples.iter().map(|(_, y)| *y).fold(1.0, f64::max);
    paint.add(egui::Shape::line(
        samples
            .into_iter()
            .map(|(x, y)| pos2(plot.left() + x as f32 * plot.width(), plot.bottom() - ((y - min) / (max - min)) as f32 * plot.height()))
            .collect(),
        Stroke::new(1.5, if selected { t.accent } else { t.text_dim }),
    ));
    paint.text(pos2(rect.center().x, rect.bottom() - 5.0), Align2::CENTER_BOTTOM, name, Tokens::ui(10.0), t.text);
    app.auto.add(id, rect, name);
    response
}

fn motion_preview(app: &mut EffectcraftApp, ui: &mut egui::Ui, state: &mut CurrentState) {
    let now = ui.input(|i| i.time);
    if state.preview_playing {
        if let Some(last) = state.preview_last_time {
            state.preview_time = (state.preview_time + (now - last).clamp(0.0, 0.1) / 1.5).rem_euclid(1.0);
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    }
    state.preview_last_time = Some(now);
    ui.horizontal(|ui| {
        if button(app, ui, "current.previewPlay", if state.preview_playing { "Pause preview" } else { "Play preview" }, true) {
            state.preview_playing = !state.preview_playing;
        }
        if button(app, ui, "current.previewReset", "Reset", true) {
            state.preview_time = 0.0;
            state.preview_playing = false;
        }
        let r = ui.add(egui::Slider::new(&mut state.preview_time, 0.0..=1.0).show_value(false));
        app.auto.add("current.previewTime", r.rect, "Preview time");
    });
    let (rail, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
    let min = (0..=96).map(|i| progress(state, f64::from(i) / 96.0)).fold(0.0, f64::min) - 0.08;
    let max = (0..=96).map(|i| progress(state, f64::from(i) / 96.0)).fold(1.0, f64::max) + 0.08;
    let x = |value: f64| rail.left() + 8.0 + ((value - min) / (max - min)) as f32 * (rail.width() - 16.0).max(0.0);
    let paint = ui.painter();
    paint.line_segment([pos2(x(0.0), rail.center().y), pos2(x(1.0), rail.center().y)], Stroke::new(1.0, app.tokens.text_faint));
    for value in [0.0, 1.0] {
        paint.line_segment([pos2(x(value), rail.top() + 7.0), pos2(x(value), rail.bottom() - 7.0)], Stroke::new(1.0, app.tokens.text_dim));
    }
    paint.circle_filled(pos2(x(progress(state, state.preview_time)), rail.center().y), 5.0, app.tokens.accent);
    app.auto.add("current.motionPreview", rail, "Motion preview independent of the project playhead");
}

fn graph(app: &mut EffectcraftApp, ui: &mut egui::Ui, state: &mut CurrentState) -> bool {
    let t = app.tokens;
    let width = ui.available_width().max(80.0);
    let height = if state.compact { width.clamp(115.0, 155.0) } else { width.clamp(170.0, 245.0) };
    let (frame, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let plot = frame.shrink2(vec2(22.0, 20.0));
    if !plot.is_positive() {
        return false;
    }
    let values: Vec<_> = (0..=160).map(|i| progress(state, f64::from(i) / 160.0)).collect();
    let min_y = if state.mode == CurrentMode::Bezier { state.curve[1].min(state.curve[3]).min(0.0) } else { values.iter().copied().fold(0.0, f64::min) } - 0.12;
    let max_y = if state.mode == CurrentMode::Bezier { state.curve[1].max(state.curve[3]).max(1.0) } else { values.iter().copied().fold(1.0, f64::max) } + 0.12;
    let point = |x: f64, y: f64| pos2(plot.left() + x as f32 * plot.width(), plot.bottom() - ((y - min_y) / (max_y - min_y)) as f32 * plot.height());
    let paint = ui.painter().with_clip_rect(frame);
    paint.rect_filled(frame, 3.0, t.field_bg);
    for x in [0.0, 0.25, 0.5, 0.75, 1.0] {
        paint.line_segment([point(x, min_y), point(x, max_y)], Stroke::new(1.0, t.separator));
    }
    for y in [0.0, 0.25, 0.5, 0.75, 1.0] {
        paint.line_segment([point(0.0, y), point(1.0, y)], Stroke::new(1.0, t.separator));
    }
    let start = point(0.0, 0.0);
    let end = point(1.0, 1.0);
    paint.line_segment([start, end], Stroke::new(1.0, t.text_faint));
    let [x1, y1, x2, y2] = state.curve;
    paint.add(egui::Shape::line(
        (0..=160)
            .map(|i| {
                let u = f64::from(i) / 160.0;
                if state.mode == CurrentMode::Bezier { point(cubic(x1, x2, u), cubic(y1, y2, u)) } else { point(u, progress(state, u)) }
            })
            .collect(),
        Stroke::new(2.0, t.accent),
    ));
    paint.circle_filled(start, 3.0, t.text);
    paint.circle_filled(end, 3.0, t.text);
    paint.text(pos2(plot.center().x, frame.bottom() - 2.0), Align2::CENTER_BOTTOM, "Time", Tokens::ui(10.0), t.text_dim);
    if state.preview_playing || state.preview_time > 0.0 {
        paint.circle_filled(point(state.preview_time, progress(state, state.preview_time)), 4.0, t.text);
    }
    app.auto.add("current.graph", frame, "Normalized easing curve");
    if state.mode != CurrentMode::Bezier {
        return false;
    }
    let mut changed = false;
    for (index, anchor, id) in [(0, start, "current.handle1"), (2, end, "current.handle2")] {
        let x = state.curve.get(index).copied().unwrap_or(0.0);
        let y = state.curve.get(index + 1).copied().unwrap_or(0.0);
        let at = point(x, y);
        paint.line_segment([anchor, at], Stroke::new(1.0, t.text_dim));
        let hit = Rect::from_center_size(at, vec2(18.0, 18.0));
        let response = ui
            .interact(hit, ui.id().with(id), Sense::click_and_drag())
            .on_hover_text("Drag to shape easing. Shift snaps to 0.05; Alt mirrors the other handle.");
        app.auto.add(id, hit, if index == 0 { "Outgoing curve handle" } else { "Incoming curve handle" });
        paint.circle_filled(at, 5.0, if response.hovered() || response.dragged() { t.accent_hover } else { t.accent });
        paint.circle_stroke(at, 5.0, Stroke::new(1.0, t.text));
        if response.dragged()
            && let Some(mouse) = response.interact_pointer_pos()
        {
            let mut x = ((mouse.x - plot.left()) / plot.width()) as f64;
            let mut y = min_y + ((plot.bottom() - mouse.y) / plot.height()) as f64 * (max_y - min_y);
            if ui.input(|i| i.modifiers.shift) {
                x = (x * 20.0).round() / 20.0;
                y = (y * 20.0).round() / 20.0;
            }
            x = x.clamp(0.001, 0.999);
            y = y.clamp(-9.0, 10.0);
            if let Some(handle) = state.curve.get_mut(index..index + 2) {
                handle.copy_from_slice(&[x, y]);
                changed = true;
            }
            if ui.input(|i| i.modifiers.alt) {
                let other = if index == 0 { 2 } else { 0 };
                if let Some(handle) = state.curve.get_mut(other..other + 2) {
                    handle.copy_from_slice(&[1.0 - x, 1.0 - y]);
                }
            }
        }
    }
    changed
}

fn button(app: &mut EffectcraftApp, ui: &mut egui::Ui, id: &str, text: &str, enabled: bool) -> bool {
    let r = ui.add_enabled(enabled, egui::Button::new(text));
    app.auto.add(id, r.rect, text);
    r.clicked()
}

pub fn show(app: &mut EffectcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    let mut state = app.ui.current.clone();
    if !state.loaded {
        load_library(app, &mut state);
    }
    if (GraphConfig { curve: state.curve, ..Default::default() }).validate().is_err() {
        state.curve = Curve::DEFAULT.0;
        state.text = curve_text(state.curve);
        state.message = "Invalid saved curve values were reset. Choose or enter a valid curve.".into();
    }
    if state.physics.validate().is_err() {
        state.physics = Physics::default();
        state.message = "Invalid motion values were reset. Choose a motion preset or edit its controls.".into();
    }
    state.density = state.density.clamp(1, 8);
    if !state.preview_time.is_finite() {
        state.preview_time = 0.0;
    }
    if state.applied_project != std::sync::Arc::as_ptr(&app.session.project) as usize || state.applied_selection != app.session.state.selected_keys {
        state.physics_token = None;
        state.pending_apply = None;
    }
    if let Some(mode) = state.mode.physics() {
        state.physics.mode = mode;
    }
    let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(10.0)).layout(egui::Layout::top_down(egui::Align::Min)));
    egui::ScrollArea::vertical().id_salt("current.scroll").show(&mut ui, |ui| {
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            for (mode, label, id) in [
                (CurrentMode::Bezier, "Bezier", "current.mode.bezier"),
                (CurrentMode::Spring, "Spring", "current.mode.spring"),
                (CurrentMode::Elastic, "Elastic", "current.mode.elastic"),
                (CurrentMode::Bounce, "Bounce", "current.mode.bounce"),
                (CurrentMode::Standard, "Standard", "current.mode.standard"),
            ] {
                let r = ui.selectable_label(state.mode == mode, label);
                app.auto.add(id, r.rect, label);
                if r.clicked() {
                    state.mode = mode;
                    if let Some(mode) = mode.physics() {
                        state.physics.mode = mode;
                    }
                    state.name.clear();
                    changed = true;
                }
            }
            let r = ui.checkbox(&mut state.compact, "Compact");
            app.auto.add("current.compact", r.rect, "Compact layout");
        });
        changed |= graph(app, ui, &mut state);
        motion_preview(app, ui, &mut state);
        if state.mode == CurrentMode::Bezier {
            ui.horizontal_wrapped(|ui| {
                for (index, label) in [(0, "X1"), (1, "Y1"), (2, "X2"), (3, "Y2")] {
                    ui.label(label);
                    let range = if index % 2 == 0 { 0.0..=1.0 } else { -9.0..=10.0 };
                    if let Some(value) = state.curve.get_mut(index) {
                        let r = ui.add(crate::widgets::drag_value(egui::DragValue::new(value).range(range).speed(0.005).max_decimals(3)));
                        app.auto.add(&format!("current.{label}"), r.rect, label);
                        changed |= r.changed();
                    }
                }
            });
            if changed {
                state.text = curve_text(state.curve);
            }
            let r = ui.add(egui::TextEdit::singleline(&mut state.text).id_salt("current.values").desired_width(f32::INFINITY).char_limit(256));
            app.auto.add("current.values", r.rect, "Cubic Bezier values");
            ui.horizontal_wrapped(|ui| {
                if button(app, ui, "current.useValues", "Use values", true) {
                    match parse_curve(&state.text) {
                        Ok(curve) => {
                            state.curve = curve;
                            state.message.clear();
                            changed = true;
                        }
                        Err(e) => state.message = e,
                    }
                }
                if button(app, ui, "current.copy", "Copy", true) {
                    ui.ctx().copy_text(curve_text(state.curve));
                    state.message = "Curve copied as cubic-bezier values.".into();
                }
                if button(app, ui, "current.reverse", "Reverse", true) {
                    let [x1, y1, x2, y2] = state.curve;
                    state.curve = [1.0 - x2, 1.0 - y2, 1.0 - x1, 1.0 - y1];
                    state.text = curve_text(state.curve);
                    changed = true;
                }
            });
        } else {
            if state.mode == CurrentMode::Standard {
                ui.horizontal_wrapped(|ui| {
                    let r = egui::ComboBox::from_id_salt("current.standardFamily").selected_text(state.standard.family.label()).show_ui(ui, |ui| {
                        for family in StandardFamily::ALL {
                            let r = ui.selectable_value(&mut state.standard.family, family, family.label());
                            app.auto.add(&format!("current.family.{}", family.label().to_ascii_lowercase()), r.rect, family.label());
                            changed |= r.changed();
                        }
                    });
                    app.auto.add("current.standardFamily", r.response.rect, "Easing family");
                    for direction in Direction::ALL {
                        let r = ui.selectable_value(&mut state.standard.direction, direction, direction.label());
                        app.auto.add(
                            &format!(
                                "current.direction.{}",
                                match direction {
                                    Direction::In => "in",
                                    Direction::Out => "out",
                                    Direction::InOut => "inout",
                                }
                            ),
                            r.rect,
                            direction.label(),
                        );
                        changed |= r.changed();
                    }
                });
            }
            ui.horizontal_wrapped(|ui| {
                if state.mode != CurrentMode::Standard {
                    for (id, label, value, range) in [
                        ("current.frequency", "Frequency", &mut state.physics.frequency, 0.5..=12.0),
                        ("current.damping", "Damping", &mut state.physics.damping, 0.1..=20.0),
                        ("current.amplitude", "Amplitude", &mut state.physics.amplitude, 0.0..=3.0),
                    ] {
                        ui.label(label);
                        let r = ui.add(crate::widgets::drag_value(egui::DragValue::new(value).range(range).speed(0.02).max_decimals(2)));
                        app.auto.add(id, r.rect, label);
                        changed |= r.changed();
                    }
                }
                ui.label("Samples/frame");
                let r = ui.add(egui::DragValue::new(&mut state.density).range(1..=8));
                app.auto.add("current.density", r.rect, "Baked samples per composition frame");
                changed |= r.changed();
            });
            if !state.compact {
                ui.label(RichText::new("Creates intermediate keys at the composition frame rate. Endpoints stay in place.").small().color(t.text_dim));
            }
        }
        ui.add_space(6.0);
        let selected = app.session.state.selected_keys.len();
        let ready = selected >= 2;
        let valid = if state.mode == CurrentMode::Bezier { Curve(state.curve).for_apply().is_ok() } else { state.physics.validate().is_ok() };
        if state.mode == CurrentMode::Bezier
            && let Err(error) = Curve(state.curve).validate()
        {
            ui.label(RichText::new(error).small().color(t.text_dim));
        }
        let apply_label = if state.mode == CurrentMode::Bezier { "Apply to selected keys" } else { "Bake motion between selected keys" };
        let apply =
            ui.add_enabled(ready && valid, egui::Button::new(RichText::new(apply_label).strong()).fill(t.accent).min_size(vec2(ui.available_width(), 28.0)));
        app.auto.add("current.apply", apply.rect, apply_label);
        if apply.clicked() {
            apply_current(app, &mut state);
        }
        ui.horizontal_wrapped(|ui| {
            let r = ui.checkbox(&mut state.live_apply, "Live apply").on_hover_text(
                "Apply after each control gesture settles. Physics refinements reuse the same undo step. Preview playback never changes the project.",
            );
            app.auto.add("current.liveApply", r.rect, "Live apply after a control gesture");
            if r.changed() {
                // Enabling Live Apply is opt-in, but it is not itself a project edit.
                state.pending_apply = None;
            }
            if state.mode == CurrentMode::Bezier && button(app, ui, "current.read", "Read selected keys", ready) {
                match app.session.execute("keys.currentRead", json!({})) {
                    Ok(r) => match serde_json::from_value(r["curve"].clone()) {
                        Ok(curve) => {
                            state.curve = curve;
                            state.text = curve_text(curve);
                            state.message = "Read the first selected interval's first changing dimension.".into();
                        }
                        Err(e) => state.message = e.to_string(),
                    },
                    Err(e) => state.message = e.to_string(),
                }
            }
        });
        if !state.compact || !ready {
            ui.label(
                RichText::new(if ready {
                    format!("{selected} keys selected. Adjacent selected pairs only.")
                } else {
                    "Select two adjacent keyframes in the timeline.".into()
                })
                .small()
                .color(t.text_dim),
            );
        }
        ui.add_space(8.0);
        ui.separator();
        changed |= library(app, ui, &mut state, ready, valid);
        if state.live_apply && changed && ready && valid {
            state.pending_apply = Some(ui.input(|i| i.time));
            state.applied_project = std::sync::Arc::as_ptr(&app.session.project) as usize;
            state.applied_selection.clone_from(&app.session.state.selected_keys);
        }
        if state.live_apply
            && let Some(last) = state.pending_apply
        {
            let now = ui.input(|i| i.time);
            if now - last >= 0.25 && !ui.input(|i| i.pointer.any_down()) {
                apply_current(app, &mut state);
            } else {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
            }
        }
        if !state.message.is_empty() {
            ui.add_space(6.0);
            ui.label(RichText::new(&state.message).color(t.text));
        }
    });
    app.ui.current = state;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_values_accept_css_or_plain_text_and_reject_invalid_data() {
        assert_eq!(parse_curve("cubic-bezier(0.3, -0.2, 0.7, 1.2);").unwrap(), [0.3, -0.2, 0.7, 1.2]);
        assert_eq!(parse_curve("0.3,0,0.7,1").unwrap(), Curve::DEFAULT.0);
        let precise = [0.123456789, -0.123456789, 0.987654321, 1.23456789];
        assert_eq!(parse_curve(&curve_text(precise)).unwrap(), precise);
        for invalid in ["0.3,0,0.7", "0,0,1,1,2", "0.3,NaN,0.7,1", "1.2,0,0.7,1"] {
            assert!(parse_curve(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn preview_uses_elapsed_time_instead_of_the_bezier_parameter() {
        let curve = [0.78, 0.0, 0.22, 1.0];
        for u in [0.1, 0.3, 0.6, 0.9] {
            let time = cubic(curve[0], curve[2], u);
            let expected = cubic(curve[1], curve[3], u);
            assert!((bezier_progress(curve, time) - expected).abs() < 1e-10);
        }
    }
}
