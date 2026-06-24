//! Chart rendering layer now lives in the `typhoon-chart-ui` crate (ADR-125 Target 2,
//! slice 7b). Re-exported so the app glob (`use self::technical_analysis::*;`) and the bare
//! `draw_chart` / `format_price` / `parse_range` / `chart_overlay_company_name` call sites
//! resolve unchanged.
pub(crate) use typhoon_chart_ui::render::*;
