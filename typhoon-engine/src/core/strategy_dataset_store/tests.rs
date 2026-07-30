use super::*;
use crate::core::strategy_dataset::{
    AdjustmentPolicy, CalendarPolicy, DatasetProvenance, DatasetQaPolicy,
};
use std::collections::BTreeSet;

// ── Helpers ────────────────────────────────────────────────────────

fn bar(ts: &str, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Bar {
    Bar {
        timestamp: ts.to_string(),
        open,
        high,
        low,
        close,
        volume,
    }
}

fn input(symbol: &str) -> DatasetManifestInput {
    DatasetManifestInput {
        symbol: symbol.to_string(),
        timeframe: "1Day".to_string(),
        provenance: DatasetProvenance {
            source: "kraken".to_string(),
            venue: "kraken-spot".to_string(),
            pipeline: "cache-merge/v1".to_string(),
        },
        adjustment: AdjustmentPolicy::Raw,
        calendar: CalendarPolicy::Continuous24x7,
        qa_policy: DatasetQaPolicy::default(),
    }
}

/// A payload that exercises the awkward corners of the encoding: a negative
/// zero, a subnormal, a very large magnitude, a long fractional value, and
/// timestamps of differing byte length.
fn awkward_bars() -> Vec<Bar> {
    vec![
        bar("2024-01-01T00:00:00Z", 100.0, 101.0, 99.0, 100.5, -0.0),
        bar(
            "2024-01-02T00:00:00+00:00",
            100.5,
            1.797_693_134_862_315_7e308,
            f64::MIN_POSITIVE,
            100.75,
            1e-300,
        ),
        bar(
            "2024-01-03T00:00:00.123456Z",
            100.75,
            100.900_000_000_000_09,
            100.1,
            100.333_333_333_333_33,
            123_456_789.987_654_32,
        ),
    ]
}

fn clean_bars(count: usize) -> Vec<Bar> {
    let start = chrono::DateTime::from_timestamp(1_704_067_200, 0).expect("epoch");
    let mut close = 100.0_f64;
    (0..count)
        .map(|index| {
            let open = close;
            close = open * 1.005;
            bar(
                &(start + chrono::Duration::days(index as i64))
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                open,
                close * 1.005,
                open * 0.995,
                close,
                1_000.0 + index as f64,
            )
        })
        .collect()
}

fn store_at(root: &std::path::Path) -> FileDatasetStore {
    FileDatasetStore::open(root).expect("store opens")
}

/// Bit-exact comparison — `==` would call `-0.0` and `+0.0` equal and would
/// make the byte-identity claim weaker than it reads.
fn assert_bit_identical(left: &[Bar], right: &[Bar]) {
    assert_eq!(left.len(), right.len(), "bar count");
    for (index, (a, b)) in left.iter().zip(right).enumerate() {
        assert_eq!(a.timestamp, b.timestamp, "bar {index} timestamp");
        for (field, x, y) in [
            ("open", a.open, b.open),
            ("high", a.high, b.high),
            ("low", a.low, b.low),
            ("close", a.close, b.close),
            ("volume", a.volume, b.volume),
        ] {
            assert_eq!(
                x.to_bits(),
                y.to_bits(),
                "bar {index} field {field}: {x} vs {y}"
            );
        }
    }
}

// ── Restart recovery (ADR-135 M0 gate) ─────────────────────────────

#[test]
fn a_stored_dataset_recovers_byte_identically_across_a_restart() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bars = awkward_bars();

    let (manifest, qa) = {
        let store = store_at(temp.path());
        let stored = store
            .build_and_put(&input("BTC/USD"), &bars)
            .expect("dataset stores");
        assert_eq!(stored.outcome, DatasetPutOutcome::Stored);
        (stored.manifest, stored.qa)
    }; // the store handle — and any in-process cache with it — is gone here.

    // A fresh process would do exactly this: open the same root and ask for
    // the id it recorded.
    let reopened = store_at(temp.path());
    let record = reopened
        .open_record(&manifest.dataset_id)
        .expect("record opens after restart");

    let recovered = record.load_bars().expect("bars load");
    assert_bit_identical(&bars, &recovered);
    assert_eq!(
        encode_bar_payload(&bars).expect("encode"),
        encode_bar_payload(&recovered).expect("encode"),
        "recovered payload is not byte-identical"
    );

    // The recovered manifest and QA report are the sealed originals.
    assert_eq!(record.manifest(), &manifest);
    assert_eq!(record.qa(), &qa);
    record
        .manifest()
        .verify(&recovered)
        .expect("recovered manifest verifies against recovered bars");
    record
        .manifest()
        .verify_qa_report(record.qa())
        .expect("recovered QA report matches the seal");
    assert_eq!(record.manifest().dataset_id, manifest.dataset_id);
    assert_eq!(record.manifest().manifest_id, manifest.manifest_id);
}

