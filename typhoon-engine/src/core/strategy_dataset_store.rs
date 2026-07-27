//! Content-addressed on-disk store for immutable strategy datasets — the
//! ADR-135 L1 persistence boundary for milestone M0.
//!
//! A dataset is three files in one directory named by its dataset id:
//!
//! ```text
//! <root>/v1/<id[0..2]>/<id>/manifest.json   the sealed manifest
//!                          /qa.json         the QA report the seal covers
//!                          /bars.bin        the canonical bar payload
//! ```
//!
//! ## Why a purpose-built payload format
//!
//! `bars.bin` is not JSON. Two properties are needed that a text encoding
//! cannot give cheaply:
//!
//! 1. **Byte-identical recovery.** Prices are written as [`f64::to_bits`], so
//!    what comes back is the same bit pattern that went in — including `-0.0`,
//!    subnormals, and values whose shortest decimal rendering is
//!    implementation-defined. Timestamps are stored verbatim, so
//!    `2024-01-02T00:00:00+00:00` never silently becomes `...Z`.
//! 2. **O(1)-ish paged reads.** A trailing offset index turns "give me bars
//!    5,000–5,200" into two small reads instead of decoding everything before
//!    them, which is what lets the inspector page a large dataset without
//!    loading it.
//!
//! ```text
//! magic "TYPHDSB1" | bar_count u64 | index_offset u64
//! records:  ts_len u32 | ts bytes | open | high | low | close | volume   (u64 bits each)
//! index:    (bar_count + 1) × u64 absolute record offsets, last == index_offset
//! trailer:  sha256 over everything above
//! ```
//!
//! ## Durability
//!
//! A record is written into a staging directory, every file is `fsync`ed, the
//! staging directory itself is `fsync`ed, and only then is it `rename`d into
//! place — an atomic publish on POSIX. A record directory is never opened for
//! writing after publication, so a crash can leave staging residue (swept on
//! the next put) but never a half-published record.
//!
//! ## What integrity checking does and does not claim
//!
//! Opening a record verifies the manifest's own seal, the QA report against
//! that seal, and the payload against its trailing digest. That catches
//! truncation, bit rot, and hand-edits of any single file.
//! [`DatasetRecord::load_bars`] additionally recomputes the dataset id from the
//! decoded bars, which is the one check that binds payload to manifest. None
//! of this defends against an attacker who can rewrite every file at once —
//! this is a local artifact store, not a signed one, and the threat model is
//! corruption.

use crate::broker::alpaca::Bar;
use crate::core::strategy_dataset::{
    DatasetError, DatasetManifest, DatasetManifestInput, DatasetQaFinding, DatasetQaReport,
};
use sha2::{Digest, Sha256};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// On-disk layout version. A change to the directory shape or the payload
/// format lands under a new directory, so old records stay readable by the
/// build that wrote them instead of being reinterpreted.
pub const DATASET_STORE_LAYOUT_VERSION: &str = "v1";

/// Hard cap on bars in one stored dataset.
pub const MAX_STORED_BARS: u64 = 4_000_000;

/// Hard cap on `bars.bin` bytes, checked before the file is read.
pub const MAX_BAR_PAYLOAD_BYTES: u64 = 1_073_741_824;

/// Hard cap on `manifest.json` bytes.
pub const MAX_MANIFEST_JSON_BYTES: u64 = 262_144;

/// Hard cap on `qa.json` bytes. Larger than the manifest because a report
/// carries up to `DatasetQaPolicy::max_findings` located findings.
pub const MAX_QA_JSON_BYTES: u64 = 33_554_432;

/// Largest bar window one [`DatasetRecord::read_page`] call may return.
pub const MAX_PAGE_BARS: usize = 1_000;

/// Largest number of records one [`FileDatasetStore::list`] call may return.
pub const MAX_LISTED_RECORDS: usize = 4_096;

/// Longest bar timestamp the payload format can frame.
pub const MAX_BAR_TIMESTAMP_BYTES: usize = 64;

/// Largest a single encoded record can be: the timestamp length prefix, the
/// longest legal timestamp, and five `u64` price/volume fields.
const MAX_RECORD_BYTES: u64 = 4 + MAX_BAR_TIMESTAMP_BYTES as u64 + 40;

/// Length of a dataset id (a hex SHA-256).
const DATASET_ID_LEN: usize = 64;

const PAYLOAD_MAGIC: &[u8; 8] = b"TYPHDSB1";
const PAYLOAD_HEADER_BYTES: u64 = 24;
const PAYLOAD_DIGEST_BYTES: u64 = 32;
const PAYLOAD_STREAM_CHUNK: usize = 65_536;

const MANIFEST_FILE: &str = "manifest.json";
const QA_FILE: &str = "qa.json";
const PAYLOAD_FILE: &str = "bars.bin";
const STAGING_DIR: &str = ".staging";

// ── Errors ─────────────────────────────────────────────────────────

