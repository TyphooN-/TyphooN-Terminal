use std::cell::Cell;
use std::sync::Arc;

use super::refresh_arc_slice_cache;

#[test]
fn unchanged_cache_key_reuses_arc_without_rebuilding() {
    let original: Arc<[i32]> = vec![1, 2, 3].into();
    let mut cached = Arc::clone(&original);
    let mut cached_key = Some((7_u64, "all"));
    let builds = Cell::new(0);

    refresh_arc_slice_cache(&mut cached, &mut cached_key, (7, "all"), || {
        builds.set(builds.get() + 1);
        vec![9]
    });

    assert!(Arc::ptr_eq(&cached, &original));
    assert_eq!(builds.get(), 0);
}

#[test]
fn changed_cache_key_replaces_arc_even_when_length_matches() {
    let original: Arc<[i32]> = vec![1, 2].into();
    let mut cached = Arc::clone(&original);
    let mut cached_key = Some((7_u64, "all"));

    refresh_arc_slice_cache(&mut cached, &mut cached_key, (8, "all"), || vec![3, 4]);

    assert!(!Arc::ptr_eq(&cached, &original));
    assert_eq!(cached.as_ref(), [3, 4]);
    assert_eq!(cached_key, Some((8, "all")));
}
