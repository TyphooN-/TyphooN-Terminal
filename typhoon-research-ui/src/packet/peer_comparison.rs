use typhoon_engine::core::fundamentals::Fundamentals;

/// Sector peer comparison. ADR-125 Phase 1 step 3 — a free function over engine
/// slices (no `TyphooNApp`): the caller passes the symbol's resolved record and the
/// full `all_fundamentals`; this filters the sector peers and hands them to the pure
/// table builder `format::write_sector_peer_comparison`.
pub fn write_symbol_sector_peer_comparison(
    p: &mut String,
    sym_upper: &str,
    fund: Option<&Fundamentals>,
    all_fundamentals: &[Fundamentals],
) {
    if let Some(f) = fund {
        if !f.sector.is_empty() {
            let peers: Vec<&Fundamentals> = all_fundamentals
                .iter()
                .filter(|o| {
                    o.sector.eq_ignore_ascii_case(&f.sector)
                        && !o.symbol.eq_ignore_ascii_case(sym_upper)
                })
                .collect();
            crate::format::write_sector_peer_comparison(p, f, &peers);
        }
    }
}
