//! Article-body hydration and readability extraction for cached news.

/// blow up the cache. Returns `None` for non-2xx responses, empty bodies,
/// or extracted text under 200 chars (likely a paywall splash, not the
/// article). The output is suitable for direct storage via
/// [`upsert_news_body`] and for indexing in `research_news_fts`.
pub async fn fetch_article_body(url: &str) -> Option<String> {
    fetch_article_body_with_image(url)
        .await
        .map(|(body, _)| body)
}

/// Same as `fetch_article_body` but also returns the hero image URL when
/// the page exposes one via og:image / twitter:image. Used by the body
/// hydrator to backfill `NewsArticle.image_url` for sources whose RSS
/// feed (Yahoo RSS is the big offender) doesn't carry image metadata.
/// Returns `None` when the fetch fails or the extracted body is below
/// the minimum chars threshold (probably a paywall / redirect splash).
pub async fn fetch_article_body_with_image(url: &str) -> Option<(String, String)> {
    const MAX_FETCH_BYTES: usize = 2 * 1024 * 1024; // 2 MiB cap on raw HTML
    const MIN_BODY_CHARS: usize = 200; // anything shorter is probably a redirect/paywall splash
    const FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);
    let url = url.trim();
    if url.is_empty() || !(url.starts_with("http://") || url.starts_with("https://")) {
        return None;
    }
    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (compatible; TyphooN-Terminal/0.1; +https://riskprivacy.com/typhoon)",
        )
        .timeout(FETCH_TIMEOUT)
        .build()
        .ok()?;
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    // Soft size cap: stream-read until we hit the limit, then bail.
    let bytes = resp.bytes().await.ok()?;
    let html = if bytes.len() > MAX_FETCH_BYTES {
        String::from_utf8_lossy(&bytes[..MAX_FETCH_BYTES]).into_owned()
    } else {
        String::from_utf8_lossy(&bytes).into_owned()
    };
    let (text, image_url) = extract_article_with_image(&html);
    if text.chars().count() < MIN_BODY_CHARS {
        return None;
    }
    Some((text, image_url))
}

/// DOM-aware HTML → text extractor. Builds an html5ever DOM via `scraper`
/// and walks it picking the publisher's main article container while
/// dropping site chrome (nav, header, footer, aside, related-articles,
/// menus, ads). The byte-level scanner this replaced kept every text node
/// in document order, which on Yahoo Finance meant the entire left nav
/// menu rendered before the actual article body.
///
/// Strategy:
///   1. Try a priority list of semantic + class/id selectors known to
///      wrap the article body (`<article>`, `<main>`, `.caas-body`,
///      `.article-body`, etc.). First one whose subtree has enough text
///      wins.
///   2. If none matched, fall back to `<body>` minus the same chrome.
///   3. While walking either path, recursively skip subtrees that match
///      a drop selector (nav / header / footer / .related / etc.).
///   4. `finalize_extracted_text` then collapses whitespace and folds
///      long whitespace runs into paragraph breaks.
///
/// Returns the cleaned text. For callers that also want the hero image
/// (og:image / twitter:image), use `extract_article_with_image` instead.
pub fn extract_article_text(html: &str) -> String {
    let (body, _image) = extract_article_with_image(html);
    body
}

/// Same DOM walk as `extract_article_text` but additionally returns the
/// hero image URL extracted from `<meta property="og:image">` (or the
/// twitter:image variants) — empty string if the document has none.
/// Used by the body hydrator to backfill the `image_url` field on
/// articles whose source RSS didn't supply one (Yahoo, primarily).
pub fn extract_article_with_image(html: &str) -> (String, String) {
    let doc = scraper::Html::parse_document(html);
    let image_url = extract_hero_image_url(&doc);

    // Priority list of selectors for the article body container. The
    // first one that matches and yields ≥200 chars of text wins. Ordered
    // most-specific → least-specific so a site with both `<article>` and
    // `.caas-body` (Yahoo nests them) picks the tighter wrapper.
    const ARTICLE_SELECTORS: &[&str] = &[
        "div.caas-body",
        "div.article-body",
        "div.article-content",
        "div.story-body",
        "div.post-content",
        "div.entry-content",
        "div#article-body",
        "div#article-content",
        "div#main-content",
        "article",
        "[role=\"main\"]",
        "main",
    ];

    let drop_selectors = parse_drop_selectors();

    for sel_str in ARTICLE_SELECTORS {
        let Ok(sel) = scraper::Selector::parse(sel_str) else {
            continue;
        };
        for node in doc.select(&sel) {
            let mut buf = String::with_capacity(2048);
            walk_visible_text(node, &drop_selectors, &mut buf);
            if buf.chars().count() >= 200 {
                let body = finalize_extracted_text(buf.into_bytes());
                return (body, image_url);
            }
        }
    }

    // Fallback: walk the document body and rely on the drop list. Better
    // than the old whole-page strip because chrome elements are now
    // pruned by selector rather than greedily included as plain text.
    let mut buf = String::with_capacity(2048);
    if let Ok(body_sel) = scraper::Selector::parse("body") {
        if let Some(body) = doc.select(&body_sel).next() {
            walk_visible_text(body, &drop_selectors, &mut buf);
        }
    }
    let body = finalize_extracted_text(buf.into_bytes());
    (body, image_url)
}

