# ADR-100: News Article Rendering: DOM-aware extractor, hero images, CommonMark viewer; NO HTML/JS renderer

**Status:** Accepted (2026-05-26)
**Supersedes / Builds on:** ADR-078 (news ingest pipeline), ADR-080 (web ingest + packet viewer), ADR-096 (AI return-path auto-ingest)

## Context

The News floating window rendered article bodies via `ui.label(egui::RichText::new(&a.body))` — flat plain-text, no images, no formatting. The body itself was produced by `news::extract_article_text`, a byte-level HTML stripper that dropped `<script>/<style>/<noscript>` and kept *every other* text node in document order.

On modern publisher pages (Yahoo Finance specifically, but the pattern is universal — Bloomberg, Reuters, MarketWatch, etc.) that meant the article render contained the full site chrome before any real content:

```
Today's news / US / Politics / World / Weather / Climate change /
Health / Science / Originals / Newsletters / Games / Life / …
[20+ more nav items]
WORK Medical Technology Group Ltd. (NASDAQ: WOK) shares are trending
on Wednesday. [actual article begins here]
```

The user reported this directly with two screenshots: top-of-pane filled with Yahoo navigation, the article body buried below.

Three orthogonal questions surfaced from that thread:

1. **Are we syncing images with news articles?** Yes — `NewsArticle.image_url` is a real column populated by 5 of 6 providers (GDELT `socialimage`, Marketaux `image_url`, AlphaVantage `banner_image`, FMP `image`, Finnhub `image`). Yahoo RSS leaves it empty. The data is stored but never rendered.
2. **Should we render articles "as nicely as possible"?** Yes, but the bottleneck is content selection (the extractor), not the renderer.
3. **Should we add a full HTML renderer?** No — see threat-model section below.

## Decision

Three coupled changes shipped together:

### 1. DOM-aware article extractor (`scraper` crate)

`engine/src/core/news.rs::extract_article_text` is rewritten on top of the `scraper` crate (built on `html5ever`). Two passes:

- **First pass — known containers in priority order**:
  ```
  div.caas-body          (Yahoo Finance)
  div.article-body
  div.article-content
  div.story-body         (BBC / Guardian variants)
  div.post-content       (WordPress)
  div.entry-content      (WordPress)
  div#article-body
  div#article-content
  div#main-content
  article                (semantic)
  [role="main"]          (ARIA)
  main                   (semantic)
  ```
  First selector that yields ≥200 chars of text wins. Ordered most-specific → least-specific so a site with both `<article>` and `.caas-body` (Yahoo nests them) picks the tighter wrapper.

- **Second pass — drop boilerplate from whatever subtree was selected**:
  ```
  Semantic: nav / header / footer / aside / form / button / script / style / noscript
  ARIA: [role=navigation|banner|contentinfo|complementary]
  Class hooks: .nav .navbar .menu .sidebar .footer .header .masthead
               .breadcrumb .related .related-articles .recommended
               .comments .advertisement .ad-container .ad-slot
               .social .social-share .share .newsletter .subscribe
               .paywall .cookie-banner .promo .promotion
  Yahoo-specific: .caas-tools .caas-related .caas-readmore .caas-share
                  .caas-da .caas-disclaimer
  ```

- **Fallback** when no semantic container matches: walk `<body>` with the same drop list. Strictly better than the previous "grab everything" behaviour.

### 2. Hero image extraction + rendering

`extract_article_with_image()` returns `(body, image_url)`. The image is sourced from (in order):

- `<meta property="og:image">`
- `<meta name="twitter:image">`
- `<meta name="twitter:image:src">`
- `<meta property="og:image:secure_url">`

Only absolute `http://` / `https://` URIs are returned (defensive against `javascript:` and relative paths).

The body hydrator (`native/src/app/news_ingest.rs`) calls the new path and writes via `upsert_news_body_and_image`, which uses a conditional SQL update so a backfilled image never clobbers an image the source RSS already supplied:

```sql
image_url = CASE WHEN image_url = '' AND ?3 <> '' THEN ?3 ELSE image_url END
```

The News window renders the hero image at the top of the article body via `egui::Image::new(url).max_width(560).corner_radius(4.0)`. URL→texture decode happens via `egui_extras` image loaders (`PNG`/`JPEG`/`WEBP`/`HTTP` dispatch), installed once at `TyphooNApp::new` via `egui_extras::install_image_loaders(&cc.egui_ctx)`.

### 3. CommonMark viewer for the article body