#[test]
fn storing_the_same_dataset_twice_is_idempotent() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let bars = clean_bars(20);

    let first = store
        .build_and_put(&input("ETH/USD"), &bars)
        .expect("first put");
    assert_eq!(first.outcome, DatasetPutOutcome::Stored);

    let record_dir = store.record_dir(&first.manifest.dataset_id);
    let before: Vec<(std::path::PathBuf, Vec<u8>)> = read_record_files(&record_dir);

    let second = store
        .build_and_put(&input("ETH/USD"), &bars)
        .expect("second put");
    assert_eq!(second.outcome, DatasetPutOutcome::AlreadyPresent);
    assert_eq!(second.manifest.dataset_id, first.manifest.dataset_id);

    let after: Vec<(std::path::PathBuf, Vec<u8>)> = read_record_files(&record_dir);
    assert_eq!(before, after, "an existing record must not be rewritten");
    assert!(
        store
            .contains(&first.manifest.dataset_id)
            .expect("contains")
    );
}

#[test]
fn positive_and_negative_zero_publish_and_recover_independently_in_both_orders() {
    for negative_first in [false, true] {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = store_at(temp.path());
        let mut positive = clean_bars(1);
        positive[0].volume = 0.0;
        let mut negative = positive.clone();
        negative[0].volume = -0.0;
        let (first, second) = if negative_first {
            (&negative, &positive)
        } else {
            (&positive, &negative)
        };

        let first_stored = store
            .build_and_put(&input("ZERO/USD"), first)
            .expect("first zero variant stores");
        let second_stored = store
            .build_and_put(&input("ZERO/USD"), second)
            .expect("second zero variant stores");

        assert_eq!(first_stored.outcome, DatasetPutOutcome::Stored);
        assert_eq!(second_stored.outcome, DatasetPutOutcome::Stored);
        assert_ne!(
            first_stored.manifest.dataset_id,
            second_stored.manifest.dataset_id
        );
        assert_ne!(
            encode_bar_payload(first).expect("encode first"),
            encode_bar_payload(second).expect("encode second")
        );
        let first_loaded = store
            .open_record(&first_stored.manifest.dataset_id)
            .expect("first opens")
            .load_bars()
            .expect("first loads");
        let second_loaded = store
            .open_record(&second_stored.manifest.dataset_id)
            .expect("second opens")
            .load_bars()
            .expect("second loads");
        assert_bit_identical(first, &first_loaded);
        assert_bit_identical(second, &second_loaded);
    }
}

#[test]
fn an_existing_corrupt_record_is_not_reported_as_already_present() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let bars = clean_bars(20);
    let stored = store
        .build_and_put(&input("ETH/USD"), &bars)
        .expect("first put");

    let payload = store
        .record_dir(&stored.manifest.dataset_id)
        .join(PAYLOAD_FILE);
    let mut bytes = std::fs::read(&payload).expect("read payload");
    bytes[PAYLOAD_HEADER_BYTES as usize] ^= 0x01;
    std::fs::write(&payload, bytes).expect("corrupt payload");

    assert!(
        store.build_and_put(&input("ETH/USD"), &bars).is_err(),
        "a corrupt destination must be surfaced, not accepted as idempotent"
    );
}

#[test]
fn the_same_dataset_id_with_a_different_manifest_seal_is_a_conflict() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let bars = clean_bars(20);
    let first_input = input("ETH/USD");
    let stored = store.build_and_put(&first_input, &bars).expect("first put");

    let mut changed_input = first_input;
    changed_input.qa_policy.spike_band_multiple = 7.0;
    let changed =
        DatasetManifest::build_with_qa(&changed_input, &bars).expect("alternate sealed manifest");
    assert_eq!(stored.manifest.dataset_id, changed.0.dataset_id);
    assert_ne!(stored.manifest.manifest_id, changed.0.manifest_id);

    assert!(
        store.put(&changed.0, &changed.1, &bars).is_err(),
        "one dataset-id path must not silently retain a different sealed manifest"
    );
}

fn read_record_files(dir: &std::path::Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut files: Vec<(std::path::PathBuf, Vec<u8>)> = std::fs::read_dir(dir)
        .expect("record dir")
        .map(|entry| {
            let entry = entry.expect("entry");
            let bytes = std::fs::read(entry.path()).expect("read");
            (entry.path(), bytes)
        })
        .collect();
    files.sort();
    files
}

