use std::fmt::Write as _;
use typhoon_engine::broker::alpaca::PositionInfo;
use typhoon_engine::core::fundamentals::Fundamentals;

/// Overview block: the user's open position(s) in the symbol (primary AI context),
/// then the fundamentals header + valuation table. ADR-125 Phase 1 step 3 — a free
/// function over engine slices (no `TyphooNApp`). The dispatcher does the one
/// `all_fundamentals` lookup and passes the resolved record + position slices.
pub fn write_symbol_investigation_overview_sections(
    p: &mut String,
    sym_upper: &str,
    fund: Option<&Fundamentals>,
    live_positions: &[PositionInfo],
    kr_positions: &[PositionInfo],
) {
    // User's open positions in this symbol — emit before fundamentals so the AI
    // treats the user's exposure as primary context when answering questions like
    // "what do you think about my position?".
    let pos_section = render_user_position_section(sym_upper, live_positions, kr_positions);
    if !pos_section.is_empty() {
        let _ = write!(p, "{pos_section}");
    }

    // Fundamentals row — pure markdown via the formatter layer.
    if let Some(f) = fund {
        crate::format::write_fundamentals_overview(p, f);
    } else {
        let _ = writeln!(
            p,
            "_No fundamentals on file for this symbol. Run EVSCRAPE to populate._"
        );
        let _ = writeln!(p);
    }
}

/// Render the user's open Alpaca/Kraken position(s) in `sym_upper` as a markdown
/// block, or an empty string if none. Pure over the position slices.
fn render_user_position_section(
    sym_upper: &str,
    live_positions: &[PositionInfo],
    kr_positions: &[PositionInfo],
) -> String {
    let matches_sym = |p: &PositionInfo| p.symbol.eq_ignore_ascii_case(sym_upper) && p.qty != 0.0;
    let alpaca: Vec<&PositionInfo> = live_positions.iter().filter(|p| matches_sym(p)).collect();
    let kr: Vec<&PositionInfo> = kr_positions.iter().filter(|p| matches_sym(p)).collect();
    if alpaca.is_empty() && kr.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    let _ = writeln!(out, "### Current user position in {sym_upper}");
    let _ = writeln!(
        out,
        "*The user holds the following open position(s) in this symbol. When answering questions like \"what do you think of my position?\" treat this as the primary context.*"
    );
    let _ = writeln!(out);

    let emit_lot = |out: &mut String, broker: &str, p: &PositionInfo| {
        let side_upper = if p.side.eq_ignore_ascii_case("short") || p.qty < 0.0 {
            "SHORT"
        } else {
            "LONG"
        };
        let abs_qty = p.qty.abs();
        let current_price = if abs_qty > 0.0 {
            p.market_value.abs() / abs_qty
        } else {
            0.0
        };
        let cost_basis = p.avg_entry_price * abs_qty;
        let unreal_pct = if cost_basis > 0.0 {
            (p.unrealized_pl / cost_basis) * 100.0
        } else {
            0.0
        };
        let sign = if p.unrealized_pl >= 0.0 { "+" } else { "" };
        let _ = writeln!(
            out,
            "- **{broker}** — {side_upper} {abs_qty:.4} @ avg {avg:.4} (current ~{cur:.4}); market value {mv:.2}; unrealized {sign}{pnl:.2} ({sign}{pct:.2}%)",
            side_upper = side_upper,
            abs_qty = abs_qty,
            avg = p.avg_entry_price,
            cur = current_price,
            mv = p.market_value,
            sign = sign,
            pnl = p.unrealized_pl,
            pct = unreal_pct,
        );
    };
    for p in &alpaca {
        emit_lot(&mut out, "Alpaca", p);
    }
    for p in &kr {
        emit_lot(&mut out, "Kraken", p);
    }
    let _ = writeln!(out);
    out
}
