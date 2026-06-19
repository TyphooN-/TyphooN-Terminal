//! Native app unit tests, split from a former ~3.5k-line single file by
//! area for readability (ADR-086). Sliced via `include!` to keep ONE
//! test-module scope (shared fixtures: test_bar, make_bars,
//! make_oscillating_bars, make_close_bars, sample_events).

use super::*;

include!("source_and_commands.rs");
include!("indicators.rs");
include!("chart_features.rs");