#[test]
fn a_published_record_leaves_no_staging_residue() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let stored = store
        .build_and_put(&input("SOL/USD"), &clean_bars(10))
        .expect("put");

    let names: BTreeSet<String> = std::fs::read_dir(store.record_dir(&stored.manifest.dataset_id))
        .expect("record dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(
        names,
        ["bars.bin", "manifest.json", "qa.json"]
            .into_iter()
            .map(str::to_string)
            .collect::<BTreeSet<_>>()
    );

    // Staging happens under a reserved directory that is emptied on success.
    let staging = store.staging_dir();
    let residue: Vec<_> = std::fs::read_dir(&staging)
        .map(|entries| entries.map(|e| e.expect("entry").path()).collect())
        .unwrap_or_default();
    assert!(residue.is_empty(), "staging residue: {residue:?}");
}

#[test]
fn a_stale_staging_directory_is_never_visible_as_a_record() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let stored = store
        .build_and_put(&input("XRP/USD"), &clean_bars(10))
        .expect("put");

    // Simulate a crash mid-put: a half-written staging directory survives.
    let orphan = store.staging_dir().join("crashed-1234");
    std::fs::create_dir_all(&orphan).expect("create orphan");
    std::fs::write(orphan.join("manifest.json"), b"{").expect("write orphan");

    let listed = store.list(64).expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].dataset_id, stored.manifest.dataset_id);

    // The next successful put sweeps it.
    store
        .build_and_put(&input("ADA/USD"), &clean_bars(10))
        .expect("put");
    assert!(!orphan.exists(), "stale staging directory was not swept");
}

#[cfg(unix)]
#[test]
fn a_failed_put_publishes_nothing() {
    use std::os::unix::fs::PermissionsExt;

    // Running as root would make the read-only root writable anyway.
    if effective_uid() == 0 {
        return;
    }

    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let bars = clean_bars(10);
    let manifest = DatasetManifest::build(&input("DOT/USD"), &bars).expect("manifest");

    // The layout directory is where staging and shard directories are created,
    // so making *it* read-only is what fails the put mid-flight.
    let layout = temp.path().join(DATASET_STORE_LAYOUT_VERSION);
    let mut permissions = std::fs::metadata(&layout).expect("metadata").permissions();
    permissions.set_mode(0o500); // r-x — no writes below this point
    std::fs::set_permissions(&layout, permissions).expect("chmod");

    let outcome = store.build_and_put(&input("DOT/USD"), &bars);

    let mut restore = std::fs::metadata(&layout).expect("metadata").permissions();
    restore.set_mode(0o700);
    std::fs::set_permissions(&layout, restore).expect("restore chmod");

    assert!(
        matches!(outcome, Err(DatasetStoreError::Io { .. })),
        "expected an I/O failure, got {outcome:?}"
    );
    assert!(
        !store.record_dir(&manifest.dataset_id).exists(),
        "a failed put must not publish a record directory"
    );
    assert!(matches!(
        store.open_record(&manifest.dataset_id),
        Err(DatasetStoreError::NotFound { .. })
    ));
}

#[cfg(unix)]
fn effective_uid() -> u32 {
    // `std` exposes no euid, and pulling a libc dependency in for one test is
    // not worth it: reading the process status file is portable enough here.
    // An unreadable status file reports a non-root uid, so the test still runs.
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                line.strip_prefix("Uid:")
                    .and_then(|rest| rest.split_whitespace().nth(1)?.parse().ok())
            })
        })
        .unwrap_or(1)
}

// ── Integrity ──────────────────────────────────────────────────────

#[test]
fn a_tampered_payload_is_refused() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let stored = store
        .build_and_put(&input("BTC/USD"), &clean_bars(20))
        .expect("put");

    let payload = store
        .record_dir(&stored.manifest.dataset_id)
        .join("bars.bin");
    let mut bytes = std::fs::read(&payload).expect("read payload");
    // Flip one bit deep inside a price field.
    let midpoint = bytes.len() / 2;
    bytes[midpoint] ^= 0x01;
    std::fs::write(&payload, &bytes).expect("write payload");

    assert!(
        matches!(
            store.open_record(&stored.manifest.dataset_id),
            Err(DatasetStoreError::CorruptPayload { .. })
        ),
        "a flipped payload bit must be caught on open"
    );
}

#[test]
fn a_tampered_manifest_is_refused() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let stored = store
        .build_and_put(&input("BTC/USD"), &clean_bars(20))
        .expect("put");

    let path = store
        .record_dir(&stored.manifest.dataset_id)
        .join("manifest.json");
    let mut manifest = stored.manifest.clone();
    manifest.symbol = "ETH/USD".to_string();
    std::fs::write(&path, serde_json::to_vec(&manifest).expect("serialize")).expect("write");

    assert!(
        matches!(
            store.open_record(&stored.manifest.dataset_id),
            Err(DatasetStoreError::Dataset(_))
        ),
        "an edited manifest must not open"
    );
}