/// Selectors for subtrees we never want in the article text: site
/// navigation, headers/footers, related-article rails, ads, social
/// widgets, comment blocks, login/paywall prompts. Parsed once per
/// extract call. Anything that doesn't parse as a valid CSS selector
/// is silently skipped.
fn parse_drop_selectors() -> Vec<scraper::Selector> {
    const DROP_PATTERNS: &[&str] = &[
        // Semantic / role tags
        "nav",
        "header",
        "footer",
        "aside",
        "form",
        "button",
        "script",
        "style",
        "noscript",
        "[role=\"navigation\"]",
        "[role=\"banner\"]",
        "[role=\"contentinfo\"]",
        "[role=\"complementary\"]",
        // Common boilerplate class hooks across publishers
        ".nav",
        ".navbar",
        ".menu",
        ".sidebar",
        ".footer",
        ".header",
        ".masthead",
        ".breadcrumb",
        ".breadcrumbs",
        ".related",
        ".related-articles",
        ".related-content",
        ".recommended",
        ".comments",
        ".comment",
        ".advertisement",
        ".ad-container",
        ".ad-slot",
        ".social",
        ".social-share",
        ".share",
        ".newsletter",
        ".subscribe",
        ".paywall",
        ".cookie-banner",
        ".cookie-notice",
        ".consent",
        ".promo",
        ".promotion",
        // Yahoo Finance specific noise
        ".caas-tools",
        ".caas-related",
        ".caas-readmore",
        ".caas-share",
        ".caas-da",
        ".caas-disclaimer",
    ];
    DROP_PATTERNS
        .iter()
        .filter_map(|s| scraper::Selector::parse(s).ok())
        .collect()
}

/// Recursive walker that appends visible text from `node` into `out`,
/// skipping any subtree rooted at an element matching one of the drop
/// selectors. A trailing newline is inserted after block-level elements
/// so paragraphs don't get glued together.
fn walk_visible_text(node: scraper::ElementRef, drops: &[scraper::Selector], out: &mut String) {
    for d in drops {
        if d.matches(&node) {
            return;
        }
    }
    for child in node.children() {
        if let Some(text_node) = child.value().as_text() {
            out.push_str(text_node);
        } else if let Some(elem) = scraper::ElementRef::wrap(child) {
            walk_visible_text(elem, drops, out);
            let name = elem.value().name();
            if matches!(
                name,
                "p" | "br"
                    | "div"
                    | "li"
                    | "tr"
                    | "h1"
                    | "h2"
                    | "h3"
                    | "h4"
                    | "h5"
                    | "h6"
                    | "blockquote"
                    | "section"
                    | "article"
                    | "pre"
            ) {
                out.push('\n');
            }
        }
    }
}

/// Pull a hero image URL from common OpenGraph / Twitter Card meta tags.
/// Returns empty string when none is present or the URL isn't absolute
/// http(s). Used by the body hydrator to populate `NewsArticle.image_url`
/// for sources that don't supply one in their RSS / API payload.
fn extract_hero_image_url(doc: &scraper::Html) -> String {
    const META_SELECTORS: &[&str] = &[
        "meta[property=\"og:image\"]",
        "meta[name=\"twitter:image\"]",
        "meta[name=\"twitter:image:src\"]",
        "meta[property=\"og:image:secure_url\"]",
    ];
    for sel_str in META_SELECTORS {
        let Ok(sel) = scraper::Selector::parse(sel_str) else {
            continue;
        };
        if let Some(node) = doc.select(&sel).next() {
            if let Some(val) = node.value().attr("content") {
                let trimmed = val.trim();
                if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                    return trimmed.to_string();
                }
            }
        }
    }
    String::new()
}

fn finalize_extracted_text(raw_bytes: Vec<u8>) -> String {
    // Lossy decode here is final — anything that wasn't valid UTF-8 in the
    // source HTML gets a replacement char, which is fine for an indexable
    // article body.
    let raw = String::from_utf8_lossy(&raw_bytes).into_owned();
    let decoded = decode_html_entities(&raw);
    // Collapse whitespace runs to single spaces; convert paragraph breaks
    // (multiple spaces) into newlines so the stored text is readable.
    let mut out = String::with_capacity(decoded.len());
    let mut last_was_space = true;
    let mut consecutive_spaces = 0u32;
    for ch in decoded.chars() {
        if ch.is_whitespace() {
            consecutive_spaces += 1;
            if !last_was_space {
                out.push(if consecutive_spaces > 4 { '\n' } else { ' ' });
                last_was_space = true;
            } else if consecutive_spaces == 5 {
                // first promotion of a long run → make it a paragraph break
                if out.ends_with(' ') {
                    out.pop();
                }
                out.push('\n');
            }
        } else {
            consecutive_spaces = 0;
            out.push(ch);
            last_was_space = false;
        }
    }
    out.trim().to_string()
}

