use super::{FilingSection, FilingSummary};

// ── Heuristic filing summarizer ─────────────────────────────────────
//
// Pure-text, deterministic, no LLM. Parses plain-text produced by
// `strip_html_to_text` and extracts type-specific structured highlights.

fn canonical_form(form_type: &str) -> String {
    let up = form_type.trim().to_uppercase();
    // Strip amendment suffix (e.g., "10-K/A" → "10-K").
    let base = up.split('/').next().unwrap_or(&up);
    base.to_string()
}

/// Find first `n` non-empty paragraphs from `text` starting at `start_offset`.
fn first_paragraphs(text: &str, start_offset: usize, n: usize, max_len: usize) -> Vec<String> {
    let slice = &text[start_offset.min(text.len())..];
    slice
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| p.len() > 40) // skip stubs / section headers
        .take(n)
        .map(|p| {
            if p.len() > max_len {
                let mut cut = max_len;
                while cut > 0 && !p.is_char_boundary(cut) {
                    cut -= 1;
                }
                format!("{}…", &p[..cut])
            } else {
                p.to_string()
            }
        })
        .collect()
}

/// Locate a named section by case-insensitive header match. Returns (title, offset_after_header).
fn find_section(text: &str, needles: &[&str]) -> Option<(String, usize)> {
    let upper = text.to_uppercase();
    for needle in needles {
        let up_needle = needle.to_uppercase();
        if let Some(idx) = upper.find(&up_needle) {
            let end = idx + up_needle.len();
            return Some((needle.to_string(), end));
        }
    }
    None
}

