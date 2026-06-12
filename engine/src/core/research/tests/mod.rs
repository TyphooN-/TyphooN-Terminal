//! Research unit tests, split from a former ~21.8k-line single file by
//! development round for readability (ADR-108). Sliced via `include!` to
//! preserve ONE test-module scope, because fixtures (synth_bars,
//! open_mem_conn*, mk_* builders) are shared across rounds.

use super::*;

include!("setup_rounds_01_09.rs");
include!("rounds_10_15.rs");
include!("rounds_16_21.rs");
include!("rounds_22_29.rs");
include!("rounds_30_40.rs");
include!("rounds_41_50.rs");
include!("rounds_51_53.rs");
include!("rounds_60_71.rs");
include!("candlestick_72_78.rs");
include!("candlestick_79_88.rs");
include!("quant_stats_76_77.rs");
