//! Native compatibility shim for chart source cache-key generation.
//!
//! The implementation now lives in `typhoon-chart-ui` so the future broker
//! runtime can use it without depending on `typhoon-native`.

pub(crate) use typhoon_chart_ui::cache_keys::chart_source_cache_keys;