/// Everything that can go wrong at the storage boundary.
#[derive(Debug)]
pub enum DatasetStoreError {
    /// A dataset id was not a 64-character lowercase hex string. Rejected
    /// before any path is built from it, so a traversal attempt never reaches
    /// the filesystem.
    InvalidDatasetId { found: String },
    /// No record with this id.
    NotFound { dataset_id: String },
    Io {
        path: String,
        operation: &'static str,
        message: String,
    },
    /// A stored artifact exceeded its byte cap. Reported from the file's
    /// length, before any of it is read into memory.
    ArtifactTooLarge {
        artifact: &'static str,
        limit: u64,
        found: u64,
    },
    /// A stored artifact did not decode, or carried unknown/omitted fields.
    InvalidArtifact {
        artifact: &'static str,
        message: String,
    },
    /// The bar payload is structurally broken or fails its digest.
    CorruptPayload { reason: String },
    /// The manifest, QA report, and bars did not agree.
    Dataset(DatasetError),
    /// More bars than [`MAX_STORED_BARS`].
    TooManyBars { limit: u64, found: u64 },
    /// The manifest's `bar_count` disagreed with the supplied bars.
    BarCountMismatch { recorded: u64, supplied: u64 },
    /// A page request started at or past the end of the dataset.
    PageOutOfRange { offset: u64, total_bars: u64 },
    /// A page request asked for zero bars or more than [`MAX_PAGE_BARS`].
    PageTooLarge { limit: usize, requested: usize },
    /// A listing asked for more than [`MAX_LISTED_RECORDS`].
    ListLimitTooLarge { limit: usize, requested: usize },
}

impl std::fmt::Display for DatasetStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDatasetId { found } => {
                write!(f, "`{found}` is not a valid dataset id")
            }
            Self::NotFound { dataset_id } => write!(f, "no stored dataset `{dataset_id}`"),
            Self::Io {
                path,
                operation,
                message,
            } => write!(f, "{operation} failed for `{path}`: {message}"),
            Self::ArtifactTooLarge {
                artifact,
                limit,
                found,
            } => write!(f, "{artifact} is {found} bytes, limit {limit}"),
            Self::InvalidArtifact { artifact, message } => {
                write!(f, "{artifact} is not a valid artifact: {message}")
            }
            Self::CorruptPayload { reason } => write!(f, "bar payload is corrupt: {reason}"),
            Self::Dataset(error) => write!(f, "{error}"),
            Self::TooManyBars { limit, found } => {
                write!(f, "dataset has {found} bars, limit {limit}")
            }
            Self::BarCountMismatch { recorded, supplied } => write!(
                f,
                "manifest records {recorded} bars but {supplied} were supplied"
            ),
            Self::PageOutOfRange { offset, total_bars } => write!(
                f,
                "page offset {offset} is past the end ({total_bars} bars)"
            ),
            Self::PageTooLarge { limit, requested } => {
                write!(f, "page of {requested} bars requested, allowed 1..={limit}")
            }
            Self::ListLimitTooLarge { limit, requested } => {
                write!(f, "listing of {requested} requested, allowed 0..={limit}")
            }
        }
    }
}

impl std::error::Error for DatasetStoreError {}

impl From<DatasetError> for DatasetStoreError {
    fn from(error: DatasetError) -> Self {
        Self::Dataset(error)
    }
}

fn io_error(path: &Path, operation: &'static str, error: std::io::Error) -> DatasetStoreError {
    DatasetStoreError::Io {
        path: path.display().to_string(),
        operation,
        message: error.to_string(),
    }
}

// ── Payload codec ──────────────────────────────────────────────────

fn corrupt(reason: impl Into<String>) -> DatasetStoreError {
    DatasetStoreError::CorruptPayload {
        reason: reason.into(),
    }
}

/// Encode `bars` into the canonical payload format.
///
/// Rejects anything the format cannot represent exactly: an over-long
/// timestamp, a non-finite price or volume, or more bars than the cap. Nothing
/// is normalized on the way in — `-0.0` stays `-0.0`.
pub fn encode_bar_payload(bars: &[Bar]) -> Result<Vec<u8>, DatasetStoreError> {
    if bars.len() as u64 > MAX_STORED_BARS {
        return Err(DatasetStoreError::TooManyBars {
            limit: MAX_STORED_BARS,
            found: bars.len() as u64,
        });
    }

    let mut records: Vec<u8> = Vec::new();
    let mut offsets: Vec<u64> = Vec::with_capacity(bars.len() + 1);
    for (index, bar) in bars.iter().enumerate() {
        offsets.push(PAYLOAD_HEADER_BYTES + records.len() as u64);
        let timestamp = bar.timestamp.as_bytes();
        if timestamp.is_empty() || timestamp.len() > MAX_BAR_TIMESTAMP_BYTES {
            return Err(corrupt(format!(
                "bar {index} timestamp is {} bytes, allowed 1..={MAX_BAR_TIMESTAMP_BYTES}",
                timestamp.len()
            )));
        }
        records.extend_from_slice(&(timestamp.len() as u32).to_be_bytes());
        records.extend_from_slice(timestamp);
        for (field, value) in [
            ("open", bar.open),
            ("high", bar.high),
            ("low", bar.low),
            ("close", bar.close),
            ("volume", bar.volume),
        ] {
            if !value.is_finite() {
                return Err(corrupt(format!(
                    "bar {index} field `{field}` is not finite"
                )));
            }
            records.extend_from_slice(&value.to_bits().to_be_bytes());
        }
    }
    let index_offset = PAYLOAD_HEADER_BYTES + records.len() as u64;
    offsets.push(index_offset);

    let mut out = Vec::with_capacity(
        PAYLOAD_HEADER_BYTES as usize
            + records.len()
            + offsets.len() * 8
            + PAYLOAD_DIGEST_BYTES as usize,
    );
    out.extend_from_slice(PAYLOAD_MAGIC);
    out.extend_from_slice(&(bars.len() as u64).to_be_bytes());
    out.extend_from_slice(&index_offset.to_be_bytes());
    out.extend_from_slice(&records);
    for offset in &offsets {
        out.extend_from_slice(&offset.to_be_bytes());
    }
    let digest = Sha256::digest(&out);
    out.extend_from_slice(&digest);
    Ok(out)
}

