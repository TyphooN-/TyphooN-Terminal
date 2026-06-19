// ── HTML Stripping (reusable for content storage + on-demand viewer) ────────

/// Convert raw HTML to searchable plain text.
pub fn strip_html_to_text(html: &str) -> String {
    let mut text = html.to_string();
    // Remove style/script/head/noscript blocks
    for tag in &["style", "script", "head", "noscript"] {
        while let Some(start) = text.find(&format!("<{}", tag)) {
            if let Some(end) = text[start..].find(&format!("</{}>", tag)) {
                text.replace_range(start..start + end + tag.len() + 3, "\n");
            } else {
                break;
            }
        }
    }
    // Convert structural HTML to whitespace
    text = text
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n");
    text = text.replace("</p>", "\n\n").replace("</div>", "\n");
    text = text
        .replace("</tr>", "\n")
        .replace("</td>", " | ")
        .replace("</th>", " | ");
    text = text.replace("</li>", "\n").replace("<li>", "  - ");
    // Strip remaining tags
    let mut without_tags = String::with_capacity(text.len());
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => without_tags.push(ch),
            _ => {}
        }
    }
    // Entity decoding + line polish happens in a shared pass so cached
    // legacy content (which went through an older, weaker decoder) can be
    // cleaned up at read time too.
    polish_filing_text(&without_tags)
}

/// Decode HTML entities and drop visual-noise lines from an already-stripped
/// filing body. Safe to apply to both fresh strip_html_to_text output and
/// cached `content_plain` blobs — the latter may still contain raw
/// `&#160;` / `&#9744;` entities written by older builds that only handled
/// the named-entity set.
pub fn polish_filing_text(text: &str) -> String {
    let decoded = decode_html_entities(text);
    // Drop lines that are visually empty after entity decoding: tables
    // serialised as `| | | |`, NBSP-only rows, or pure punctuation
    // dividers that contribute nothing to the reader.
    let mut out: Vec<&str> = Vec::with_capacity(decoded.lines().count());
    for line in decoded.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if line_is_visually_empty(trimmed) {
            continue;
        }
        out.push(trimmed);
    }
    out.join("\n")
}

/// Decode named and numeric HTML entities. Handles the legacy named set
/// (`&amp;`, `&lt;`, `&gt;`, `&nbsp;`, `&quot;`, `&apos;`, `&#39;`) plus
/// any numeric entity in decimal (`&#NNN;`) or hex (`&#xHH;` / `&#XHH;`)
/// form. `&` outside an entity context is left alone. Returns the decoded
/// string with the original byte width preserved when no entities are
/// present.
pub(super) fn decode_html_entities(input: &str) -> String {
    if !input.contains('&') {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'&' {
            // Multi-byte UTF-8 char — push the full char in one step.
            let ch = input[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
            continue;
        }
        // Look for the matching ';' within a small window. Real entities
        // are at most ~8 chars including the leading '&' and trailing ';'.
        let scan_end = (i + 12).min(bytes.len());
        let semi = bytes[i..scan_end].iter().position(|&b| b == b';');
        let Some(semi_off) = semi else {
            out.push('&');
            i += 1;
            continue;
        };
        let entity = &input[i + 1..i + semi_off]; // between '&' and ';'
        if let Some(decoded) = decode_entity_body(entity) {
            out.push_str(&decoded);
            i += semi_off + 1;
        } else {
            out.push('&');
            i += 1;
        }
    }
    out
}

fn decode_entity_body(body: &str) -> Option<String> {
    if body.is_empty() {
        return None;
    }
    if let Some(rest) = body.strip_prefix('#') {
        let code = if let Some(hex) = rest.strip_prefix(|c: char| c == 'x' || c == 'X') {
            u32::from_str_radix(hex, 16).ok()?
        } else {
            rest.parse::<u32>().ok()?
        };
        let ch = char::from_u32(code)?;
        // NBSP normalises to a regular space so downstream "line is empty"
        // checks behave intuitively.
        return Some(if ch == '\u{a0}' {
            " ".to_string()
        } else {
            ch.to_string()
        });
    }
    Some(
        match body {
            "amp" => "&",
            "lt" => "<",
            "gt" => ">",
            "quot" => "\"",
            "apos" => "'",
            "nbsp" => " ",
            _ => return None,
        }
        .to_string(),
    )
}

/// `true` if the line contributes nothing visual: only whitespace, pipes,
/// stray punctuation, or runs of NBSP-equivalent characters that survived
/// entity decoding via direct insertion.
fn line_is_visually_empty(line: &str) -> bool {
    line.chars()
        .all(|c| c.is_whitespace() || c == '|' || c == '\u{a0}')
}
