//! The broker message protocol now lives in `typhoon_engine::broker::protocol` (ADR-127
//! Phase C). Re-exported so `state`'s re-export and the ~220/97 native call sites that
//! construct `BrokerCmd`/match `BrokerMsg` are unchanged.
pub(crate) use typhoon_engine::broker::protocol::*;