#[test]
fn a_tampered_qa_report_is_refused() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let mut bars = clean_bars(20);
    bars[3].high = bars[3].low * 0.5; // a genuine finding to erase
    let stored = store.build_and_put(&input("BTC/USD"), &bars).expect("put");
    assert!(stored.qa.has_errors());

    let path = store
        .record_dir(&stored.manifest.dataset_id)
        .join("qa.json");
    let mut qa = stored.qa.clone();
    qa.findings.clear();
    std::fs::write(&path, serde_json::to_vec(&qa).expect("serialize")).expect("write");

    assert!(
        matches!(
            store.open_record(&stored.manifest.dataset_id),
            Err(DatasetStoreError::Dataset(
                crate::core::strategy_dataset::DatasetError::QaReportHashMismatch { .. }
            ))
        ),
        "an edited QA report must not open"
    );
}

#[test]
fn oversized_and_malformed_artifacts_are_refused_before_decoding() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let stored = store
        .build_and_put(&input("BTC/USD"), &clean_bars(10))
        .expect("put");
    let dir = store.record_dir(&stored.manifest.dataset_id);

    std::fs::write(
        dir.join("manifest.json"),
        vec![b'x'; MAX_MANIFEST_JSON_BYTES as usize + 1],
    )
    .expect("write");
    assert!(matches!(
        store.open_record(&stored.manifest.dataset_id),
        Err(DatasetStoreError::ArtifactTooLarge {
            artifact: "manifest.json",
            ..
        })
    ));

    // An unknown field must not be silently ignored — a manifest is sealed.
    let mut value: serde_json::Value =
        serde_json::to_value(&stored.manifest).expect("manifest to value");
    value["surprise"] = serde_json::Value::Bool(true);
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_vec(&value).expect("serialize"),
    )
    .expect("write");
    assert!(matches!(
        store.open_record(&stored.manifest.dataset_id),
        Err(DatasetStoreError::InvalidArtifact { .. })
    ));
}

#[test]
fn a_truncated_or_garbage_payload_is_refused() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let stored = store
        .build_and_put(&input("BTC/USD"), &clean_bars(20))
        .expect("put");
    let payload = store
        .record_dir(&stored.manifest.dataset_id)
        .join("bars.bin");
    let original = std::fs::read(&payload).expect("read");

    for (label, bytes) in [
        ("empty", Vec::new()),
        ("header only", original[..8.min(original.len())].to_vec()),
        ("truncated", original[..original.len() / 2].to_vec()),
        ("garbage", vec![0xffu8; original.len()]),
    ] {
        std::fs::write(&payload, &bytes).expect("write");
        let outcome = store.open_record(&stored.manifest.dataset_id);
        assert!(
            matches!(outcome, Err(DatasetStoreError::CorruptPayload { .. })),
            "{label} payload was accepted: {outcome:?}"
        );
    }
}

#[test]
fn dataset_ids_are_validated_before_the_filesystem_is_touched() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());

    for hostile in [
        "",
        "..",
        "../../etc/passwd",
        "a/b",
        "ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        &"0".repeat(63),
        &"0".repeat(65),
        "0000000000000000000000000000000000000000000000000000000000000g0",
    ] {
        assert!(
            matches!(
                store.open_record(hostile),
                Err(DatasetStoreError::InvalidDatasetId { .. })
            ),
            "{hostile:?} was not rejected"
        );
        assert!(matches!(
            store.contains(hostile),
            Err(DatasetStoreError::InvalidDatasetId { .. })
        ));
    }

    // A well-formed but absent id is a miss, not a validation error.
    assert!(matches!(
        store.open_record(&"0".repeat(64)),
        Err(DatasetStoreError::NotFound { .. })
    ));
    assert!(!store.contains(&"0".repeat(64)).expect("contains"));
}

// ── Hostile filesystem boundary (Linux) ────────────────────────────

#[cfg(target_os = "linux")]
#[test]
fn store_bootstrap_rejects_symlink_ancestor_without_creating_external_suffix() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside");
    let trusted = temp.path().join("trusted");
    std::fs::create_dir(&trusted).expect("trusted parent");
    symlink(outside.path(), trusted.join("escape")).expect("ancestor symlink");

    let root = trusted.join("escape/new/leaf");
    assert!(FileDatasetStore::open(&root).is_err());
    assert!(
        !outside.path().join("new").exists(),
        "bootstrap followed an ancestor symlink and wrote outside its boundary"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn store_open_rejects_a_symlinked_layout_directory() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside");
    symlink(
        outside.path(),
        temp.path().join(DATASET_STORE_LAYOUT_VERSION),
    )
    .expect("symlink");
    assert!(FileDatasetStore::open(temp.path()).is_err());
}

