//! Operator intervention log and deterministic hybrid replay (ADR-135 §6.13).
//!
//! A *hybrid* run is one where an automated strategy runs but an operator also
//! acts — overriding an entry, pulling a stop, closing early. Such a run is
//! only evidence of anything if it can be replayed: otherwise its ledger is an
//! anecdote about a session nobody can reconstruct.
//!
//! This module records every operator action against the decision it happened
//! at, seals the record content-addressably, and replays it. The M2 gate is
//! that replaying a recorded hybrid run reproduces its ledger bit for bit.
//!
//! # Why decision index, not wall-clock time
//!
//! An intervention is anchored to the *decision ordinal* the simulator was on
//! when it happened, not to a timestamp. The simulator's decision sequence is
//! already the deterministic, seed-derived spine of a run (§6.10); replaying
//! against it needs no clock and cannot drift. A wall-clock anchor would have
//! to be re-resolved to a decision on replay, and any rounding there is a
//! divergence.
//!
//! # What replay does not do
//!
//! Replay does not re-ask the operator anything, and it does not merge new
//! automated behaviour with an old log. The automated strategy is re-run
//! exactly as recorded and the log is applied on top; if the strategy's code
//! has changed since, the run id changes with it and the two ledgers are not
//! expected to match.

use crate::core::strategy_simulator::{
    ClientOrderId, DecisionContext, ModifyRequest, OrderIntents, OrderRequest, ReferenceStrategy,
    StrategyError,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Wire-format version of [`InterventionLog`].
pub const INTERVENTION_LOG_SCHEMA_VERSION: u32 = 1;

/// Domain-separation prefix for the log's content address.
const INTERVENTION_LOG_DOMAIN: &[u8] = b"typhoon.strategy_intervention.log_id.v1";

/// Largest number of interventions one run may record.
pub const MAX_INTERVENTIONS: usize = 65_536;

/// Largest encoded log accepted by the loading API.
pub const MAX_INTERVENTION_LOG_JSON_BYTES: usize = 8 * 1024 * 1024;

/// What an operator did. These are exactly the three things a strategy can do,
/// because an operator acting through the same order interface is the only way
/// their action can be modelled with the same fidelity as an automated one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum InterventionAction {
    Submit {
        request: OrderRequest,
    },
    Cancel {
        target: ClientOrderId,
    },
    Modify {
        target: ClientOrderId,
        change: ModifyRequest,
    },
}

/// One operator action, anchored to the decision it interrupted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Intervention {
    /// Zero-based ordinal of the decision the simulator was making. Several
    /// interventions may share one index; they apply in recorded order.
    pub decision_index: u64,
    /// Free-text reason, carried so a replayed ledger can be read back with the
    /// operator's own justification attached rather than as anonymous orders.
    pub note: String,
    pub action: InterventionAction,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InterventionLog {
    schema_version: u32,
    log_id: String,
    interventions: Vec<Intervention>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InterventionLogWire {
    schema_version: u32,
    log_id: String,
    interventions: Vec<Intervention>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InterventionError {
    TooMany { limit: usize, found: usize },
    TooLarge { limit: usize, found: usize },
    Unordered { at: usize },
    NoteTooLong { at: usize },
    InvalidJson { message: String },
    UnsupportedSchemaVersion { found: u32, supported: u32 },
    IdentityMismatch { expected: String, actual: String },
}

impl std::fmt::Display for InterventionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "invalid intervention log: {self:?}")
    }
}

impl std::error::Error for InterventionError {}

const MAX_NOTE_LEN: usize = 512;

impl InterventionLog {
    /// Seals a recorded session. Interventions must already be in the order
    /// they happened; a log that is out of order is rejected rather than
    /// sorted, because reordering two actions on the same decision changes what
    /// the run did.
    pub fn build(interventions: Vec<Intervention>) -> Result<Self, InterventionError> {
        if interventions.len() > MAX_INTERVENTIONS {
            return Err(InterventionError::TooMany {
                limit: MAX_INTERVENTIONS,
                found: interventions.len(),
            });
        }
        for (index, pair) in interventions.windows(2).enumerate() {
            if pair[0].decision_index > pair[1].decision_index {
                return Err(InterventionError::Unordered { at: index + 1 });
            }
        }
        if let Some(at) = interventions
            .iter()
            .position(|entry| entry.note.len() > MAX_NOTE_LEN)
        {
            return Err(InterventionError::NoteTooLong { at });
        }
        let log_id = compute_log_id(&interventions);
        Ok(Self {
            schema_version: INTERVENTION_LOG_SCHEMA_VERSION,
            log_id,
            interventions,
        })
    }

    pub fn empty() -> Self {
        Self::build(Vec::new()).expect("an empty log is always valid")
    }

