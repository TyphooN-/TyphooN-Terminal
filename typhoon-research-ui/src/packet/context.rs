//! Read-only research context for the symbol-investigation packet sections
//! (ADR-125 Phase 1 step 3). Bundles the data a section needs — the research-DB
//! connection (acquired once, up the call stack) and the loaded fundamentals — so
//! a section can be a free function over the context instead of `&TyphooNApp`.
//!
//! Threading one connection here also removes the nested-`try_connection` hazard
//! (`read_conn` is a non-reentrant `try_lock`): a section uses `ctx.conn` instead
//! of re-acquiring the shared read lock, so it cannot silently no-op when an
//! ancestor already holds it.

use typhoon_engine::core::cache::Connection;

pub struct SymbolResearchContext<'a> {
    /// Research-DB read connection, acquired once by the dispatcher and threaded
    /// down so sections do not re-acquire `read_conn`.
    pub conn: &'a Connection,
    // Grows as more sections convert (e.g. `all_fundamentals` for the
    // fundamentals-driven sections, visible flags, command input).
}
