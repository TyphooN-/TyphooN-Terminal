//! Interaction shell for the "compute snapshot" research windows (ADR-125 Phase 1
//! step 3, input/action half).
//!
//! The per-window renderers share one shell: a symbol input + Use Chart / Load
//! Cached / Compute buttons + loading indicator, then the snapshot display. This
//! free function owns that shell. It is generic over the snapshot type `S` and the
//! action type `Cmd`, so it has no `TyphooNApp` or `BrokerCmd` coupling — the
//! per-window variation arrives as closures, and the Compute action is *returned*
//! (`Option<Cmd>`) instead of being sent inline. The caller threads its
//! `&mut self.<x>_win_*` fields in and sends the returned command. Crate-movable:
//! depends only on egui + the engine cache types.

use crate::app::common::{AXIS_TEXT, BTN_MG};
use typhoon_engine::core::cache::{Connection, SqliteCache};

/// Read-only context + presentation knobs for a compute window.
pub(super) struct ComputeWindow<'a> {
    pub title: &'a str,
    pub default_size: [f32; 2],
    /// Optional `.max_size(...)` constraint (most windows leave this `None`).
    pub max_size: Option<[f32; 2]>,
    /// Default symbol (from the active chart) used to seed an empty input and the
    /// "Use Chart" button.
    pub chart_symbol: &'a str,
    pub cache: Option<&'a SqliteCache>,
}

/// Render one compute window. Mutates the window's own state through the passed
/// references (visibility, symbol input, loading flag, cached snapshot) and returns
/// the Compute action to dispatch, if the user clicked Compute this frame.
///
/// - `load_cached`: given a DB connection + uppercased symbol, return a fresh
///   snapshot to display (the "Load Cached" button).
/// - `make_cmd`: build the compute command from the uppercased symbol.
/// - `render_snapshot`: the pure display body (see `render.rs`).
#[allow(clippy::too_many_arguments)]
pub(super) fn render_compute_window<S, Cmd>(
    ctx: &egui::Context,
    win: ComputeWindow,
    show: &mut bool,
    symbol: &mut String,
    loading: &mut bool,
    snapshot: &mut S,
    load_cached: impl FnOnce(&Connection, &str) -> Option<S>,
    make_cmd: impl FnOnce(String) -> Cmd,
    render_snapshot: impl FnOnce(&mut egui::Ui, &S),
) -> Option<Cmd> {
    render_compute_window_ext(
        ctx,
        win,
        show,
        symbol,
        loading,
        snapshot,
        |_| {},
        load_cached,
        make_cmd,
        render_snapshot,
    )
}

/// As [`render_compute_window`], plus `extra_controls`: a closure rendered in the
/// button row (between "Use Chart" and "Load Cached") for windows that expose an
/// extra parameter control (e.g. a `window_days` DragValue).
#[allow(clippy::too_many_arguments)]
pub(super) fn render_compute_window_ext<S, Cmd>(
    ctx: &egui::Context,
    win: ComputeWindow,
    show: &mut bool,
    symbol: &mut String,
    loading: &mut bool,
    snapshot: &mut S,
    extra_controls: impl FnOnce(&mut egui::Ui),
    load_cached: impl FnOnce(&Connection, &str) -> Option<S>,
    make_cmd: impl FnOnce(String) -> Cmd,
    render_snapshot: impl FnOnce(&mut egui::Ui, &S),
) -> Option<Cmd> {
    if !*show {
        return None;
    }
    if symbol.is_empty() {
        *symbol = win.chart_symbol.to_string();
    }
    let mut action: Option<Cmd> = None;
    let mut open = *show;
    let mut window = egui::Window::new(win.title)
        .open(&mut open)
        .resizable(true)
        .default_size(win.default_size);
    if let Some(max_size) = win.max_size {
        window = window.max_size(max_size);
    }
    window.show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
            ui.add(egui::TextEdit::singleline(symbol).desired_width(100.0));
            if ui.button("Use Chart").clicked() {
                *symbol = win.chart_symbol.to_string();
            }
            extra_controls(ui);
            if ui.button("Load Cached").clicked() {
                if let Some(cache) = win.cache {
                    if let Ok(conn) = cache.connection() {
                        let sym_u = symbol.to_uppercase();
                        if let Some(snap) = load_cached(&conn, &sym_u) {
                            *snapshot = snap;
                            *symbol = sym_u;
                        }
                    }
                }
            }
            if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                let sym = symbol.to_uppercase();
                *loading = true;
                *symbol = sym.clone();
                action = Some(make_cmd(sym));
            }
            if *loading {
                ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
            }
        });
        render_snapshot(ui, snapshot);
    });
    *show = open;
    action
}