/// Heuristic readability pass for scraped article bodies shown in the News
/// panel. The DOM extractor's drop-list removes most chrome, but on some
/// publishers (notably Stocktwits / Yahoo syndication) inline cruft still
/// leaks into the plain text: repeated "Loading..." lazy-loader placeholders,
/// inline "Advertisement|Remove ads." markers, and a trailing reader-comments
/// blob that runs straight into the article copy with no break.
///
/// Pure string transform applied at render time, so it cleans the already
/// cached bodies without a re-scrape. It:
///   - turns inline ad markers into paragraph breaks (they sit between paras),
///   - drops the "Loading..." placeholders,
///   - delineates the reader-comments section with a header + per-comment lines,
///   - makes every extracted line its own CommonMark paragraph (a single '\n'
///     is a soft break in CommonMark and otherwise collapses the whole body
///     into one unreadable wall of text).
pub fn clean_article_body(body: &str) -> String {
    let mut text = body.to_string();

    // Inline ad markers → paragraph breaks (they separate real paragraphs).
    for junk in [
        "Advertisement|Remove ads.",
        "Advertisement | Remove ads.",
        "|Remove ads.",
        "Remove ads.",
        "Advertisement",
    ] {
        text = text.replace(junk, "\n\n");
    }

    // Lazy-loader placeholders → gone (often repeated dozens of times).
    text = text.replace("Loading...", " ").replace("Loading…", " ");

    // Delineate the trailing reader-comments blob. Markers are publisher-stable
    // Stocktwits phrasings; only the FIRST match splits, so legitimate uses of
    // "said" earlier in the article copy are left untouched.
    const COMMENT_MARKERS: &[&str] =
        &["One user said", "Another user said", "Comments posted here"];
    if let Some(idx) = COMMENT_MARKERS.iter().filter_map(|m| text.find(m)).min() {
        let (article, comments) = text.split_at(idx);
        let comments = comments
            .replace("One user said", "\n- One user said")
            .replace("Another user said", "\n- Another user said");
        text = format!(
            "{}\n\n---\n\n**Reader comments**\n\n{}",
            article.trim_end(),
            comments.trim_start()
        );
    }

    // Normalize whitespace and force CommonMark paragraph breaks: collapse each
    // line's internal whitespace, drop empties, and join with blank lines.
    text.split('\n')
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
        .trim()
        .to_string()
}

fn decode_html_entities(s: &str) -> String {
    // Walk by chars but track byte position so we can spot `&...;` entities
    // (which are pure ASCII) without breaking up multi-byte UTF-8 sequences
    // in the surrounding text.
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&' {
            if let Some(semi) = (i + 1..(i + 12).min(bytes.len())).find(|&j| bytes[j] == b';') {
                let entity = &s[i + 1..semi];
                let mapped = match entity {
                    "amp" => Some("&"),
                    "lt" => Some("<"),
                    "gt" => Some(">"),
                    "quot" => Some("\""),
                    "apos" => Some("'"),
                    "nbsp" => Some(" "),
                    "mdash" => Some("—"),
                    "ndash" => Some("–"),
                    "hellip" => Some("…"),
                    "lsquo" | "rsquo" => Some("'"),
                    "ldquo" | "rdquo" => Some("\""),
                    "copy" => Some("©"),
                    "reg" => Some("®"),
                    _ => None,
                };
                if let Some(m) = mapped {
                    out.push_str(m);
                    i = semi + 1;
                    continue;
                }
                // Numeric: &#NNN; or &#xHH;
                if let Some(num) = entity.strip_prefix('#') {
                    let parsed = if let Some(hex) =
                        num.strip_prefix('x').or_else(|| num.strip_prefix('X'))
                    {
                        u32::from_str_radix(hex, 16).ok()
                    } else {
                        num.parse::<u32>().ok()
                    };
                    if let Some(code) = parsed.and_then(char::from_u32) {
                        out.push(code);
                        i = semi + 1;
                        continue;
                    }
                }
            }
        }
        // Determine the byte length of the char starting at `i` so we copy
        // the whole UTF-8 sequence in one shot.
        let len = utf8_char_len(bytes[i]);
        if i + len <= bytes.len() {
            if let Ok(seg) = std::str::from_utf8(&bytes[i..i + len]) {
                out.push_str(seg);
                i += len;
                continue;
            }
        }
        // Fallback for an invalid sequence: skip one byte and continue.
        i += 1;
    }
    out
}

fn utf8_char_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b < 0xC0 {
        1
    }
    // continuation; treat as 1 to make progress on malformed input
    else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    }
}