fn read_be_u64(bytes: &[u8], at: usize) -> Option<u64> {
    let slice: [u8; 8] = bytes.get(at..at + 8)?.try_into().ok()?;
    Some(u64::from_be_bytes(slice))
}

/// Structural facts about a payload, read from its header and trailer alone.
#[derive(Debug, Clone, Copy)]
struct PayloadLayout {
    bar_count: u64,
    index_offset: u64,
    total_bytes: u64,
}

impl PayloadLayout {
    fn index_entry_offset(&self, index: u64) -> u64 {
        self.index_offset + index * 8
    }

    fn expected_total(&self) -> Option<u64> {
        let index_bytes = self.bar_count.checked_add(1)?.checked_mul(8)?;
        self.index_offset
            .checked_add(index_bytes)?
            .checked_add(PAYLOAD_DIGEST_BYTES)
    }
}

fn parse_payload_header(
    header: &[u8],
    total_bytes: u64,
) -> Result<PayloadLayout, DatasetStoreError> {
    if header.len() < PAYLOAD_HEADER_BYTES as usize {
        return Err(corrupt("shorter than the payload header"));
    }
    if &header[..8] != PAYLOAD_MAGIC {
        return Err(corrupt("wrong magic — not a TyphooN dataset payload"));
    }
    let bar_count = read_be_u64(header, 8).ok_or_else(|| corrupt("truncated bar count"))?;
    if bar_count > MAX_STORED_BARS {
        return Err(DatasetStoreError::TooManyBars {
            limit: MAX_STORED_BARS,
            found: bar_count,
        });
    }
    let index_offset = read_be_u64(header, 16).ok_or_else(|| corrupt("truncated index offset"))?;
    if index_offset < PAYLOAD_HEADER_BYTES {
        return Err(corrupt("index offset overlaps the header"));
    }
    let layout = PayloadLayout {
        bar_count,
        index_offset,
        total_bytes,
    };
    match layout.expected_total() {
        Some(expected) if expected == total_bytes => Ok(layout),
        Some(expected) => Err(corrupt(format!(
            "payload is {total_bytes} bytes, layout implies {expected}"
        ))),
        None => Err(corrupt("payload layout overflows")),
    }
}

/// Decode one record starting at `cursor` within `bytes`.
fn decode_record(bytes: &[u8], cursor: usize) -> Result<(Bar, usize), DatasetStoreError> {
    let length_bytes: [u8; 4] = bytes
        .get(cursor..cursor + 4)
        .and_then(|slice| slice.try_into().ok())
        .ok_or_else(|| corrupt("truncated timestamp length"))?;
    let length = u32::from_be_bytes(length_bytes) as usize;
    if length == 0 || length > MAX_BAR_TIMESTAMP_BYTES {
        return Err(corrupt(format!(
            "timestamp length {length} outside 1..={MAX_BAR_TIMESTAMP_BYTES}"
        )));
    }
    let start = cursor + 4;
    let end = start + length;
    let timestamp = bytes
        .get(start..end)
        .ok_or_else(|| corrupt("truncated timestamp"))?;
    let timestamp = std::str::from_utf8(timestamp)
        .map_err(|_| corrupt("timestamp is not valid UTF-8"))?
        .to_string();

    let mut values = [0.0_f64; 5];
    for (slot, value) in values.iter_mut().enumerate() {
        let at = end + slot * 8;
        let bits = read_be_u64(bytes, at).ok_or_else(|| corrupt("truncated price field"))?;
        let decoded = f64::from_bits(bits);
        if !decoded.is_finite() {
            return Err(corrupt("decoded a non-finite price or volume"));
        }
        *value = decoded;
    }
    Ok((
        Bar {
            timestamp,
            open: values[0],
            high: values[1],
            low: values[2],
            close: values[3],
            volume: values[4],
        },
        end + 40,
    ))
}