#[cfg(target_os = "linux")]
#[test]
fn put_rejects_a_symlinked_shard_without_writing_outside_root() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside");
    let store = store_at(temp.path());
    let bars = clean_bars(3);
    let manifest = DatasetManifest::build(&input("LINK/USD"), &bars).expect("manifest");
    symlink(
        outside.path(),
        temp.path()
            .join(DATASET_STORE_LAYOUT_VERSION)
            .join(&manifest.dataset_id[..2]),
    )
    .expect("shard symlink");

    assert!(store.build_and_put(&input("LINK/USD"), &bars).is_err());
    assert_eq!(
        std::fs::read_dir(outside.path())
            .expect("outside list")
            .count(),
        0
    );
}

#[cfg(target_os = "linux")]
#[test]
fn open_rejects_a_symlinked_record_directory() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside");
    let store = store_at(temp.path());
    let id = "ab".to_string() + &"0".repeat(62);
    let shard = temp.path().join(DATASET_STORE_LAYOUT_VERSION).join("ab");
    std::fs::create_dir(&shard).expect("shard");
    symlink(outside.path(), shard.join(&id)).expect("record symlink");
    assert!(store.open_record(&id).is_err());
}

#[cfg(target_os = "linux")]
#[test]
fn open_rejects_symlinked_manifest_qa_and_payload_files() {
    use std::os::unix::fs::symlink;
    for artifact in [MANIFEST_FILE, QA_FILE, PAYLOAD_FILE] {
        let temp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside");
        let store = store_at(temp.path());
        let stored = store
            .build_and_put(&input("LINK/USD"), &clean_bars(3))
            .expect("put");
        let artifact_path = store.record_dir(&stored.manifest.dataset_id).join(artifact);
        let outside_path = outside.path().join(artifact);
        std::fs::rename(&artifact_path, &outside_path).expect("move artifact");
        symlink(&outside_path, &artifact_path).expect("artifact symlink");
        assert!(
            store.open_record(&stored.manifest.dataset_id).is_err(),
            "symlinked {artifact} was accepted"
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn bounded_read_uses_one_handle_when_the_path_is_replaced_after_open() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let first_bars = clean_bars(3);
    let first = store
        .build_and_put(&input("FIRST/USD"), &first_bars)
        .expect("first put");
    let record = store
        .open_record(&first.manifest.dataset_id)
        .expect("open first");

    let mut replacement_bars = first_bars.clone();
    replacement_bars[0].open += 10.0;
    let replacement = encode_bar_payload(&replacement_bars).expect("replacement payload");
    let payload_path = store
        .record_dir(&first.manifest.dataset_id)
        .join(PAYLOAD_FILE);
    std::fs::rename(&payload_path, payload_path.with_extension("old")).expect("move old");
    std::fs::write(&payload_path, replacement).expect("write replacement");

    let page = record.read_page(0, 3).expect("page stays on opened inode");
    assert_bit_identical(&first_bars, &page.bars);
}

#[cfg(target_os = "linux")]
#[test]
fn post_open_path_swap_hook_reads_only_the_descriptor_pinned_inode() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("artifact");
    std::fs::write(&path, b"pinned").expect("seed");
    let file = std::fs::File::open(&path).expect("open before swap");
    let observed = std::sync::atomic::AtomicU64::new(0);
    let bytes = read_bounded_file_with_hook(
        &file,
        "test",
        32,
        || {
            std::fs::rename(&path, temp.path().join("opened-inode")).expect("move opened inode");
            std::fs::write(&path, b"replacement").expect("install replacement");
        },
        &observed,
    )
    .expect("read pinned descriptor");
    assert_eq!(bytes, b"pinned");
    assert_eq!(std::fs::read(&path).expect("replacement"), b"replacement");
}

#[cfg(target_os = "linux")]
#[test]
fn a_file_that_grows_during_bounded_read_never_reads_past_limit_plus_one() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("growing");
    std::fs::write(&path, b"1234").expect("seed");
    let file = std::fs::File::open(&path).expect("open");
    let observed = std::sync::atomic::AtomicU64::new(0);
    let result = read_bounded_file_with_hook(
        &file,
        "test",
        4,
        || std::fs::write(&path, vec![b'x'; 128]).expect("grow"),
        &observed,
    );
    assert!(matches!(
        result,
        Err(DatasetStoreError::ArtifactTooLarge { .. })
    ));
    assert!(observed.load(std::sync::atomic::Ordering::Relaxed) <= 5);
}

#[cfg(target_os = "linux")]
#[test]
fn listing_retains_at_most_the_requested_number_of_candidate_names() {
    let names = (0..10_000).map(|index| format!("{index:064x}"));
    let high_water = std::sync::atomic::AtomicUsize::new(0);
    let retained = retain_smallest_names(names, 7, &high_water);
    assert_eq!(retained.len(), 7);
    assert!(high_water.load(std::sync::atomic::Ordering::Relaxed) <= 7);
}

