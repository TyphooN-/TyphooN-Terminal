//! Research unit tests, split from a former ~21.8k-line single file into
//! cohesive domain shards for readability (ADR-108). Sliced via `include!` to
//! preserve ONE test-module scope, because fixtures (synth_bars,
//! open_mem_conn*, mk_* builders) are shared across shards.

use super::*;

include!("setup_foundational_research.rs");
include!("fundamental_quality_research.rs");
include!("rank_event_research.rs");
include!("risk_distribution_research.rs");
include!("statistical_diagnostics_research.rs");
include!("technical_flow_research.rs");
include!("moving_average_research.rs");
include!("price_momentum_research.rs");
include!("candlestick_core_research.rs");
include!("candlestick_extended_research.rs");
include!("quant_statistical_test_research.rs");