/// Decode a complete payload, verifying its digest and structure.
pub fn decode_bar_payload(bytes: &[u8]) -> Result<Vec<Bar>, DatasetStoreError> {
    let layout = parse_payload_header(bytes, bytes.len() as u64)?;
    let signed_len = (layout.total_bytes - PAYLOAD_DIGEST_BYTES) as usize;
    let expected = Sha256::digest(&bytes[..signed_len]);
    if bytes[signed_len..] != expected[..] {
        return Err(corrupt("digest mismatch"));
    }

    let mut bars = Vec::with_capacity(layout.bar_count as usize);
    let mut cursor = PAYLOAD_HEADER_BYTES as usize;
    for index in 0..layout.bar_count {
        let recorded = read_be_u64(bytes, layout.index_entry_offset(index) as usize)
            .ok_or_else(|| corrupt("truncated offset index"))?;
        if recorded != cursor as u64 {
            return Err(corrupt(format!(
                "offset index entry {index} says {recorded}, records end at {cursor}"
            )));
        }
        let (bar, next) = decode_record(bytes, cursor)?;
        if next as u64 > layout.index_offset {
            return Err(corrupt("a record runs past the offset index"));
        }
        cursor = next;
        bars.push(bar);
    }
    if cursor as u64 != layout.index_offset {
        return Err(corrupt("trailing bytes between the records and the index"));
    }
    Ok(bars)
}

// ── Store ──────────────────────────────────────────────────────────

/// Whether a put created a record or found one already there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetPutOutcome {
    Stored,
    AlreadyPresent,
}

/// The manifest, QA report, and publication outcome of one put.
#[derive(Debug, Clone)]
pub struct StoredDataset {
    pub manifest: DatasetManifest,
    pub qa: DatasetQaReport,
    pub outcome: DatasetPutOutcome,
}

/// The header line the inspector lists, readable without opening a payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetRecordSummary {
    pub dataset_id: String,
    pub manifest_id: String,
    pub symbol: String,
    pub timeframe: String,
    pub source: String,
    pub venue: String,
    pub pipeline: String,
    pub adjustment: crate::core::strategy_dataset::AdjustmentPolicy,
    pub calendar_policy_id: String,
    pub qa_policy_id: String,
    pub bar_count: u64,
    pub first_timestamp: Option<String>,
    pub last_timestamp: Option<String>,
    pub qa_error_count: u64,
    pub qa_warning_count: u64,
    pub qa_findings_truncated: bool,
}

impl DatasetRecordSummary {
    pub fn from_manifest(manifest: &DatasetManifest) -> Self {
        Self {
            dataset_id: manifest.dataset_id.clone(),
            manifest_id: manifest.manifest_id.clone(),
            symbol: manifest.symbol.clone(),
            timeframe: manifest.timeframe.clone(),
            source: manifest.provenance.source.clone(),
            venue: manifest.provenance.venue.clone(),
            pipeline: manifest.provenance.pipeline.clone(),
            adjustment: manifest.adjustment,
            calendar_policy_id: manifest.calendar_policy_id.clone(),
            qa_policy_id: manifest.qa_policy_id.clone(),
            bar_count: manifest.bar_count,
            first_timestamp: manifest.first_timestamp.clone(),
            last_timestamp: manifest.last_timestamp.clone(),
            qa_error_count: manifest.qa_error_count,
            qa_warning_count: manifest.qa_warning_count,
            qa_findings_truncated: manifest.qa_findings_truncated,
        }
    }

    /// Short label for a list row.
    pub fn title(&self) -> String {
        format!("{} · {}", self.symbol, self.timeframe)
    }
}

/// A bounded window of bars plus the QA findings that land inside it.
///
/// Deliberately not `PartialEq`: `Bar` holds floats, and a derived comparison
/// would call `-0.0` equal to `+0.0` — exactly the distinction the byte-identity
/// guarantee exists to preserve. Compare bit patterns instead.
#[derive(Debug, Clone)]
pub struct DatasetPage {
    pub offset: u64,
    pub total_bars: u64,
    pub bars: Vec<Bar>,
    pub findings: Vec<DatasetQaFinding>,
}

/// A filesystem-backed dataset store rooted at one directory.
#[derive(Debug, Clone)]
pub struct FileDatasetStore {
    root: PathBuf,
}