#[cfg(target_os = "linux")]
#[test]
fn first_publication_syncs_layout_before_publishing_and_shard_after_rename() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    publication_trace_begin();
    let stored = store
        .build_and_put(&input("DURABLE/USD"), &clean_bars(3))
        .expect("put");
    assert_eq!(stored.outcome, DatasetPutOutcome::Stored);
    let trace = publication_trace_end();
    let staging_sync = trace
        .iter()
        .position(|event| *event == "sync(staging)")
        .unwrap();
    let shard_create = trace
        .iter()
        .position(|event| *event == "create(shard)")
        .unwrap();
    let layout_sync = trace
        .iter()
        .position(|event| *event == "sync(layout)")
        .unwrap();
    let rename = trace
        .iter()
        .position(|event| *event == "rename(record)")
        .unwrap();
    let shard_sync = trace
        .iter()
        .position(|event| *event == "sync(shard)")
        .unwrap();
    assert!(staging_sync < shard_create);
    assert!(shard_create < layout_sync);
    assert!(layout_sync < rename);
    assert!(rename < shard_sync);
}

// ── Bounded paging ─────────────────────────────────────────────────

#[test]
fn pages_are_bounded_windows_over_the_payload() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let bars = clean_bars(250);
    let stored = store.build_and_put(&input("BTC/USD"), &bars).expect("put");
    let record = store
        .open_record(&stored.manifest.dataset_id)
        .expect("open");

    let page = record.read_page(0, 100).expect("first page");
    assert_eq!(page.offset, 0);
    assert_eq!(page.total_bars, 250);
    assert_bit_identical(&bars[..100], &page.bars);

    let page = record.read_page(100, 100).expect("middle page");
    assert_bit_identical(&bars[100..200], &page.bars);

    // The final page is short, not an error.
    let page = record.read_page(200, 100).expect("last page");
    assert_eq!(page.bars.len(), 50);
    assert_bit_identical(&bars[200..], &page.bars);

    // Walking every page reconstructs the dataset exactly.
    let mut walked = Vec::new();
    let mut offset = 0u64;
    while offset < page.total_bars {
        let chunk = record.read_page(offset, 64).expect("page");
        assert!(chunk.bars.len() <= 64);
        offset += chunk.bars.len() as u64;
        walked.extend(chunk.bars);
    }
    assert_bit_identical(&bars, &walked);
}

#[test]
fn pages_carry_only_their_own_qa_findings() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let mut bars = clean_bars(120);
    bars[5].high = bars[5].low * 0.5; // defect inside [0, 50)
    bars[110].high = bars[110].low * 0.5; // defect inside [100, 120)
    let stored = store.build_and_put(&input("BTC/USD"), &bars).expect("put");
    let record = store
        .open_record(&stored.manifest.dataset_id)
        .expect("open");

    let first = record.read_page(0, 50).expect("page");
    assert!(!first.findings.is_empty());
    assert!(
        first.findings.iter().all(|f| f.bar_index == Some(5)),
        "{:?}",
        first.findings
    );

    // The middle window sits between both defects and must come back clean.
    let second = record.read_page(50, 50).expect("page");
    assert!(second.findings.is_empty(), "{:?}", second.findings);
    let repeated = record.read_page(50, 50).expect("page");
    assert_eq!(
        second.findings, repeated.findings,
        "paging is deterministic"
    );

    let last = record.read_page(100, 20).expect("page");
    assert!(!last.findings.is_empty());
    assert!(
        last.findings.iter().all(|f| f.bar_index == Some(110)),
        "{:?}",
        last.findings
    );

    // Findings never leak across the window boundary in either direction.
    let boundary = record.read_page(100, 5).expect("page");
    assert!(boundary.findings.is_empty(), "{:?}", boundary.findings);
}

#[test]
fn hostile_page_requests_are_rejected_not_clamped() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let stored = store
        .build_and_put(&input("BTC/USD"), &clean_bars(10))
        .expect("put");
    let record = store
        .open_record(&stored.manifest.dataset_id)
        .expect("open");

    assert!(matches!(
        record.read_page(0, MAX_PAGE_BARS + 1),
        Err(DatasetStoreError::PageTooLarge { .. })
    ));
    assert!(matches!(
        record.read_page(0, usize::MAX),
        Err(DatasetStoreError::PageTooLarge { .. })
    ));
    assert!(matches!(
        record.read_page(0, 0),
        Err(DatasetStoreError::PageTooLarge { .. })
    ));
    assert!(matches!(
        record.read_page(10, 10),
        Err(DatasetStoreError::PageOutOfRange { .. })
    ));
    assert!(matches!(
        record.read_page(u64::MAX, 10),
        Err(DatasetStoreError::PageOutOfRange { .. })
    ));
}

