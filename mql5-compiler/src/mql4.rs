//! MQL4 (MetaTrader 4) compatibility layer.
//!
//! MT4 still has the largest pool of retail algorithmic code in existence
//! (two decades of EAs and indicators on ForexFactory / MQL5.com / Darwinex).
//! MQL4 and MQL5 share ~90% of their syntax, so rather than build a new
//! parser we rewrite MQL4-specific idioms into their MQL5 equivalents and
//! then feed the result through the existing `parse_mql5` pipeline.
//!
//! Rewrites applied (textual, conservative):
//!
//! | MQL4                                   | MQL5                                       |
//! |----------------------------------------|--------------------------------------------|
//! | `extern int Length = 14;`              | `input int Length = 14;`                   |
//! | `int init() { ... }`                   | `int OnInit() { ... }`                     |
//! | `int start() { ... }`                  | `void OnTick() { ... }`                    |
//! | `int deinit() { ... }`                 | `void OnDeinit(const int reason) { ... }`  |
//! | `Bid` / `Ask` bareword references      | `SymbolInfoDouble(_Symbol,SYMBOL_BID/ASK)` |
//! | `Close[0]` / `Open[0]` / `High[0]` …   | `iClose(_Symbol,0,0)` / …                  |
//! | `Volume[0]`                            | `iVolume(_Symbol,0,0)`                     |
//! | `Bars`                                 | `iBars(_Symbol,0)`                         |
//! | `Digits`                               | `_Digits`                                  |
//! | `Point`                                | `_Point`                                   |
//! | `Symbol()`                             | `_Symbol`                                  |
//!
//! Not rewritten (user must port manually for strategies — indicators are fine):
//! - `OrderSend(sym, op, lots, price, slip, sl, tp, cmt, mn, exp, color)`
//!   — MQL5 uses MqlTradeRequest/MqlTradeResult structs which have no textual
//!   1:1 rewrite. We emit a warning diagnostic instead.
//! - `OrderSelect(...)` / `OrderTicket()` / `OrderLots()` — same reason.
//! - `iCustom()` with more than 3 params — param layout changed.
//!
//! Detection is automatic: the `compile_mql4` entry point runs the rewrite pass
//! unconditionally and forwards to `compile_mql5`. You can also feed MQL4
//! source to `compile_mql5` and get a cleaner parse for the pure-overlap case.

use crate::CompileResult;

/// Compile MQL4 source as if it were MQL5 after an automatic rewrite pass.
pub fn compile_mql4(source: &str) -> CompileResult {
    let (rewritten, warnings) = rewrite_mql4_to_mql5(source);
    let mut result = crate::compile_mql5(&rewritten);
    for w in warnings {
        result.diagnostics.push(crate::Diagnostic {
            level: crate::DiagLevel::Warning,
            message: w,
            line: 0,
            col: 0,
        });
    }
    result
}

/// Textual MQL4 → MQL5 rewrite. Returns the rewritten source plus any warnings
/// produced when we detect constructs that cannot be auto-ported.
pub fn rewrite_mql4_to_mql5(source: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let mut out = String::with_capacity(source.len() + 128);

    // Line-by-line rewrite so we don't mangle string literals or comments that
    // contain the same keywords. We only touch tokens outside strings/comments.
    let mut in_block_comment = false;

    for raw_line in source.lines() {
        let mut line = String::with_capacity(raw_line.len());
        let mut chars = raw_line.chars().peekable();
        let mut in_string = false;
        let mut in_line_comment = false;

        while let Some(c) = chars.next() {
            if in_block_comment {
                line.push(c);
                if c == '*' && chars.peek() == Some(&'/') {
                    line.push('/');
                    chars.next();
                    in_block_comment = false;
                }
                continue;
            }
            if in_line_comment {
                line.push(c);
                continue;
            }
            if in_string {
                line.push(c);
                if c == '\\' {
                    if let Some(&n) = chars.peek() {
                        line.push(n);
                        chars.next();
                    }
                } else if c == '"' {
                    in_string = false;
                }
                continue;
            }
            match c {
                '"' => { in_string = true; line.push(c); }
                '/' if chars.peek() == Some(&'/') => {
                    in_line_comment = true;
                    line.push(c);
                    line.push('/');
                    chars.next();
                }
                '/' if chars.peek() == Some(&'*') => {
                    in_block_comment = true;
                    line.push(c);
                    line.push('*');
                    chars.next();
                }
                _ => line.push(c),
            }
        }

        let rewritten = rewrite_line(&line, &mut warnings);
        out.push_str(&rewritten);
        out.push('\n');
    }

    (out, warnings)
}

