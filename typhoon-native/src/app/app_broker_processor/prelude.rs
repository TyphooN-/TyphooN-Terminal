//! Broker processor prelude seam for ADR-125 Target 3.
//!
//! Direct broker-processor children import this module instead of globbing the
//! native app parent. Keeping the app-facing surface centralized makes the
//! future broker-runtime crate extraction a mechanical dependency-audit step.

pub(super) use crate::app::*;
