use egui::load::{Bytes, BytesLoadResult, BytesLoader, BytesPoll, LoadError};
use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::Arc;

const MAX_REMOTE_IMAGE_BYTES: usize = 8 * 1024 * 1024;
const MAX_REMOTE_IMAGE_CACHE_BYTES: usize = 32 * 1024 * 1024;
const MAX_REMOTE_IMAGE_ENTRIES: usize = 64;

#[derive(Clone)]
pub(super) struct RemoteFile {
    bytes: Arc<[u8]>,
    mime: Option<String>,
}

#[derive(Clone)]
enum Entry {
    Pending(u64),
    Ready(Result<RemoteFile, String>),
}

#[derive(Default)]
pub(super) struct Cache {
    entries: HashMap<String, Entry>,
    insertion_order: VecDeque<String>,
    ready_bytes: usize,
    pending_bytes: usize,
    next_request_id: u64,
}

impl Cache {
    pub(super) fn remove(&mut self, uri: &str) {
        match self.entries.remove(uri) {
            Some(Entry::Ready(Ok(file))) => {
                self.ready_bytes = self.ready_bytes.saturating_sub(file.bytes.len());
            }
            // Forgetting the UI entry does not cancel the spawned download. Its
            // reservation remains until that exact request completes.
            Some(Entry::Pending(_)) => {}
            Some(Entry::Ready(Err(_))) | None => {}
        }
        self.insertion_order.retain(|cached| cached != uri);
    }

    fn evict_oldest_ready(&mut self) -> bool {
        let Some(index) = self
            .insertion_order
            .iter()
            .position(|uri| matches!(self.entries.get(uri), Some(Entry::Ready(_))))
        else {
            return false;
        };
        if let Some(uri) = self.insertion_order.remove(index) {
            if let Some(Entry::Ready(Ok(file))) = self.entries.remove(&uri) {
                self.ready_bytes = self.ready_bytes.saturating_sub(file.bytes.len());
            }
        }
        true
    }

    pub(super) fn reserve_request(&mut self, uri: String) -> Option<u64> {
        while self.entries.len() >= MAX_REMOTE_IMAGE_ENTRIES
            || self
                .ready_bytes
                .checked_add(self.pending_bytes)
                .and_then(|used| used.checked_add(MAX_REMOTE_IMAGE_BYTES))
                .is_none_or(|reserved| reserved > MAX_REMOTE_IMAGE_CACHE_BYTES)
        {
            if !self.evict_oldest_ready() {
                return None;
            }
        }
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1);
        self.pending_bytes += MAX_REMOTE_IMAGE_BYTES;
        self.entries.insert(uri.clone(), Entry::Pending(request_id));
        self.insertion_order.push_back(uri);
        Some(request_id)
    }

    pub(super) fn finish(
        &mut self,
        uri: &str,
        request_id: u64,
        result: Result<RemoteFile, String>,
    ) -> bool {
        self.pending_bytes = self.pending_bytes.saturating_sub(MAX_REMOTE_IMAGE_BYTES);
        if !matches!(self.entries.get(uri), Some(Entry::Pending(current)) if *current == request_id)
        {
            return false;
        }
        if let Ok(file) = &result {
            self.ready_bytes = self.ready_bytes.saturating_add(file.bytes.len());
        }
        self.entries.insert(uri.to_owned(), Entry::Ready(result));
        while self.ready_bytes > MAX_REMOTE_IMAGE_CACHE_BYTES {
            if !self.evict_oldest_ready() {
                break;
            }
        }
        true
    }

    pub(super) fn forget_all(&mut self) {
        self.entries.clear();
        self.insertion_order.clear();
        self.ready_bytes = 0;
    }
}

pub(super) struct RemoteImageLoader {
    cache: Arc<egui::mutex::Mutex<Cache>>,
    client: reqwest::Client,
    runtime: tokio::runtime::Handle,
}

impl RemoteImageLoader {
    pub(super) const ID: &'static str = egui::generate_loader_id!(RemoteImageLoader);

    pub(super) fn new(runtime: tokio::runtime::Handle) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 5 {
                    attempt.error("too many remote image redirects")
                } else if is_safe_remote_url(attempt.url()) {
                    attempt.follow()
                } else {
                    attempt.error("remote image redirect destination is not public")
                }
            }))
            .dns_resolver(PublicDnsResolver)
            .no_proxy()
            .user_agent("TyphooN-Terminal/news-image-loader")
            .build()
            .expect("static remote image HTTP client configuration must be valid");
        Self {
            cache: Arc::default(),
            client,
            runtime,
        }
    }
}

impl BytesLoader for RemoteImageLoader {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, ctx: &egui::Context, uri: &str) -> BytesLoadResult {
        if !supports_remote_image_uri(uri) {
            return Err(LoadError::NotSupported);
        }

