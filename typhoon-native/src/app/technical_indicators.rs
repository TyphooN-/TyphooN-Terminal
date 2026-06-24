//! Indicator math now lives in the `typhoon-chart-ui` crate (ADR-125 Target 2, slice 2).
//! Re-exported so the `use …::technical_indicators::*;` glob in `app` / `technical_analysis`
//! and the bare `compute_*` call sites resolve unchanged.
pub(crate) use typhoon_chart_ui::indicators::*;
