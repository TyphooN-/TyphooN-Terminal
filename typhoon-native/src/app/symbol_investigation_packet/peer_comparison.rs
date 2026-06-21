use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_sector_peer_comparison(
        &self,
        p: &mut String,
        sym_upper: &str,
        fund: Option<&typhoon_engine::core::fundamentals::Fundamentals>,
    ) {
        // Gather the sector peers from app state; the median comparison table is
        // the pure `format::write_sector_peer_comparison` (ADR-125 Phase 1 step 2).
        if let Some(f) = fund {
            if !f.sector.is_empty() {
                let peers: Vec<&typhoon_engine::core::fundamentals::Fundamentals> = self
                    .bg
                    .all_fundamentals
                    .iter()
                    .filter(|o| {
                        o.sector.eq_ignore_ascii_case(&f.sector)
                            && !o.symbol.eq_ignore_ascii_case(&sym_upper)
                    })
                    .collect();
                super::format::write_sector_peer_comparison(p, f, &peers);
            }
        }
    }
}