// ── Listing ────────────────────────────────────────────────────────

#[test]
fn listing_is_bounded_and_deterministically_ordered() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let mut ids = Vec::new();
    for index in 0..6 {
        let stored = store
            .build_and_put(&input(&format!("SYM{index}/USD")), &clean_bars(5 + index))
            .expect("put");
        ids.push(stored.manifest.dataset_id);
    }

    let all = store.list(64).expect("list");
    assert_eq!(all.len(), 6);
    let listed: Vec<&str> = all.iter().map(|s| s.dataset_id.as_str()).collect();
    let mut sorted = listed.clone();
    sorted.sort_unstable();
    assert_eq!(listed, sorted, "listing must be id-ordered");

    let bounded = store.list(2).expect("bounded list");
    assert_eq!(bounded.len(), 2);
    assert_eq!(bounded[0].dataset_id, all[0].dataset_id);

    assert!(matches!(
        store.list(MAX_LISTED_RECORDS + 1),
        Err(DatasetStoreError::ListLimitTooLarge { .. })
    ));

    // Summaries carry the provenance and QA headline the inspector shows,
    // without opening the payload.
    let summary = &all[0];
    assert_eq!(summary.timeframe, "1Day");
    assert_eq!(summary.source, "kraken");
    assert_eq!(summary.venue, "kraken-spot");
    assert_eq!(summary.pipeline, "cache-merge/v1");
    assert_eq!(summary.adjustment, AdjustmentPolicy::Raw);
    assert_eq!(
        summary.calendar_policy_id,
        CalendarPolicy::Continuous24x7.policy_id()
    );
    assert!(summary.bar_count >= 5);
    assert!(!summary.qa_findings_truncated);
}

#[test]
fn listing_rejects_a_manifest_stored_under_a_different_dataset_id() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let stored = store
        .build_and_put(&input("BOUND/USD"), &clean_bars(3))
        .expect("put");
    let wrong_id = if stored.manifest.dataset_id.starts_with("00") {
        format!("ff{}", &stored.manifest.dataset_id[2..])
    } else {
        format!("00{}", &stored.manifest.dataset_id[2..])
    };
    let wrong_dir = store.record_dir(&wrong_id);
    std::fs::create_dir_all(&wrong_dir).expect("wrong record directory");
    std::fs::copy(
        store
            .record_dir(&stored.manifest.dataset_id)
            .join(MANIFEST_FILE),
        wrong_dir.join(MANIFEST_FILE),
    )
    .expect("copy sealed manifest");

    assert!(matches!(
        store.list(MAX_LISTED_RECORDS),
        Err(DatasetStoreError::InvalidArtifact { .. })
    ));
}

// ── Encoding ────────────────────────────────────────────────────────

#[test]
fn the_payload_encoder_refuses_input_it_cannot_represent() {
    let too_long = "2024-01-01T00:00:00Z".repeat(64);
    assert!(matches!(
        encode_bar_payload(&[bar(&too_long, 1.0, 1.0, 1.0, 1.0, 1.0)]),
        Err(DatasetStoreError::CorruptPayload { .. })
    ));
    assert!(matches!(
        encode_bar_payload(&[bar("2024-01-01T00:00:00Z", f64::NAN, 1.0, 1.0, 1.0, 1.0)]),
        Err(DatasetStoreError::CorruptPayload { .. })
    ));
}

#[test]
fn the_payload_round_trips_through_its_own_codec() {
    let bars = awkward_bars();
    let encoded = encode_bar_payload(&bars).expect("encode");
    let decoded = decode_bar_payload(&encoded).expect("decode");
    assert_bit_identical(&bars, &decoded);

    // Empty is a legal dataset and must round-trip too.
    let encoded = encode_bar_payload(&[]).expect("encode empty");
    assert!(
        decode_bar_payload(&encoded)
            .expect("decode empty")
            .is_empty()
    );
}

#[test]
fn storing_more_bars_than_the_cap_is_refused() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let bars = clean_bars(4);
    let manifest = DatasetManifest::build(&input("BTC/USD"), &bars).expect("manifest");
    let qa = manifest.run_qa(&bars);

    // Forge an over-cap bar count rather than allocating four million bars.
    let mut oversized = manifest.clone();
    oversized.bar_count = MAX_STORED_BARS + 1;
    assert!(matches!(
        store.put(&oversized, &qa, &bars),
        Err(DatasetStoreError::TooManyBars { .. })
    ));
}

// ── Final-holdout split (ADR-135 §7.8) ─────────────────────────────

