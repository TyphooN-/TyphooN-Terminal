//! Sealed, bounded report artifact over a verified run and its M1 simulator ledger.
//!
//! Report ownership lives here. `strategy_metrics` owns metric definitions and computation;
//! this module owns persistence, identity, and replay verification.

use crate::core::strategy_ir::StrategyRunManifest;
use crate::core::strategy_metrics::{
    METRICS_SCHEMA_VERSION, MetricsError, StrategyAnalysis, analyze_simulation, metric_registry,
};
use crate::core::strategy_simulator::{SimulationReport, SymbolStream};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt;

pub const REPORT_ARTIFACT_SCHEMA_VERSION: u32 = 1;
pub const MAX_REPORT_ARTIFACT_JSON_BYTES: usize = 16 * 1024 * 1024;
const MAX_REPORT_COLLECTION_ITEMS: usize = 1_000_000;
const REPORT_ID_DOMAIN: &[u8] = b"typhoon.strategy_report.report_id.v1";
const SIMULATOR_REPORT_DOMAIN: &[u8] = b"typhoon.strategy_report.simulator_report.v1";
const SIMULATOR_EVENT_DOMAIN: &[u8] = b"typhoon.strategy_report.simulator_events.v1";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StrategyReportArtifact {
    schema_version: u32,
    report_id: String,
    run_id: String,
    metrics_version: String,
    simulator_report_digest: String,
    simulator_event_digest: String,
    analysis: StrategyAnalysis,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportWire {
    schema_version: u32,
    report_id: String,
    run_id: String,
    metrics_version: String,
    simulator_report_digest: String,
    simulator_event_digest: String,
    analysis: StrategyAnalysis,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReportArtifactError {
    TooLarge { limit: usize, found: usize },
    InvalidJson { message: String },
    InvalidRunManifest { message: String },
    Metrics(MetricsError),
    UnsupportedSchemaVersion { found: u32, supported: u32 },
    UnsupportedMetricsVersion { found: String },
    InvalidStructure { field: &'static str },
    IdentityMismatch { expected: String, actual: String },
    RunIdMismatch { expected: String, actual: String },
    SimulatorDigestMismatch { expected: String, actual: String },
    EventDigestMismatch { expected: String, actual: String },
}

impl fmt::Display for ReportArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid strategy report artifact: {self:?}")
    }
}

impl Error for ReportArtifactError {}

impl StrategyReportArtifact {
    pub fn build(
        manifest: &StrategyRunManifest,
        simulator_report: &SimulationReport,
        streams: &[SymbolStream],
        initial_equity: f64,
    ) -> Result<Self, ReportArtifactError> {
        verify_manifest(manifest)?;
        let analysis = analyze_simulation(simulator_report, streams, initial_equity)
            .map_err(ReportArtifactError::Metrics)?;
        // Seal the exact JSON-wire interpretation that persisted artifacts will
        // reload. This prevents a platform parser's one-ULP decimal conversion
        // from making a freshly built artifact unverifiable after round-trip.
        let analysis = canonical_wire_analysis(&analysis)?;
        let mut artifact = Self {
            schema_version: REPORT_ARTIFACT_SCHEMA_VERSION,
            report_id: String::new(),
            run_id: manifest.run_id().to_string(),
            metrics_version: METRICS_SCHEMA_VERSION.to_string(),
            simulator_report_digest: digest_json(SIMULATOR_REPORT_DOMAIN, simulator_report)?,
            simulator_event_digest: digest_json(SIMULATOR_EVENT_DOMAIN, &simulator_report.events)?,
            analysis,
        };
        // `compute_report_id` deliberately excludes `report_id`, so sealing first
        // lets `validate_structure` demand a real digest in every id field
        // instead of carving out an "still being built" exemption that a loaded
        // artifact could also slip through.
        artifact.report_id = artifact.compute_report_id()?;
        artifact.validate_structure()?;
        Ok(artifact)
    }

    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, ReportArtifactError> {
        if bytes.len() > MAX_REPORT_ARTIFACT_JSON_BYTES {
            return Err(ReportArtifactError::TooLarge {
                limit: MAX_REPORT_ARTIFACT_JSON_BYTES,
                found: bytes.len(),
            });
        }
        let wire: ReportWire = serde_json::from_slice(bytes).map_err(invalid_json)?;
        let artifact = Self {
            schema_version: wire.schema_version,
            report_id: wire.report_id,
            run_id: wire.run_id,
            metrics_version: wire.metrics_version,
            simulator_report_digest: wire.simulator_report_digest,
            simulator_event_digest: wire.simulator_event_digest,
            analysis: wire.analysis,
        };
        artifact.verify()?;
        Ok(artifact)
    }

    pub fn to_json_vec(&self) -> Result<Vec<u8>, ReportArtifactError> {
        self.verify()?;
        let bytes = serde_json::to_vec(self).map_err(invalid_json)?;
        if bytes.len() > MAX_REPORT_ARTIFACT_JSON_BYTES {
            return Err(ReportArtifactError::TooLarge {
                limit: MAX_REPORT_ARTIFACT_JSON_BYTES,
                found: bytes.len(),
            });
        }
        Ok(bytes)
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn report_id(&self) -> &str {
        &self.report_id
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn metrics_version(&self) -> &str {
        &self.metrics_version
    }

    pub fn simulator_report_digest(&self) -> &str {
        &self.simulator_report_digest
    }

    pub fn simulator_event_digest(&self) -> &str {
        &self.simulator_event_digest
    }

    pub fn analysis(&self) -> &StrategyAnalysis {
        &self.analysis
    }

    pub fn verify(&self) -> Result<(), ReportArtifactError> {
        if self.schema_version != REPORT_ARTIFACT_SCHEMA_VERSION {
            return Err(ReportArtifactError::UnsupportedSchemaVersion {
                found: self.schema_version,
                supported: REPORT_ARTIFACT_SCHEMA_VERSION,
            });
        }
        if self.metrics_version != METRICS_SCHEMA_VERSION
            || self.analysis.metrics_version != METRICS_SCHEMA_VERSION
        {
            return Err(ReportArtifactError::UnsupportedMetricsVersion {
                found: self.metrics_version.clone(),
            });
        }
        self.validate_structure()?;
        let actual = self.compute_report_id()?;
        if actual != self.report_id {
            return Err(ReportArtifactError::IdentityMismatch {
                expected: self.report_id.clone(),
                actual,
            });
        }
        Ok(())
    }

    /// Verify that this sealed artifact and a completed simulation report are
    /// the exact identity-bound pair. Native presentation can call this after
    /// the run worker has already verified the manifest; it does not need to
    /// rebuild or scan the run inputs just to consume the result.
    pub fn verify_simulation_report(
        &self,
        simulator_report: &SimulationReport,
    ) -> Result<(), ReportArtifactError> {
        self.verify()?;
        let report_digest = digest_json(SIMULATOR_REPORT_DOMAIN, simulator_report)?;
        if report_digest != self.simulator_report_digest {
            return Err(ReportArtifactError::SimulatorDigestMismatch {
                expected: self.simulator_report_digest.clone(),
                actual: report_digest,
            });
        }
        let event_digest = digest_json(SIMULATOR_EVENT_DOMAIN, &simulator_report.events)?;
        if event_digest != self.simulator_event_digest {
            return Err(ReportArtifactError::EventDigestMismatch {
                expected: self.simulator_event_digest.clone(),
                actual: event_digest,
            });
        }
        Ok(())
    }

    pub fn verify_against(
        &self,
        manifest: &StrategyRunManifest,
        simulator_report: &SimulationReport,
    ) -> Result<(), ReportArtifactError> {
        self.verify_simulation_report(simulator_report)?;
        verify_manifest(manifest)?;
        if manifest.run_id() != self.run_id {
            return Err(ReportArtifactError::RunIdMismatch {
                expected: self.run_id.clone(),
                actual: manifest.run_id().to_string(),
            });
        }
        Ok(())
    }

    fn validate_structure(&self) -> Result<(), ReportArtifactError> {
        for (field, digest) in [
            ("report_id", &self.report_id),
            ("run_id", &self.run_id),
            ("simulator_report_digest", &self.simulator_report_digest),
            ("simulator_event_digest", &self.simulator_event_digest),
        ] {
            if !is_lowercase_sha256(digest) {
                return Err(ReportArtifactError::InvalidStructure { field });
            }
        }
        let lengths = [
            ("analysis.metrics", self.analysis.metrics.len()),
            ("analysis.trades", self.analysis.trades.len()),
            (
                "analysis.underwater_curve",
                self.analysis.underwater_curve.len(),
            ),
            (
                "analysis.calendar.daily",
                self.analysis.calendar.daily.len(),
            ),
        ];
        if let Some((field, _)) = lengths
            .into_iter()
            .find(|(_, len)| *len > MAX_REPORT_COLLECTION_ITEMS)
        {
            return Err(ReportArtifactError::InvalidStructure { field });
        }
        let registry = metric_registry();
        if self.analysis.metrics.len() != registry.len()
            || self
                .analysis
                .metrics
                .iter()
                .zip(registry)
                .any(|(actual, expected)| actual.id != expected.id)
        {
            return Err(ReportArtifactError::InvalidStructure {
                field: "analysis.metrics",
            });
        }
        Ok(())
    }

    fn compute_report_id(&self) -> Result<String, ReportArtifactError> {
        // The schema fixes field order. Every component is separately framed and domain-separated;
        // report_id itself is deliberately excluded.
        let analysis = serde_json::to_vec(&self.analysis).map_err(invalid_json)?;
        let mut hasher = Sha256::new();
        hash_frame(&mut hasher, REPORT_ID_DOMAIN);
        hash_frame(&mut hasher, &self.schema_version.to_be_bytes());
        hash_frame(&mut hasher, self.run_id.as_bytes());
        hash_frame(&mut hasher, self.metrics_version.as_bytes());
        hash_frame(&mut hasher, self.simulator_report_digest.as_bytes());
        hash_frame(&mut hasher, self.simulator_event_digest.as_bytes());
        hash_frame(&mut hasher, &analysis);
        Ok(hex_digest(&hasher.finalize()))
    }
}

fn verify_manifest(manifest: &StrategyRunManifest) -> Result<(), ReportArtifactError> {
    manifest
        .verify()
        .map_err(|error| ReportArtifactError::InvalidRunManifest {
            message: error.to_string(),
        })?;
    if manifest.binding().metrics_version != METRICS_SCHEMA_VERSION {
        return Err(ReportArtifactError::UnsupportedMetricsVersion {
            found: manifest.binding().metrics_version.clone(),
        });
    }
    Ok(())
}

fn digest_json<T: Serialize + ?Sized>(
    domain: &[u8],
    value: &T,
) -> Result<String, ReportArtifactError> {
    let bytes = serde_json::to_vec(value).map_err(invalid_json)?;
    let mut hasher = Sha256::new();
    hash_frame(&mut hasher, domain);
    hash_frame(&mut hasher, &bytes);
    Ok(hex_digest(&hasher.finalize()))
}

fn canonical_wire_analysis(
    analysis: &StrategyAnalysis,
) -> Result<StrategyAnalysis, ReportArtifactError> {
    let bytes = serde_json::to_vec(analysis).map_err(invalid_json)?;
    serde_json::from_slice(&bytes).map_err(invalid_json)
}

fn invalid_json(error: serde_json::Error) -> ReportArtifactError {
    ReportArtifactError::InvalidJson {
        message: error.to_string(),
    }
}

fn hash_frame(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_REPORT_ARTIFACT_JSON_BYTES, REPORT_ARTIFACT_SCHEMA_VERSION, ReportArtifactError,
        StrategyReportArtifact,
    };
    use crate::core::strategy_ir::{
        DatasetBinding, RepaintAcknowledgement, RepaintQaBinding, RunBinding, StrategyRunManifest,
    };
    use crate::core::strategy_metrics::METRICS_SCHEMA_VERSION;
    use crate::core::strategy_metrics::tests::report;

    const DAY_NS: i64 = 86_400_000_000_000;

    fn manifest() -> StrategyRunManifest {
        StrategyRunManifest::build(&RunBinding {
            datasets: vec![DatasetBinding {
                input_id: "primary".to_string(),
                dataset_id: "a".repeat(64),
            }],
            strategy_id: "b".repeat(64),
            config_id: "c".repeat(64),
            seed: 7,
            engine_version: "typhoon-engine/test".to_string(),
            metrics_version: METRICS_SCHEMA_VERSION.to_string(),
            intervention_log_id: None,
            repaint_qa: vec![],
        })
        .expect("manifest")
    }

    #[test]
    fn report_artifact_round_trips_detects_tampering_and_rejects_replay_mismatch() {
        let manifest = manifest();
        let simulator = report(vec![], &[(0, 1_000.0), (DAY_NS + 1, 1_001.0)]);
        let artifact = StrategyReportArtifact::build(&manifest, &simulator, &[], 1_000.0)
            .expect("artifact builds");
        assert_eq!(artifact.schema_version(), REPORT_ARTIFACT_SCHEMA_VERSION);
        assert_eq!(artifact.run_id(), manifest.run_id());
        assert_eq!(artifact.metrics_version(), METRICS_SCHEMA_VERSION);

        // Identity is a pure function of the sealed inputs, not of build order.
        let repeated = StrategyReportArtifact::build(&manifest, &simulator, &[], 1_000.0)
            .expect("repeat builds");
        assert_eq!(repeated.report_id(), artifact.report_id());
        assert_eq!(repeated, artifact);

        let json = artifact.to_json_vec().expect("serializes");
        let restored = StrategyReportArtifact::from_json_slice(&json).expect("round trip");
        assert_eq!(restored, artifact);
        assert_eq!(restored.analysis(), artifact.analysis());
        restored
            .verify_against(&manifest, &simulator)
            .expect("replay verifies");
        restored
            .verify_simulation_report(&simulator)
            .expect("completed simulation output verifies without rebuilding the manifest");

        let mut tampered: serde_json::Value = serde_json::from_slice(&json).expect("json");
        tampered["analysis"]["metrics"][0]["value"]["value"] = serde_json::json!(99.0);
        assert!(matches!(
            StrategyReportArtifact::from_json_slice(
                &serde_json::to_vec(&tampered).expect("tampered json")
            ),
            Err(ReportArtifactError::IdentityMismatch { .. })
        ));

        // A replay whose ledger differs is rejected even though the report itself
        // is internally consistent.
        let replay = report(vec![], &[(0, 1_000.0), (DAY_NS + 1, 999.0)]);
        assert!(matches!(
            artifact.verify_against(&manifest, &replay),
            Err(ReportArtifactError::SimulatorDigestMismatch { .. })
        ));
    }

    #[test]
    fn report_loader_is_byte_bounded_before_decode() {
        let oversized = vec![b' '; MAX_REPORT_ARTIFACT_JSON_BYTES + 1];
        assert!(matches!(
            StrategyReportArtifact::from_json_slice(&oversized),
            Err(ReportArtifactError::TooLarge { .. })
        ));
    }

    #[test]
    fn report_identity_inherits_the_manifest_repaint_artifact_and_acknowledgement_binding() {
        let base = manifest();
        let mut binding = base.to_input();
        binding.repaint_qa = vec![RepaintQaBinding {
            indicator_id: "d".repeat(64),
            artifact_id: "e".repeat(64),
            acknowledgement: RepaintAcknowledgement::WarningAcknowledged {
                note: "reviewed repaint evidence".to_string(),
            },
        }];
        let bound = StrategyRunManifest::build(&binding).expect("repaint-bound manifest");
        assert_ne!(bound.run_id(), base.run_id());

        let simulator = report(vec![], &[(0, 1_000.0), (DAY_NS + 1, 1_001.0)]);
        let base_report =
            StrategyReportArtifact::build(&base, &simulator, &[], 1_000.0).expect("base report");
        let bound_report =
            StrategyReportArtifact::build(&bound, &simulator, &[], 1_000.0).expect("bound report");
        assert_eq!(bound_report.run_id(), bound.run_id());
        assert_ne!(bound_report.report_id(), base_report.report_id());
    }

    #[test]
    fn an_unsealed_report_id_never_passes_verification() {
        let manifest = manifest();
        let simulator = report(vec![], &[(0, 1_000.0), (DAY_NS + 1, 1_001.0)]);
        let artifact = StrategyReportArtifact::build(&manifest, &simulator, &[], 1_000.0)
            .expect("artifact builds");
        let mut wire: serde_json::Value =
            serde_json::from_slice(&artifact.to_json_vec().expect("serializes")).expect("json");
        wire["report_id"] = serde_json::json!("");
        assert!(matches!(
            StrategyReportArtifact::from_json_slice(
                &serde_json::to_vec(&wire).expect("blank-id json")
            ),
            Err(ReportArtifactError::InvalidStructure { field: "report_id" })
        ));
    }
}