/// Monotonic suffix for staging directory names. Combined with the process id
/// this keeps concurrent puts — in this process or another — from colliding,
/// without a clock or an RNG.
static STAGING_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl FileDatasetStore {
    /// Open (creating if needed) a store rooted at `root`.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, DatasetStoreError> {
        let root = root.as_ref().to_path_buf();
        let layout = root.join(DATASET_STORE_LAYOUT_VERSION);
        std::fs::create_dir_all(&layout).map_err(|e| io_error(&layout, "create store root", e))?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn layout_dir(&self) -> PathBuf {
        self.root.join(DATASET_STORE_LAYOUT_VERSION)
    }

    /// Where in-flight records are assembled before being renamed into place.
    pub fn staging_dir(&self) -> PathBuf {
        self.layout_dir().join(STAGING_DIR)
    }

    /// The directory a record with this id occupies.
    ///
    /// Builds a path but touches nothing. The id is *not* validated here —
    /// every public entry point validates first — so the shard prefix is taken
    /// with `get`, which yields `None` rather than panicking if a caller hands
    /// in a string whose second byte is mid-character.
    pub fn record_dir(&self, dataset_id: &str) -> PathBuf {
        let shard = dataset_id.get(..2).unwrap_or(dataset_id);
        self.layout_dir().join(shard).join(dataset_id)
    }

    /// Whether a record with this id is present.
    pub fn contains(&self, dataset_id: &str) -> Result<bool, DatasetStoreError> {
        validate_dataset_id(dataset_id)?;
        Ok(self.record_dir(dataset_id).join(MANIFEST_FILE).is_file())
    }

    /// Build a manifest and QA report for `bars` and publish all three files.
    pub fn build_and_put(
        &self,
        input: &DatasetManifestInput,
        bars: &[Bar],
    ) -> Result<StoredDataset, DatasetStoreError> {
        let (manifest, qa) = DatasetManifest::build_with_qa(input, bars)?;
        let outcome = self.put(&manifest, &qa, bars)?;
        Ok(StoredDataset {
            manifest,
            qa,
            outcome,
        })
    }

    /// Publish an already-built dataset.
    ///
    /// Every input is re-verified: the caller could have assembled a manifest
    /// that does not describe these bars, and a store that accepted it would
    /// hand back an unverifiable record forever after.
    pub fn put(
        &self,
        manifest: &DatasetManifest,
        qa: &DatasetQaReport,
        bars: &[Bar],
    ) -> Result<DatasetPutOutcome, DatasetStoreError> {
        validate_dataset_id(&manifest.dataset_id)?;
        if manifest.bar_count > MAX_STORED_BARS {
            return Err(DatasetStoreError::TooManyBars {
                limit: MAX_STORED_BARS,
                found: manifest.bar_count,
            });
        }
        if manifest.bar_count != bars.len() as u64 {
            return Err(DatasetStoreError::BarCountMismatch {
                recorded: manifest.bar_count,
                supplied: bars.len() as u64,
            });
        }
        manifest.verify(bars)?;
        manifest.verify_qa_report(qa)?;

        let destination = self.record_dir(&manifest.dataset_id);
        if destination.join(MANIFEST_FILE).is_file() {
            return self.verify_existing_record(manifest, qa);
        }

        let payload = encode_bar_payload(bars)?;
        let manifest_json =
            serde_json::to_vec(manifest).map_err(|error| DatasetStoreError::InvalidArtifact {
                artifact: MANIFEST_FILE,
                message: error.to_string(),
            })?;
        let qa_json =
            serde_json::to_vec(qa).map_err(|error| DatasetStoreError::InvalidArtifact {
                artifact: QA_FILE,
                message: error.to_string(),
            })?;

        let staging_root = self.staging_dir();
        std::fs::create_dir_all(&staging_root)
            .map_err(|e| io_error(&staging_root, "create staging root", e))?;
        self.sweep_staging(&staging_root);

        let ticket = STAGING_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let staging = staging_root.join(format!("{}-{ticket}", std::process::id()));
        // A leftover from an earlier crash with the same pid+ticket would make
        // the publish ambiguous; start from a clean directory.
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir_all(&staging)
            .map_err(|e| io_error(&staging, "create staging directory", e))?;

        let published = self.publish(
            &staging,
            &destination,
            manifest,
            qa,
            &manifest_json,
            &qa_json,
            &payload,
        );
        if published.is_err() {
            let _ = std::fs::remove_dir_all(&staging);
        }
        published
    }

    fn publish(
        &self,
        staging: &Path,
        destination: &Path,
        manifest: &DatasetManifest,
        qa: &DatasetQaReport,
        manifest_json: &[u8],
        qa_json: &[u8],
        payload: &[u8],
    ) -> Result<DatasetPutOutcome, DatasetStoreError> {
        for (name, bytes) in [
            (MANIFEST_FILE, manifest_json),
            (QA_FILE, qa_json),
            (PAYLOAD_FILE, payload),
        ] {
            write_durably(&staging.join(name), bytes)?;
        }
        sync_dir(staging)?;

        let shard = destination
            .parent()
            .ok_or_else(|| corrupt("record path has no shard directory"))?;
        std::fs::create_dir_all(shard).map_err(|e| io_error(shard, "create shard directory", e))?;

        match std::fs::rename(staging, destination) {
            Ok(()) => {
                sync_dir(shard)?;
                Ok(DatasetPutOutcome::Stored)
            }
            Err(error) => {
                // Another writer published the same content first. Content
                // addressing makes that a no-op, not a conflict.
                if destination.join(MANIFEST_FILE).is_file() {
                    let _ = std::fs::remove_dir_all(staging);
                    self.verify_existing_record(manifest, qa)
                } else {
                    Err(io_error(destination, "publish record", error))
                }
            }
        }
    }

    fn verify_existing_record(
        &self,
        manifest: &DatasetManifest,
        qa: &DatasetQaReport,
    ) -> Result<DatasetPutOutcome, DatasetStoreError> {
        let record = self.open_record(&manifest.dataset_id)?;
        if record.manifest() != manifest || record.qa() != qa {
            return Err(DatasetStoreError::InvalidArtifact {
                artifact: MANIFEST_FILE,
                message: "dataset id already exists with a different sealed manifest or QA report"
                    .to_string(),
            });
        }
        record.load_bars()?;
        Ok(DatasetPutOutcome::AlreadyPresent)
    }

    /// Remove staging directories left by an interrupted put.
    ///
    /// This sweeps *all* staging entries, not just this process's. On a
    /// single-user desktop terminal that is the right trade: crash residue from
    /// a previous run gets cleaned up, and the worst case for a second process
    /// putting concurrently is that its own put fails at the rename — a
    /// content-addressed retry, never a corrupt or partial record.
    fn sweep_staging(&self, staging_root: &Path) {
        let Ok(entries) = std::fs::read_dir(staging_root) else {
            return;
        };
        for entry in entries.flatten() {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }

    /// Open a stored record: manifest and QA report decoded and cross-checked,
    /// payload structurally validated and digest-verified.
    ///
    /// Bars are **not** loaded — that is [`DatasetRecord::load_bars`] or a
    /// bounded [`DatasetRecord::read_page`].
    pub fn open_record(&self, dataset_id: &str) -> Result<DatasetRecord, DatasetStoreError> {
        validate_dataset_id(dataset_id)?;
        let dir = self.record_dir(dataset_id);
        if !dir.join(MANIFEST_FILE).is_file() {
            return Err(DatasetStoreError::NotFound {
                dataset_id: dataset_id.to_string(),
            });
        }

        let manifest: DatasetManifest = read_json_artifact(
            &dir.join(MANIFEST_FILE),
            MANIFEST_FILE,
            MAX_MANIFEST_JSON_BYTES,
        )?;
        manifest.verify_seal()?;
        if manifest.dataset_id != dataset_id {
            return Err(DatasetStoreError::InvalidArtifact {
                artifact: MANIFEST_FILE,
                message: format!(
                    "stored under `{dataset_id}` but claims `{}`",
                    manifest.dataset_id
                ),
            });
        }

        let qa: DatasetQaReport =
            read_json_artifact(&dir.join(QA_FILE), QA_FILE, MAX_QA_JSON_BYTES)?;
        manifest.verify_qa_report(&qa)?;

        let payload_path = dir.join(PAYLOAD_FILE);
        let layout = verify_payload_file(&payload_path)?;
        if layout.bar_count != manifest.bar_count {
            return Err(corrupt(format!(
                "payload holds {} bars, manifest records {}",
                layout.bar_count, manifest.bar_count
            )));
        }

        Ok(DatasetRecord {
            manifest,
            qa,
            payload_path,
            layout,
        })
    }

    /// Up to `limit` record summaries, ordered by dataset id.
    ///
    /// Directory entries that are not records (staging, stray files) are
    /// skipped; a directory that *is* a record but whose manifest will not
    /// decode is an error, because silently hiding a corrupt dataset is how a
    /// browser lies about what is on disk.
    pub fn list(&self, limit: usize) -> Result<Vec<DatasetRecordSummary>, DatasetStoreError> {
        if limit > MAX_LISTED_RECORDS {
            return Err(DatasetStoreError::ListLimitTooLarge {
                limit: MAX_LISTED_RECORDS,
                requested: limit,
            });
        }
        let mut summaries = Vec::new();
        if limit == 0 {
            return Ok(summaries);
        }

        for shard in sorted_dir_names(&self.layout_dir())? {
            if shard.len() != 2 || !shard.bytes().all(is_lower_hex) {
                continue;
            }
            let shard_dir = self.layout_dir().join(&shard);
            for name in sorted_dir_names(&shard_dir)? {
                if validate_dataset_id(&name).is_err() || !name.starts_with(&shard) {
                    continue;
                }
                let manifest_path = shard_dir.join(&name).join(MANIFEST_FILE);
                if !manifest_path.is_file() {
                    continue;
                }
                let manifest: DatasetManifest =
                    read_json_artifact(&manifest_path, MANIFEST_FILE, MAX_MANIFEST_JSON_BYTES)?;
                manifest.verify_seal()?;
                summaries.push(DatasetRecordSummary::from_manifest(&manifest));
                if summaries.len() >= limit {
                    return Ok(summaries);
                }
            }
        }
        Ok(summaries)
    }
}

/// An opened record. Holds the manifest, the QA report, and the payload's
/// structural layout — never the bars themselves.
#[derive(Debug, Clone)]
pub struct DatasetRecord {
    manifest: DatasetManifest,
    qa: DatasetQaReport,
    payload_path: PathBuf,
    layout: PayloadLayout,
}

impl DatasetRecord {
    pub fn manifest(&self) -> &DatasetManifest {
        &self.manifest
    }

    pub fn qa(&self) -> &DatasetQaReport {
        &self.qa
    }

    pub fn summary(&self) -> DatasetRecordSummary {
        DatasetRecordSummary::from_manifest(&self.manifest)
    }

    pub fn bar_count(&self) -> u64 {
        self.layout.bar_count
    }

    /// A bounded window of bars starting at `offset`, plus the QA findings
    /// that land inside it.
    ///
    /// Two small reads from the offset index bound the byte range, so the cost
    /// is the window's size and not the dataset's. Out-of-range and oversized
    /// requests are refused rather than clamped: a silently clamped page is a
    /// paging bug that only shows up as missing rows.
    pub fn read_page(&self, offset: u64, limit: usize) -> Result<DatasetPage, DatasetStoreError> {
        if limit == 0 || limit > MAX_PAGE_BARS {
            return Err(DatasetStoreError::PageTooLarge {
                limit: MAX_PAGE_BARS,
                requested: limit,
            });
        }
        if offset >= self.layout.bar_count {
            return Err(DatasetStoreError::PageOutOfRange {
                offset,
                total_bars: self.layout.bar_count,
            });
        }
        let end = offset
            .saturating_add(limit as u64)
            .min(self.layout.bar_count);
        let count = (end - offset) as usize;

        let mut file = std::fs::File::open(&self.payload_path)
            .map_err(|e| io_error(&self.payload_path, "open payload", e))?;
        let start_byte = self.read_index_entry(&mut file, offset)?;
        let end_byte = self.read_index_entry(&mut file, end)?;
        if end_byte < start_byte || end_byte > self.layout.index_offset {
            return Err(corrupt("offset index entries are not monotonic"));
        }

        // `count` records cannot occupy more than this, whatever the offset
        // index claims. Without the check, a rewritten index could make a
        // ten-row page allocate the whole payload.
        let span_limit = (count as u64).saturating_mul(MAX_RECORD_BYTES);
        let span = end_byte - start_byte;
        if span > span_limit {
            return Err(corrupt(format!(
                "page of {count} record(s) spans {span} bytes, at most {span_limit} is possible"
            )));
        }
        let span = span as usize;
        let mut window = vec![0u8; span];
        file.seek(SeekFrom::Start(start_byte))
            .map_err(|e| io_error(&self.payload_path, "seek payload", e))?;
        file.read_exact(&mut window)
            .map_err(|e| io_error(&self.payload_path, "read payload window", e))?;

        let mut bars = Vec::with_capacity(count);
        let mut cursor = 0usize;
        for _ in 0..count {
            let (bar, next) = decode_record(&window, cursor)?;
            cursor = next;
            bars.push(bar);
        }
        if cursor != span {
            return Err(corrupt("page window did not decode to exactly its records"));
        }

        Ok(DatasetPage {
            offset,
            total_bars: self.layout.bar_count,
            findings: self.qa.findings_in_range(offset, count),
            bars,
        })
    }

    fn read_index_entry(
        &self,
        file: &mut std::fs::File,
        index: u64,
    ) -> Result<u64, DatasetStoreError> {
        let at = self.layout.index_entry_offset(index);
        file.seek(SeekFrom::Start(at))
            .map_err(|e| io_error(&self.payload_path, "seek offset index", e))?;
        let mut buffer = [0u8; 8];
        file.read_exact(&mut buffer)
            .map_err(|e| io_error(&self.payload_path, "read offset index", e))?;
        let offset = u64::from_be_bytes(buffer);
        if offset < PAYLOAD_HEADER_BYTES || offset > self.layout.index_offset {
            return Err(corrupt(format!(
                "offset index entry {index} points outside the record region"
            )));
        }
        Ok(offset)
    }

    /// Every bar, authenticated against the manifest.
    ///
    /// This is the strongest check the store can make: the dataset id is
    /// recomputed from the decoded bars and compared with the one the manifest
    /// records, so a payload that decodes cleanly but is not *this* dataset is
    /// still refused.
    pub fn load_bars(&self) -> Result<Vec<Bar>, DatasetStoreError> {
        let bytes = read_bounded(&self.payload_path, PAYLOAD_FILE, MAX_BAR_PAYLOAD_BYTES)?;
        let bars = decode_bar_payload(&bytes)?;
        let recomputed = self.manifest.recompute_dataset_id(&bars)?;
        if recomputed != self.manifest.dataset_id {
            return Err(DatasetStoreError::Dataset(
                DatasetError::DatasetIdMismatch {
                    expected: self.manifest.dataset_id.clone(),
                    actual: recomputed,
                },
            ));
        }
        Ok(bars)
    }
}

// ── Filesystem helpers ─────────────────────────────────────────────

fn is_lower_hex(byte: u8) -> bool {
    byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
}

/// A dataset id must be exactly a lowercase hex SHA-256. Everything else —
/// empty, `..`, a path, an uppercase variant — is refused before it is used to
/// build a path.
fn validate_dataset_id(dataset_id: &str) -> Result<(), DatasetStoreError> {
    if dataset_id.len() == DATASET_ID_LEN && dataset_id.bytes().all(is_lower_hex) {
        Ok(())
    } else {
        Err(DatasetStoreError::InvalidDatasetId {
            found: dataset_id.to_string(),
        })
    }
}

fn sorted_dir_names(dir: &Path) -> Result<Vec<String>, DatasetStoreError> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(io_error(dir, "list directory", error)),
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    names.sort_unstable();
    Ok(names)
}