The body and summary panes render through `egui_commonmark::CommonMarkViewer` with a persistent `CommonMarkCache` field on `TyphooNApp`. Plain-text bodies render the same as before (CommonMark interprets them as paragraphs). The upgrade unlocks:

- Inline link rendering — clickable URLs when the extractor or AI return path preserves them.
- Inline images via `![alt](url)` — useful when an AI agent's TYPHOON_INGEST `body` field includes markdown image syntax.
- Heading / list formatting for AI-supplied bodies, which often arrive structured.

## Rejected: full HTML renderer (webview / wry / tao / similar)

Rejected explicitly on **security**, **maintenance**, and **value** grounds:

### Security (primary rejection reason)

A full HTML rendering pipeline ships:

1. **A browser engine** (CEF / WebKit / Servo) inside the terminal process.
2. **JavaScript execution** by default — the engine evaluates whatever script the page ships.
3. **Network fetching** of every subresource the page references — CDNs, third-party trackers, ad tags, analytics beacons.
4. **WebSocket connections** initiated by page JS.
5. **Plugin / fingerprinting surface** (Canvas / WebGL / fonts / etc.).

For a financial terminal that ingests **untrusted publisher content from URLs surfaced by external news APIs**, every one of those is a fresh attack surface:

- A compromised publisher article can run code in the terminal process.
- Tracking scripts get to fingerprint the user's terminal.
- The TyphooN-Terminal IP becomes attributable to specific articles read.
- A drive-by exploit in the embedded browser engine touches a process that has access to the user's positions, trade API keys, and cache DB.

The terminal already has Kraken / tastytrade / MT5 credentials in memory. Adding a renderer that evaluates arbitrary remote JS is not a trade we make.

The DOM extractor in this ADR runs `scraper` / `html5ever` over **already-fetched bytes**, with no JS execution, no subresource fetches, and no network egress beyond the single body fetch the hydrator initiates. It cannot be tricked into making additional requests by anything in the page.

### Maintenance

A bundled webview adds a ~50–150 MB dependency, a separate render thread, OS-level windowing integration that conflicts with egui's single-window paradigm, and a continuous CVE stream from the embedded engine. Every browser-engine update is a forced terminal release.

### Value

A real test of "what does HTML rendering buy us?" reveals: paragraph breaks, hyperlinks, inline images, headings, basic emphasis. All five are already covered by CommonMark + image loaders + DOM-cleaned text. Almost everything else in a typical news article render is hostile to the reader — auto-play video, modal "subscribe" dialogs, sticky ad rails — and a webview would render them faithfully. The extractor + CommonMark path is intentionally minimalist on the user's behalf.

## Consequences

- **Article views stop showing site navigation, footers, related-article rails, and ads.** The extractor delivers article text only.
- **Hero images appear at the top of every article view** when the source supplied one or the body fetcher could extract og:image. Yahoo articles, previously image-less, now have hero images for the first time.
- **CommonMark bodies render with paragraph breaks and clickable links** instead of one wall of text.
- **No remote JS executes** as part of news rendering. Image subresources are the only outbound requests, and they go through egui's loader on user-visible articles only.
- **Six new unit tests** in `engine/src/core/news.rs` cover the extractor:
  - `extract_article_prefers_caas_body_over_page_chrome` — primary Yahoo case
  - `extract_article_picks_semantic_article_tag`
  - `extract_article_falls_back_to_body_when_no_container`
  - `extract_article_drops_script_and_style_blocks`
  - `extract_image_uses_twitter_card_fallback`
  - `extract_image_ignores_non_absolute_urls`
- **New deps**: `scraper = "0.27"` (engine, ~500 KB binary impact), `egui_extras = "0.34"` with `image` + `all_loaders`, `egui_commonmark = "0.23"` with `load-images`. All four build cleanly against eframe 0.34.
- **Schema**: no migrations. `image_url` column already existed (ADR-078) — the extractor just starts populating it for sources that previously left it blank.

## Threat model carve-outs (for future reviewers)

If a future change adds *any* of the following to news rendering, this ADR should be re-opened:

- A WebView or any JS-evaluation primitive
- Auto-fetch of inline `<iframe>` content
- Background prefetch of next-article URLs (currently we only fetch on user action / hydrator opt-in)
- Cross-origin resource loading from rendered HTML (e.g. inlining a publisher's full CSS would let them load tracking pixels)

The egui image loader does fetch subresources, but only for `image_url` and inline `![](url)` markdown images — both originate from the same publisher domain as the article in practice, and the image loader doesn't execute code.