/// Extract "Item X.YY" headers from an 8-K document. Returns Vec<(item_code, first_paragraph)>.
fn extract_8k_items(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    // Iterate lines looking for "Item N.NN" at the start.
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        // Match "Item 1.01", "Item 2.02", "Item 5.07", etc.
        let lower = line.to_lowercase();
        if lower.starts_with("item ") && line.len() > 5 {
            let rest = &line[5..];
            // Take leading digits + dot + digits
            let code: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if code.contains('.') && code.len() >= 3 {
                // Next ~8 lines of body
                let mut body = String::new();
                let mut j = i + 1;
                let mut collected = 0;
                while j < lines.len() && collected < 8 {
                    let l = lines[j].trim();
                    // Stop at next item
                    if l.to_lowercase().starts_with("item ")
                        && l.chars()
                            .nth(5)
                            .map(|c| c.is_ascii_digit())
                            .unwrap_or(false)
                    {
                        break;
                    }
                    if !l.is_empty() {
                        if !body.is_empty() {
                            body.push(' ');
                        }
                        body.push_str(l);
                        collected += 1;
                    }
                    j += 1;
                }
                let title = format!("Item {}", code);
                // Rest of the header line after the code (often the item description)
                let after_code = &rest[code.len()..];
                let header_tail = after_code
                    .trim_start_matches(|c: char| c == '.' || c.is_whitespace())
                    .trim();
                let display_title = if !header_tail.is_empty() {
                    format!("{} — {}", title, header_tail)
                } else {
                    title
                };
                // Trim body to ~500 chars
                if body.len() > 500 {
                    let mut cut = 500;
                    while cut > 0 && !body.is_char_boundary(cut) {
                        cut -= 1;
                    }
                    body = format!("{}…", &body[..cut]);
                }
                out.push((display_title, body));
                i = j;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Summarize a 10-K / 10-Q by pulling Risk Factors, MD&A, Business.
fn summarize_10kq(text: &str) -> FilingSummary {
    let mut summary = FilingSummary::default();
    let candidates: &[(&str, &[&str])] = &[
        (
            "Business Overview",
            &["Item 1.", "ITEM 1.", "BUSINESS OVERVIEW"],
        ),
        ("Risk Factors", &["Item 1A.", "ITEM 1A.", "RISK FACTORS"]),
        (
            "Management's Discussion",
            &["Item 7.", "ITEM 7.", "MANAGEMENT'S DISCUSSION"],
        ),
        (
            "Quantitative & Qualitative Disclosures",
            &["Item 7A.", "ITEM 7A."],
        ),
        (
            "Legal Proceedings",
            &["Item 3.", "ITEM 3.", "LEGAL PROCEEDINGS"],
        ),
    ];
    for (label, needles) in candidates {
        if let Some((_, off)) = find_section(text, needles) {
            let paras = first_paragraphs(text, off, 2, 600);
            if !paras.is_empty() {
                summary.sections.push(FilingSection {
                    title: label.to_string(),
                    body: paras.join("\n\n"),
                });
            }
        }
    }
    // Bullet-ize the first paragraph of each found section.
    for s in &summary.sections {
        if let Some(first) = s.body.split("\n\n").next() {
            let short = if first.len() > 200 {
                let mut cut = 200;
                while cut > 0 && !first.is_char_boundary(cut) {
                    cut -= 1;
                }
                format!("{}…", &first[..cut])
            } else {
                first.to_string()
            };
            summary.bullets.push(format!("{}: {}", s.title, short));
        }
    }
    summary
}

/// Summarize a DEF 14A (proxy statement).
fn summarize_def14a(text: &str) -> FilingSummary {
    let mut summary = FilingSummary::default();
    let candidates: &[(&str, &[&str])] = &[
        (
            "Proposals",
            &["PROPOSAL 1", "PROPOSAL NO. 1", "PROPOSALS TO BE VOTED"],
        ),
        (
            "Executive Compensation",
            &["EXECUTIVE COMPENSATION", "COMPENSATION DISCUSSION"],
        ),
        (
            "Director Nominees",
            &[
                "DIRECTOR NOMINEES",
                "NOMINEES FOR DIRECTOR",
                "ELECTION OF DIRECTORS",
            ],
        ),
        (
            "Auditor Ratification",
            &["RATIFICATION", "INDEPENDENT REGISTERED PUBLIC ACCOUNTING"],
        ),
    ];
    for (label, needles) in candidates {
        if let Some((_, off)) = find_section(text, needles) {
            let paras = first_paragraphs(text, off, 1, 500);
            if !paras.is_empty() {
                summary.sections.push(FilingSection {
                    title: label.to_string(),
                    body: paras.join("\n\n"),
                });
                summary.bullets.push(format!("{}: found", label));
            }
        }
    }
    summary
}

/// Summarize an S-1 (IPO / registration statement).
fn summarize_s1(text: &str) -> FilingSummary {
    let mut summary = FilingSummary::default();
    let candidates: &[(&str, &[&str])] = &[
        ("Use of Proceeds", &["USE OF PROCEEDS"]),
        ("Risk Factors", &["RISK FACTORS"]),
        ("Prospectus Summary", &["PROSPECTUS SUMMARY", "SUMMARY"]),
        ("Business", &["BUSINESS OVERVIEW", "OUR BUSINESS"]),
        ("Dilution", &["DILUTION"]),
    ];
    for (label, needles) in candidates {
        if let Some((_, off)) = find_section(text, needles) {
            let paras = first_paragraphs(text, off, 1, 600);
            if !paras.is_empty() {
                summary.sections.push(FilingSection {
                    title: label.to_string(),
                    body: paras.join("\n\n"),
                });
                summary.bullets.push(format!("{}: extracted", label));
            }
        }
    }
    summary
}

/// Summarize a 13F holdings report — just count table rows heuristically.
fn summarize_13f(text: &str) -> FilingSummary {
    let mut summary = FilingSummary::default();
    // 13F info tables have many lines with dollar amounts. Count lines with " | " (from <td>).
    let row_count = text
        .lines()
        .filter(|l| l.matches(" | ").count() >= 3)
        .count();
    summary.headline = format!("13F — ~{} holdings (approx. from table rows)", row_count);
    summary.bullets.push(summary.headline.clone());
    if row_count == 0 {
        summary.bullets.push(
            "No holdings table detected in stripped text — data may be in XML attachment."
                .to_string(),
        );
    }
    summary
}

/// Summarize a Form 4 (insider transaction report) from raw text.
/// Note: structured InsiderTrade data is usually available separately; this is a fallback.
fn summarize_form4(text: &str) -> FilingSummary {
    let mut summary = FilingSummary::default();
    // The transaction rows encode acquired/disposed as the "(A)" / "(D)" code,
    // not the words "acquisition" / "disposition" — so the old word-count read
    // "0 acquisition / 0 disposition" on forms that plainly had trades. Surface
    // the transaction-bearing rows (those carrying a dollar amount, which is the
    // price-per-share column) directly instead; the fully parsed, structured
    // trades live in the Insiders tab. `is_transaction_row` keeps the headline
    // honest rather than emitting a misleading count.
    let is_transaction_row = |line: &str| {
        let t = line.trim();
        t.contains('$') && t.len() > 8 && t.len() < 400
    };
    let row_count = text.lines().filter(|l| is_transaction_row(l)).count();
    summary.headline = if row_count == 0 {
        "Form 4 — insider transaction report".to_string()
    } else {
        format!("Form 4 — insider transaction report ({row_count} transaction row(s))")
    };
    summary.bullets.push(summary.headline.clone());
    for row in text.lines().filter(|l| is_transaction_row(l)).take(6) {
        summary.bullets.push(row.trim().to_string());
    }
    summary
}

/// Dispatch entry point. Pass the plain-text content (from `strip_html_to_text` or
/// `get_filing_content`) and the form type. Returns an empty `FilingSummary` if
/// nothing could be extracted — caller should fall back to raw-text display.
pub fn summarize_filing(form_type: &str, content: &str) -> FilingSummary {
    let form = canonical_form(form_type);
    let mut summary = match form.as_str() {
        "8-K" => {
            let items = extract_8k_items(content);
            let mut s = FilingSummary::default();
            if let Some((first_title, _)) = items.first() {
                s.headline = format!("8-K — {}", first_title);
            } else {
                s.headline = "8-K — (no Item headers detected)".to_string();
            }
            for (title, body) in items.iter().take(8) {
                s.bullets.push(title.clone());
                s.sections.push(FilingSection {
                    title: title.clone(),
                    body: body.clone(),
                });
            }
            s
        }
        "10-K" | "10-Q" => {
            let mut s = summarize_10kq(content);
            s.headline = format!("{} — {} section(s) extracted", form, s.sections.len());
            s
        }
        "DEF 14A" | "PRE 14A" => {
            let mut s = summarize_def14a(content);
            s.headline = format!("Proxy ({}) — {} topic(s)", form, s.sections.len());
            s
        }
        "S-1" | "S-1/A" | "424B1" | "424B2" | "424B3" | "424B4" | "424B5" => {
            let mut s = summarize_s1(content);
            s.headline = format!("{} — {} section(s)", form, s.sections.len());
            s
        }
        "13F-HR" | "13F-HR/A" | "13F-NT" => summarize_13f(content),
        "4" | "4/A" => summarize_form4(content),
        _ => {
            // Generic: pull first substantial paragraphs.
            let mut s = FilingSummary::default();
            let paras = first_paragraphs(content, 0, 3, 500);
            s.headline = format!("{} — generic extract", form);
            for p in paras {
                s.bullets.push(p);
            }
            s
        }
    };
    // Guarantee at least a headline on empty outputs so the UI has something to show.
    if summary.headline.is_empty() {
        summary.headline = format!("{} filing", form);
    }
    summary
}