        let mut cache = self.cache.lock();
        if let Some(entry) = cache.entries.get(uri).cloned() {
            return match entry {
                Entry::Pending(_) => Ok(BytesPoll::Pending { size: None }),
                Entry::Ready(Ok(file)) => Ok(BytesPoll::Ready {
                    size: None,
                    bytes: Bytes::Shared(file.bytes),
                    mime: file.mime,
                }),
                Entry::Ready(Err(error)) => Err(LoadError::Loading(error)),
            };
        }
        let uri = uri.to_owned();
        let Some(request_id) = cache.reserve_request(uri.clone()) else {
            return Err(LoadError::Loading(
                "remote image request limit reached".to_owned(),
            ));
        };
        drop(cache);

        let cache = Arc::clone(&self.cache);
        let client = self.client.clone();
        let ctx = ctx.clone();
        self.runtime.spawn(async move {
            let result = fetch_remote_image(&client, &uri).await;
            let repaint = cache.lock().finish(&uri, request_id, result);
            if repaint {
                ctx.request_repaint();
            }
        });

        Ok(BytesPoll::Pending { size: None })
    }

    fn forget(&self, uri: &str) {
        self.cache.lock().remove(uri);
    }

    fn forget_all(&self) {
        self.cache.lock().forget_all();
    }

    fn byte_size(&self) -> usize {
        self.cache.lock().ready_bytes
    }

    fn has_pending(&self) -> bool {
        self.cache.lock().pending_bytes != 0
    }
}

async fn fetch_remote_image(client: &reqwest::Client, uri: &str) -> Result<RemoteFile, String> {
    let mut response = client
        .get(uri)
        .send()
        .await
        .map_err(|_| "remote image request failed".to_owned())?;
    if !response.status().is_success() {
        return Err(format!(
            "remote image request failed with HTTP {}",
            response.status()
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_REMOTE_IMAGE_BYTES as u64)
    {
        return Err(format!(
            "remote image exceeds {}-byte limit",
            MAX_REMOTE_IMAGE_BYTES
        ));
    }

    let mime = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(';').next().unwrap_or(value).trim().to_owned())
        .filter(|value| value.starts_with("image/"));
    let mut body = Vec::with_capacity(
        response
            .content_length()
            .unwrap_or(0)
            .min(MAX_REMOTE_IMAGE_BYTES as u64) as usize,
    );
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "remote image body read failed".to_owned())?
    {
        append_bounded(&mut body, &chunk, MAX_REMOTE_IMAGE_BYTES)?;
    }

    Ok(RemoteFile {
        bytes: body.into(),
        mime,
    })
}

pub(super) fn supports_remote_image_uri(uri: &str) -> bool {
    reqwest::Url::parse(uri)
        .ok()
        .is_some_and(|url| is_safe_remote_url(&url))
}

fn is_safe_remote_url(url: &reqwest::Url) -> bool {
    let expected_port = match url.scheme() {
        "http" => 80,
        "https" => 443,
        _ => return false,
    };
    if url.port().is_some_and(|port| port != expected_port)
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    let normalized_host = host.trim_end_matches('.').to_ascii_lowercase();
    if normalized_host == "localhost" || normalized_host.ends_with(".localhost") {
        return false;
    }
    normalized_host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(&normalized_host)
        .parse::<IpAddr>()
        .map(is_public_ip)
        .unwrap_or(true)
}

pub(super) fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(a == 0
                || a == 10
                || a == 127
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192 && b == 0 && c == 0)
                || (a == 192 && b == 0 && c == 2)
                || (a == 192 && b == 88 && c == 99)
                || (a == 192 && b == 168)
                || (a == 198 && (b == 18 || b == 19))
                || (a == 198 && b == 51 && c == 100)
                || (a == 203 && b == 0 && c == 113)
                || a >= 224)
        }
        IpAddr::V6(ip) => {
            if let Some(mapped) = ip.to_ipv4_mapped() {
                return is_public_ip(IpAddr::V4(mapped));
            }
            let segments = ip.segments();
            (segments[0] & 0xe000) == 0x2000
                && !(segments[0] == 0x2001
                    && (segments[1] == 0
                        || segments[1] == 2
                        || (0x10..=0x2f).contains(&segments[1])
                        || segments[1] == 0x0db8))
                && segments[0] != 0x2002
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct PublicDnsResolver;

impl reqwest::dns::Resolve for PublicDnsResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_owned();
        Box::pin(async move {
            let addresses: Vec<_> = tokio::net::lookup_host((host.as_str(), 0))
                .await?
                .filter(|address| is_public_ip(address.ip()))
                .collect();
            if addresses.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "remote image host has no public address",
                )
                .into());
            }
            Ok(Box::new(addresses.into_iter()) as reqwest::dns::Addrs)
        })
    }
}

pub(super) fn append_bounded(
    destination: &mut Vec<u8>,
    chunk: &[u8],
    limit: usize,
) -> Result<(), String> {
    if destination
        .len()
        .checked_add(chunk.len())
        .is_none_or(|length| length > limit)
    {
        return Err(format!("remote image exceeds {limit}-byte limit"));
    }
    destination.extend_from_slice(chunk);
    Ok(())
}