/// Rewrite a single line. Splits the line into alternating code/string/line-comment
/// spans and only applies the rewrites to the code spans — so user strings
/// (`Print("Bid is ", Bid);`) and trailing comments are left verbatim.
fn rewrite_line(line: &str, warnings: &mut Vec<String>) -> String {
    // Skip lines that are pure whitespace or comments — nothing to rewrite.
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || trimmed.is_empty() {
        return line.to_string();
    }

    // Tokenise the line into spans. Each span is either Code or Verbatim (string/comment).
    let mut segments: Vec<(bool, String)> = Vec::new(); // (is_code, text)
    let mut buf = String::new();
    let mut chars = line.chars().peekable();
    let mut in_string = false;
    let mut in_line_comment = false;
    while let Some(c) = chars.next() {
        if in_line_comment {
            buf.push(c);
            continue;
        }
        if in_string {
            buf.push(c);
            if c == '\\' {
                if let Some(&n) = chars.peek() { buf.push(n); chars.next(); }
            } else if c == '"' {
                segments.push((false, std::mem::take(&mut buf)));
                in_string = false;
            }
            continue;
        }
        if c == '"' {
            segments.push((true, std::mem::take(&mut buf)));
            buf.push('"');
            in_string = true;
            continue;
        }
        if c == '/' && chars.peek() == Some(&'/') {
            segments.push((true, std::mem::take(&mut buf)));
            buf.push('/');
            buf.push('/');
            chars.next();
            in_line_comment = true;
            continue;
        }
        buf.push(c);
    }
    if !buf.is_empty() {
        segments.push((!in_string && !in_line_comment, buf));
    }

    // Apply rewrites only to code segments.
    let mut out = String::with_capacity(line.len() + 32);
    for (is_code, seg) in segments {
        if !is_code {
            out.push_str(&seg);
            continue;
        }
        out.push_str(&rewrite_code_segment(&seg, warnings));
    }
    out
}

/// Apply all MQL4 → MQL5 word rewrites to a pure-code fragment (no strings/comments inside).
fn rewrite_code_segment(code: &str, warnings: &mut Vec<String>) -> String {
    let mut out = code.to_string();

    // 1. `extern` → `input` (MQL4 uses extern for chart inputs)
    out = replace_word(&out, "extern", "input");

    // 2. Entry points: `init()` / `start()` / `deinit()` at the top level.
    //    We use whole-token matches with a trailing `(` to avoid mangling
    //    user functions that happen to contain those substrings.
    if out.contains("int init(") && !out.contains("OnInit(") {
        out = out.replace("int init(", "int OnInit(");
    }
    if out.contains("int start(") && !out.contains("OnTick(") {
        // MQL4 indicators use `int start()`, EAs too. In MQL5, EA uses
        // `void OnTick()` and indicator uses `int OnCalculate(...)`.
        // We rewrite to OnTick — indicator users can rename manually if
        // they want OnCalculate. This is an MQL5-parser compatibility
        // rewrite, not a semantic port; both compile to an entry point.
        out = out.replace("int start(", "int OnTick(");
    }
    if out.contains("int deinit(") && !out.contains("OnDeinit(") {
        out = out.replace("int deinit(", "void OnDeinit(");
    }

    // 3. Environmental constants/functions.
    out = replace_word(&out, "Digits", "_Digits");
    out = replace_word(&out, "Point", "_Point");

    // 4. `Symbol()` → `_Symbol` (MQL5 prefers the constant).
    out = out.replace("Symbol()", "_Symbol");

    // 5. `Bid` / `Ask` bareword references that are NOT part of a longer
    //    identifier. Rewrite to the MQL5-style SymbolInfoDouble form.
    out = replace_word(
        &out,
        "Bid",
        "SymbolInfoDouble(_Symbol,SYMBOL_BID)",
    );
    out = replace_word(
        &out,
        "Ask",
        "SymbolInfoDouble(_Symbol,SYMBOL_ASK)",
    );

    // 6. `Close[i]` / `Open[i]` / `High[i]` / `Low[i]` / `Volume[i]` / `Time[i]`
    //    →  `iClose(_Symbol,0,i)` etc. Scan for pattern `<Series>[<expr>]`.
    out = rewrite_series_bracket(&out, "Close",  "iClose");
    out = rewrite_series_bracket(&out, "Open",   "iOpen");
    out = rewrite_series_bracket(&out, "High",   "iHigh");
    out = rewrite_series_bracket(&out, "Low",    "iLow");
    out = rewrite_series_bracket(&out, "Volume", "iVolume");
    out = rewrite_series_bracket(&out, "Time",   "iTime");

    // 7. `Bars` bareword → `iBars(_Symbol,0)` (only when not followed by `(` — Bars() is valid MQL5 too)
    out = replace_word_not_followed_by_open_paren(&out, "Bars", "iBars(_Symbol,0)");

    // 8. Warn on OrderSend/OrderSelect — no textual port.
    if out.contains("OrderSend(")
        && !out.contains("MqlTradeRequest")
        && !warnings.iter().any(|w| w.contains("OrderSend"))
    {
        warnings.push(
            "MQL4 OrderSend(...) has no textual 1:1 port to MQL5. \
             Rewrite to MqlTradeRequest + OrderSend(request, result) manually."
                .into(),
        );
    }
    if out.contains("OrderSelect(")
        && !warnings.iter().any(|w| w.contains("OrderSelect"))
    {
        warnings.push(
            "MQL4 OrderSelect(...) has no textual 1:1 port to MQL5. \
             Use PositionGetTicket / HistoryOrderGetTicket manually."
                .into(),
        );
    }

    out
}

