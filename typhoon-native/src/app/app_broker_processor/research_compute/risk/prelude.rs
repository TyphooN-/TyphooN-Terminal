//! Risk compute prelude seam for ADR-125 Target 3.
//!
//! Direct risk compute handlers import this local prelude instead of reaching
//! through their parent module.

pub(super) use crate::app::app_broker_processor::research_compute::prelude::*;