#[test]
fn a_final_holdout_split_hands_out_only_the_search_partition() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let bars = clean_bars(20);
    let stored = store
        .build_and_put(&input("BTC/USD"), &bars)
        .expect("dataset stores");

    let split = store
        .split_final_holdout(&stored.manifest.dataset_id, 5)
        .expect("split");
    let artifact = split.artifact();
    artifact.verify().expect("split verifies");

    assert_eq!(artifact.parent_dataset_id(), stored.manifest.dataset_id);
    assert_eq!(artifact.parent_manifest_id(), stored.manifest.manifest_id);
    assert_eq!(artifact.parent_bar_count(), 20);
    assert_eq!(artifact.range(), 15..20);
    assert_eq!(artifact.symbol(), "BTC/USD");
    assert_eq!(artifact.timeframe(), "1Day");
    assert_eq!(artifact.holdout_first_timestamp(), bars[15].timestamp);
    assert_eq!(artifact.holdout_last_timestamp(), bars[19].timestamp);

    // The caller receives the search partition, byte for byte, and nothing else.
    assert_bit_identical(&bars[..15], split.search_bars());
    assert_eq!(
        split.search_manifest().dataset_id,
        artifact.search_dataset_id()
    );
    assert_eq!(
        split.search_manifest().manifest_id,
        artifact.search_manifest_id()
    );

    // Only the store can turn the split back into holdout bars.
    let (holdout_manifest, holdout_bars) = store
        .materialize_final_holdout(artifact)
        .expect("holdout materializes");
    assert_bit_identical(&bars[15..], &holdout_bars);
    assert_eq!(holdout_manifest.dataset_id, artifact.holdout_dataset_id());
    assert_eq!(holdout_manifest.manifest_id, artifact.holdout_manifest_id());

    // Re-splitting the same stored parent is deterministic.
    let again = store
        .split_final_holdout(&stored.manifest.dataset_id, 5)
        .expect("split");
    assert_eq!(again.artifact().split_id(), artifact.split_id());

    // A different cut is a different split.
    let moved = store
        .split_final_holdout(&stored.manifest.dataset_id, 4)
        .expect("split");
    assert_ne!(moved.artifact().split_id(), artifact.split_id());
}

#[test]
fn a_relative_store_root_is_retained_as_an_absolute_authority_anchor() {
    let current = std::env::current_dir().expect("current directory");
    let temporary = tempfile::Builder::new()
        .prefix("relative-dataset-store-")
        .tempdir_in(&current)
        .expect("temporary directory under current directory");
    let relative = temporary
        .path()
        .strip_prefix(&current)
        .expect("temporary path is beneath current directory");

    let store = FileDatasetStore::open(relative).expect("relative dataset store");

    assert!(store.root().is_absolute());
    assert_eq!(store.root(), temporary.path());
}

#[test]
fn splitting_and_materializing_fail_closed_on_forged_foreign_and_impossible_partitions() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = store_at(temp.path());
    let bars = clean_bars(12);
    let stored = store
        .build_and_put(&input("BTC/USD"), &bars)
        .expect("dataset stores");
    let split = store
        .split_final_holdout(&stored.manifest.dataset_id, 3)
        .expect("split");

    // A holdout that is not a proper suffix of the parent is not a partition.
    for holdout in [0, 12, 13] {
        assert!(
            store
                .split_final_holdout(&stored.manifest.dataset_id, holdout)
                .is_err()
        );
    }
    assert!(matches!(
        store.split_final_holdout(&"c".repeat(64), 3),
        Err(DatasetStoreError::NotFound { .. })
    ));
    assert!(matches!(
        store.split_final_holdout("not-a-dataset-id", 3),
        Err(DatasetStoreError::InvalidDatasetId { .. })
    ));

    // A re-addressed artifact content-addresses itself, so `verify` alone would accept it. The
    // store re-seals from the stored parent instead, and refuses.
    let forged = split.artifact().test_only_forged(&"c".repeat(64));
    forged.verify().expect("a forged split is self-consistent");
    assert!(store.materialize_final_holdout(&forged).is_err());

    // A split minted from another store's dataset names a parent this store does not hold.
    let other = tempfile::tempdir().expect("tempdir");
    let other_store = store_at(other.path());
    let other_bars = clean_bars(11);
    let other_stored = other_store
        .build_and_put(&input("ETH/USD"), &other_bars)
        .expect("dataset stores");
    let foreign = other_store
        .split_final_holdout(&other_stored.manifest.dataset_id, 3)
        .expect("split");
    assert!(matches!(
        store.materialize_final_holdout(foreign.artifact()),
        Err(DatasetStoreError::NotFound { .. })
    ));
    // And the store that does hold it still materializes exactly its own holdout.
    let (_, foreign_bars) = other_store
        .materialize_final_holdout(foreign.artifact())
        .expect("holdout materializes");
    assert_bit_identical(&other_bars[8..], &foreign_bars);
}
