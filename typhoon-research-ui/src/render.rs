//! Pure snapshot-display renderers for the research floating windows (ADR-125
//! Phase 1 step 3): the per-window display bodies, free functions over
//! `(&mut egui::Ui, &Snapshot)` with no `TyphooNApp` access — the egui analog of
//! the symbol-investigation packet's formatter layer. Crate-movable: the future
//! `typhoon-research-ui` crate may depend on egui directly.

// Split into order-preserving segment modules (sNN_<first-renderer>) so a
// 23k-line single file doesn't dominate edit/compile locality; every
// renderer stays addressable as render::render_*. Segment boundaries are
// mechanical (32-33 fns each), not semantic.
mod s01_avgprice;
mod s02_ht_trendmode;
mod s03_rainbow;
mod s04_calmar;
mod s05_fqmrank;
mod s06_lyapunov;
mod s07_relvol;
mod s08_updm;

pub use s01_avgprice::*;
pub use s02_ht_trendmode::*;
pub use s03_rainbow::*;
pub use s04_calmar::*;
pub use s05_fqmrank::*;
pub use s06_lyapunov::*;
pub use s07_relvol::*;
pub use s08_updm::*;