/// Write `bytes` and `fsync` before returning, so a later directory rename
/// cannot publish a file whose contents are still in the page cache.
fn write_durably(path: &Path, bytes: &[u8]) -> Result<(), DatasetStoreError> {
    let mut file = std::fs::File::create(path).map_err(|e| io_error(path, "create file", e))?;
    file.write_all(bytes)
        .map_err(|e| io_error(path, "write file", e))?;
    file.sync_all().map_err(|e| io_error(path, "fsync file", e))
}

fn sync_dir(dir: &Path) -> Result<(), DatasetStoreError> {
    let handle = std::fs::File::open(dir).map_err(|e| io_error(dir, "open directory", e))?;
    handle
        .sync_all()
        .map_err(|e| io_error(dir, "fsync directory", e))
}

/// Read a file whose length is checked against `limit` *before* any of it is
/// pulled into memory.
fn read_bounded(
    path: &Path,
    artifact: &'static str,
    limit: u64,
) -> Result<Vec<u8>, DatasetStoreError> {
    let metadata = std::fs::metadata(path).map_err(|e| io_error(path, "stat artifact", e))?;
    if metadata.len() > limit {
        return Err(DatasetStoreError::ArtifactTooLarge {
            artifact,
            limit,
            found: metadata.len(),
        });
    }
    std::fs::read(path).map_err(|e| io_error(path, "read artifact", e))
}

