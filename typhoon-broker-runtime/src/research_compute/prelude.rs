//! Research-compute prelude seam for ADR-125 Target 3.
//!
//! First-level research compute handlers import this local prelude instead of
//! reaching through their parent module. Nested risk/technical children keep
//! their domain-parent imports until those subtrees get their own seams.

pub(super) use super::{breakout, technical_indicators};
pub(super) use crate::prelude::*;
