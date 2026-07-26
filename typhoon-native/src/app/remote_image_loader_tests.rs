use super::remote_image_loader::{Cache, append_bounded, is_public_ip, supports_remote_image_uri};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[test]
fn remote_image_loader_accepts_only_http_urls() {
    assert!(supports_remote_image_uri("https://cdn.example/image.webp"));
    assert!(supports_remote_image_uri("http://cdn.example/image.png"));
    assert!(!supports_remote_image_uri("file:///tmp/image.png"));
    assert!(!supports_remote_image_uri("data:image/png;base64,AA=="));
    assert!(!supports_remote_image_uri("http://127.0.0.1/image.png"));
    assert!(!supports_remote_image_uri(
        "http://169.254.169.254/latest/meta-data"
    ));
    assert!(!supports_remote_image_uri("https://localhost/image.png"));
    assert!(!supports_remote_image_uri("http://[::1]/image.png"));
    assert!(!supports_remote_image_uri("https://[fc00::1]/image.png"));
    assert!(!supports_remote_image_uri(
        "http://[::ffff:127.0.0.1]/image.png"
    ));
    assert!(supports_remote_image_uri(
        "https://[2606:2800:220:1:248:1893:25c8:1946]/image.png"
    ));
    assert!(!supports_remote_image_uri(
        "https://cdn.example:8443/image.png"
    ));
}

#[test]
fn remote_image_loader_rejects_non_public_destinations() {
    assert!(!is_public_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
    assert!(!is_public_ip(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1))));
    assert!(!is_public_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
    assert!(!is_public_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));
    assert!(!is_public_ip(
        "fc00::1".parse().expect("valid private IPv6")
    ));
    assert!(is_public_ip(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))));
    assert!(is_public_ip(
        "2606:2800:220:1:248:1893:25c8:1946"
            .parse()
            .expect("valid public IPv6")
    ));
}

#[test]
fn remote_image_loader_reserves_in_flight_memory_before_starting_requests() {
    let mut cache = Cache::default();
    let mut request_ids = Vec::new();
    for index in 0..4 {
        request_ids.push(
            cache
                .reserve_request(format!("https://cdn.example/{index}.png"))
                .expect("request should fit reserved capacity"),
        );
    }
    assert!(
        cache
            .reserve_request("https://cdn.example/overflow.png".to_owned())
            .is_none()
    );

    cache.remove("https://cdn.example/0.png");
    assert!(
        cache
            .reserve_request("https://cdn.example/replacement.png".to_owned())
            .is_none(),
        "forgetting an entry must not release its running download reservation"
    );
    assert!(!cache.finish(
        "https://cdn.example/0.png",
        request_ids[0],
        Err("forgotten request completed".to_owned())
    ));
    assert!(
        cache
            .reserve_request("https://cdn.example/replacement.png".to_owned())
            .is_some()
    );
}

#[test]
fn forgotten_request_cannot_overwrite_a_reloaded_uri() {
    let uri = "https://cdn.example/reused.png";
    let mut cache = Cache::default();
    let old_request = cache
        .reserve_request(uri.to_owned())
        .expect("old request should fit");
    cache.remove(uri);
    let new_request = cache
        .reserve_request(uri.to_owned())
        .expect("replacement request should fit beside the forgotten download");

    assert!(!cache.finish(uri, old_request, Err("stale result".to_owned())));
    assert!(cache.finish(uri, new_request, Err("current result".to_owned())));
}

#[test]
fn forget_all_keeps_running_downloads_reserved_until_completion() {
    let mut cache = Cache::default();
    let requests: Vec<_> = (0..4)
        .map(|index| {
            let uri = format!("https://cdn.example/{index}.png");
            let request_id = cache
                .reserve_request(uri.clone())
                .expect("request should fit");
            (uri, request_id)
        })
        .collect();

    cache.forget_all();
    assert!(
        cache
            .reserve_request("https://cdn.example/blocked.png".to_owned())
            .is_none()
    );
    assert!(!cache.finish(
        &requests[0].0,
        requests[0].1,
        Err("forgotten request completed".to_owned())
    ));
    assert!(
        cache
            .reserve_request("https://cdn.example/replacement.png".to_owned())
            .is_some()
    );
}

#[test]
fn remote_image_loader_rejects_a_body_that_exceeds_its_limit() {
    let mut body = vec![1, 2, 3, 4];
    append_bounded(&mut body, &[5, 6, 7, 8], 8).expect("body at limit should fit");
    let error = append_bounded(&mut body, &[9], 8).expect_err("oversized body must fail");

    assert_eq!(body, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    assert!(error.contains("8-byte limit"));
}