    /// The content address that [`crate::core::strategy_ir::RunBinding`]'s
    /// `intervention_log_id` refers to.
    pub fn log_id(&self) -> &str {
        &self.log_id
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn interventions(&self) -> &[Intervention] {
        &self.interventions
    }

    pub fn is_empty(&self) -> bool {
        self.interventions.is_empty()
    }

    pub fn to_json_vec(&self) -> Result<Vec<u8>, InterventionError> {
        let bytes = serde_json::to_vec(self).map_err(invalid_json)?;
        if bytes.len() > MAX_INTERVENTION_LOG_JSON_BYTES {
            return Err(InterventionError::TooLarge {
                limit: MAX_INTERVENTION_LOG_JSON_BYTES,
                found: bytes.len(),
            });
        }
        Ok(bytes)
    }

    /// Loads a sealed log, bounded before decode and re-verified after.
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, InterventionError> {
        if bytes.len() > MAX_INTERVENTION_LOG_JSON_BYTES {
            return Err(InterventionError::TooLarge {
                limit: MAX_INTERVENTION_LOG_JSON_BYTES,
                found: bytes.len(),
            });
        }
        let wire: InterventionLogWire = serde_json::from_slice(bytes).map_err(invalid_json)?;
        if wire.schema_version != INTERVENTION_LOG_SCHEMA_VERSION {
            return Err(InterventionError::UnsupportedSchemaVersion {
                found: wire.schema_version,
                supported: INTERVENTION_LOG_SCHEMA_VERSION,
            });
        }
        let rebuilt = Self::build(wire.interventions)?;
        if rebuilt.log_id != wire.log_id {
            return Err(InterventionError::IdentityMismatch {
                expected: wire.log_id,
                actual: rebuilt.log_id,
            });
        }
        Ok(rebuilt)
    }
}

fn compute_log_id(interventions: &[Intervention]) -> String {
    // Each field is length-framed so no two different logs can produce the same
    // byte stream by running their fields together.
    let mut hasher = Sha256::new();
    frame(&mut hasher, INTERVENTION_LOG_DOMAIN);
    frame(&mut hasher, &INTERVENTION_LOG_SCHEMA_VERSION.to_be_bytes());
    frame(&mut hasher, &(interventions.len() as u64).to_be_bytes());
    for entry in interventions {
        frame(&mut hasher, &entry.decision_index.to_be_bytes());
        frame(&mut hasher, entry.note.as_bytes());
        let encoded = serde_json::to_vec(&entry.action).unwrap_or_default();
        frame(&mut hasher, &encoded);
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn frame(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn invalid_json(error: serde_json::Error) -> InterventionError {
    InterventionError::InvalidJson {
        message: error.to_string(),
    }
}

/// Records what an operator does during a live hybrid session.
///
/// Wraps the automated strategy so the two act through one interface and the
/// simulator cannot tell them apart — which is the point: an intervention that
/// executed differently from an automated order would not be replayable.
pub struct HybridRecorder<'a> {
    automated: &'a mut dyn ReferenceStrategy,
    operator: Box<dyn FnMut(u64, &DecisionContext<'_>) -> Vec<Intervention> + 'a>,
    decisions: u64,
    recorded: Vec<Intervention>,
}

impl<'a> HybridRecorder<'a> {
    /// `operator` is asked, at every decision, what the human did. In a live
    /// session it drains a UI queue; in a test it is a script.
    pub fn new(
        automated: &'a mut dyn ReferenceStrategy,
        operator: impl FnMut(u64, &DecisionContext<'_>) -> Vec<Intervention> + 'a,
    ) -> Self {
        Self {
            automated,
            operator: Box::new(operator),
            decisions: 0,
            recorded: Vec::new(),
        }
    }

    /// Seals what was recorded. Consumes the recorder so a log cannot be taken
    /// mid-session and then silently extended.
    pub fn into_log(self) -> Result<InterventionLog, InterventionError> {
        InterventionLog::build(self.recorded)
    }
}

impl ReferenceStrategy for HybridRecorder<'_> {
    fn on_bar_close(
        &mut self,
        ctx: &DecisionContext<'_>,
        orders: &mut OrderIntents,
    ) -> Result<(), StrategyError> {
        let index = self.decisions;
        self.decisions += 1;
        // The automated strategy always goes first, so an operator acting on
        // the same decision is overriding a decision that has already been
        // expressed — which is what replay must reproduce.
        self.automated.on_bar_close(ctx, orders)?;
        for intervention in (self.operator)(index, ctx) {
            apply(&intervention, orders)?;
            if self.recorded.len() < MAX_INTERVENTIONS {
                self.recorded.push(intervention);
            }
        }
        Ok(())
    }
}

/// Re-runs an automated strategy with a sealed log applied on top.
///
/// Given the same strategy, streams, config and setup, this produces a ledger
/// byte-identical to the session the log came from.
pub struct HybridReplay<'a> {
    automated: &'a mut dyn ReferenceStrategy,
    log: &'a InterventionLog,
    decisions: u64,
    /// Index of the first intervention not yet applied. The log is ordered by
    /// decision index, so replay is a single forward pass.
    cursor: usize,
}

impl<'a> HybridReplay<'a> {
    pub fn new(automated: &'a mut dyn ReferenceStrategy, log: &'a InterventionLog) -> Self {
        Self {
            automated,
            log,
            decisions: 0,
            cursor: 0,
        }
    }

    /// Interventions consumed so far. After a complete replay this equals the
    /// log length; anything less means the run ended before the log did.
    pub fn applied(&self) -> usize {
        self.cursor
    }
}

impl ReferenceStrategy for HybridReplay<'_> {
    fn on_bar_close(
        &mut self,
        ctx: &DecisionContext<'_>,
        orders: &mut OrderIntents,
    ) -> Result<(), StrategyError> {
        let index = self.decisions;
        self.decisions += 1;
        self.automated.on_bar_close(ctx, orders)?;
        let entries = self.log.interventions();
        while let Some(entry) = entries.get(self.cursor) {
            if entry.decision_index != index {
                break;
            }
            apply(entry, orders)?;
            self.cursor += 1;
        }
        Ok(())
    }
}

fn apply(intervention: &Intervention, orders: &mut OrderIntents) -> Result<(), StrategyError> {
    match &intervention.action {
        InterventionAction::Submit { request } => {
            orders.submit(request.clone())?;
        }
        InterventionAction::Cancel { target } => orders.cancel(*target)?,
        InterventionAction::Modify { target, change } => orders.modify(*target, change.clone())?,
    }
    Ok(())
}

#[cfg(test)]
mod tests;