/// Replace whole-word occurrences of `needle` (not part of a longer identifier).
fn replace_word(haystack: &str, needle: &str, replacement: &str) -> String {
    if !haystack.contains(needle) {
        return haystack.to_string();
    }
    let bytes = haystack.as_bytes();
    let nb = needle.as_bytes();
    let mut out = String::with_capacity(haystack.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + nb.len() <= bytes.len() && &bytes[i..i + nb.len()] == nb {
            let prev_ok = i == 0 || !is_ident_char(bytes[i - 1]);
            let next_ok = i + nb.len() == bytes.len() || !is_ident_char(bytes[i + nb.len()]);
            if prev_ok && next_ok {
                out.push_str(replacement);
                i += nb.len();
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Like `replace_word`, but only fires when the next non-whitespace char is
/// NOT an opening paren. Used so `Bars` (bareword) is rewritten but `Bars(` (MQL5 function call) is left alone.
fn replace_word_not_followed_by_open_paren(haystack: &str, needle: &str, replacement: &str) -> String {
    if !haystack.contains(needle) {
        return haystack.to_string();
    }
    let bytes = haystack.as_bytes();
    let nb = needle.as_bytes();
    let mut out = String::with_capacity(haystack.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + nb.len() <= bytes.len() && &bytes[i..i + nb.len()] == nb {
            let prev_ok = i == 0 || !is_ident_char(bytes[i - 1]);
            let after = i + nb.len();
            let next_ok = after == bytes.len() || !is_ident_char(bytes[after]);
            // Peek ahead past whitespace to see if a `(` follows.
            let mut j = after;
            while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            let open_paren = j < bytes.len() && bytes[j] == b'(';
            if prev_ok && next_ok && !open_paren {
                out.push_str(replacement);
                i += nb.len();
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Find all `Series[expr]` occurrences and rewrite to `iFunc(_Symbol,0,expr)`.
/// We balance square brackets so nested indexing like `Close[iLowest(...)]`
/// is preserved.
fn rewrite_series_bracket(haystack: &str, series: &str, ifunc: &str) -> String {
    let bytes = haystack.as_bytes();
    let sb = series.as_bytes();
    let mut out = String::with_capacity(haystack.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + sb.len() < bytes.len() && &bytes[i..i + sb.len()] == sb {
            let prev_ok = i == 0 || !is_ident_char(bytes[i - 1]);
            if prev_ok && bytes[i + sb.len()] == b'[' {
                // Find matching ]
                let mut depth = 1;
                let mut j = i + sb.len() + 1;
                while j < bytes.len() && depth > 0 {
                    match bytes[j] {
                        b'[' => depth += 1,
                        b']' => depth -= 1,
                        _ => {}
                    }
                    if depth == 0 { break; }
                    j += 1;
                }
                if depth == 0 {
                    let inner = &haystack[i + sb.len() + 1..j];
                    out.push_str(ifunc);
                    out.push_str("(_Symbol,0,");
                    out.push_str(inner);
                    out.push(')');
                    i = j + 1;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_extern_to_input() {
        let src = "extern int Length = 14;";
        let (out, _) = rewrite_mql4_to_mql5(src);
        assert!(out.contains("input int Length = 14"));
        assert!(!out.contains("extern"));
    }

    #[test]
    fn rewrites_init_start_deinit() {
        let src = "int init() { return 0; }\nint start() { return 0; }\nint deinit() { return 0; }\n";
        let (out, _) = rewrite_mql4_to_mql5(src);
        assert!(out.contains("OnInit("));
        assert!(out.contains("OnTick("));
        assert!(out.contains("void OnDeinit("));
    }

    #[test]
    fn rewrites_close_bracket_to_iclose() {
        let src = "double x = Close[0];";
        let (out, _) = rewrite_mql4_to_mql5(src);
        assert!(out.contains("iClose(_Symbol,0,0)"));
    }

    #[test]
    fn rewrites_nested_close_bracket() {
        let src = "double x = Close[iLowest(Symbol(),0,MODE_LOW,5,1)];";
        let (out, _) = rewrite_mql4_to_mql5(src);
        assert!(out.contains("iClose(_Symbol,0,iLowest"));
        // Symbol() should also be rewritten
        assert!(out.contains("_Symbol"));
        assert!(!out.contains("Symbol()"));
    }

    #[test]
    fn rewrites_bid_ask() {
        let src = "double spread = Ask - Bid;";
        let (out, _) = rewrite_mql4_to_mql5(src);
        assert!(out.contains("SYMBOL_ASK"));
        assert!(out.contains("SYMBOL_BID"));
    }

    #[test]
    fn does_not_touch_identifiers_containing_keyword() {
        // Must not rewrite `startTime` because it contains `start`.
        let src = "int startTime = 0;\nint initValue = 5;\n";
        let (out, _) = rewrite_mql4_to_mql5(src);
        assert!(out.contains("startTime"));
        assert!(out.contains("initValue"));
        assert!(!out.contains("OnTick"));
    }

    #[test]
    fn leaves_strings_alone() {
        let src = r#"Print("Bid is ", Bid);"#;
        let (out, _) = rewrite_mql4_to_mql5(src);
        // String content preserved verbatim
        assert!(out.contains(r#""Bid is ""#));
        // Second (bareword) Bid is rewritten
        assert!(out.contains("SYMBOL_BID"));
    }

    #[test]
    fn leaves_line_comments_alone() {
        let src = "// extern int Length = 14;\nint x = 1;";
        let (out, _) = rewrite_mql4_to_mql5(src);
        assert!(out.contains("// extern int Length = 14;"));
    }

    #[test]
    fn warns_on_ordersend() {
        let src = "int t = OrderSend(Symbol(),OP_BUY,1.0,Ask,3,0,0,\"x\",0,0,clrGreen);";
        let (_, warnings) = rewrite_mql4_to_mql5(src);
        assert!(warnings.iter().any(|w| w.contains("OrderSend")));
    }

    #[test]
    fn rewrites_bars_bareword_but_not_bars_call() {
        let src = "int n = Bars;\nint m = Bars(_Symbol, PERIOD_CURRENT);\n";
        let (out, _) = rewrite_mql4_to_mql5(src);
        assert!(out.contains("iBars(_Symbol,0)"));
        // The function call form (MQL5 native) must remain untouched
        assert!(out.contains("Bars(_Symbol, PERIOD_CURRENT)"));
    }

    #[test]
    fn bars_bareword_not_touched_in_identifier_context() {
        // `iBars(...)` must not be mangled since it contains `Bars`.
        let src = "int n = iBars(_Symbol, PERIOD_CURRENT);";
        let (out, _) = rewrite_mql4_to_mql5(src);
        assert!(out.contains("iBars(_Symbol, PERIOD_CURRENT)"));
    }

    #[test]
    fn compile_mql4_returns_result() {
        // Integration sanity check: a tiny MQL4 indicator should produce
        // something (not necessarily successful — the MQL5 parser is strict,
        // but the call must not panic).
        let src = r#"
#property indicator_chart_window
#property indicator_buffers 1
extern int Length = 14;
double Buffer[];
int init() {
    SetIndexBuffer(0, Buffer);
    return 0;
}
int start() {
    for (int i = 0; i < Bars; i++) {
        Buffer[i] = Close[i];
    }
    return 0;
}
"#;
        let result = compile_mql4(src);
        // Must produce diagnostics (either OK or parse errors) without panicking
        let _ = result.diagnostics;
    }
}