/// Decode a stored JSON artifact strictly: size-bounded, unknown fields
/// refused by the type's `deny_unknown_fields`, and re-serialization compared
/// against the original so a field the decoder ignored cannot slip through.
fn read_json_artifact<T>(
    path: &Path,
    artifact: &'static str,
    limit: u64,
) -> Result<T, DatasetStoreError>
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    let bytes = read_bounded(path, artifact, limit)?;
    let original: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| DatasetStoreError::InvalidArtifact {
            artifact,
            message: error.to_string(),
        })?;
    let decoded: T = serde_json::from_value(original.clone()).map_err(|error| {
        DatasetStoreError::InvalidArtifact {
            artifact,
            message: error.to_string(),
        }
    })?;
    let recognized =
        serde_json::to_value(&decoded).map_err(|error| DatasetStoreError::InvalidArtifact {
            artifact,
            message: error.to_string(),
        })?;
    if original != recognized {
        return Err(DatasetStoreError::InvalidArtifact {
            artifact,
            message: "contains unknown, omitted, or non-canonical fields".to_string(),
        });
    }
    Ok(decoded)
}

/// Validate a payload file's header/trailer and stream-verify its digest in
/// bounded chunks — never loading the whole payload to authenticate it.
fn verify_payload_file(path: &Path) -> Result<PayloadLayout, DatasetStoreError> {
    let metadata = std::fs::metadata(path).map_err(|e| io_error(path, "stat payload", e))?;
    let total = metadata.len();
    if total > MAX_BAR_PAYLOAD_BYTES {
        return Err(DatasetStoreError::ArtifactTooLarge {
            artifact: PAYLOAD_FILE,
            limit: MAX_BAR_PAYLOAD_BYTES,
            found: total,
        });
    }
    if total < PAYLOAD_HEADER_BYTES + PAYLOAD_DIGEST_BYTES {
        return Err(corrupt("shorter than an empty payload"));
    }

    let mut file = std::fs::File::open(path).map_err(|e| io_error(path, "open payload", e))?;
    let mut header = [0u8; PAYLOAD_HEADER_BYTES as usize];
    file.read_exact(&mut header)
        .map_err(|e| io_error(path, "read payload header", e))?;
    let layout = parse_payload_header(&header, total)?;

    let signed_len = total - PAYLOAD_DIGEST_BYTES;
    file.seek(SeekFrom::Start(0))
        .map_err(|e| io_error(path, "rewind payload", e))?;
    let mut hasher = Sha256::new();
    let mut remaining = signed_len;
    let mut chunk = vec![0u8; PAYLOAD_STREAM_CHUNK];
    while remaining > 0 {
        let want = remaining.min(PAYLOAD_STREAM_CHUNK as u64) as usize;
        file.read_exact(&mut chunk[..want])
            .map_err(|e| io_error(path, "read payload", e))?;
        hasher.update(&chunk[..want]);
        remaining -= want as u64;
    }
    let mut recorded = [0u8; PAYLOAD_DIGEST_BYTES as usize];
    file.read_exact(&mut recorded)
        .map_err(|e| io_error(path, "read payload digest", e))?;
    if hasher.finalize()[..] != recorded[..] {
        return Err(corrupt("digest mismatch"));
    }
    Ok(layout)
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
