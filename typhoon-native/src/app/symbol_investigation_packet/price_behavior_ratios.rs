use super::*;

impl TyphooNApp {
    pub(super) fn write_price_behavior_ratios(&self, p: &mut String, sym_upper: &str) {
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use super::format;
                use typhoon_engine::core::research as rx;

                // Gather each snapshot from the research DB and hand it to the pure
                // formatter (ADR-125 Phase 1 step 2). The INSUFFICIENT_DATA guards
                // live inside the formatters.
                if let Ok(Some(sr)) = rx::get_sharpr(&conn, &sym_upper) {
                    format::write_sharpr(p, &sr);
                }
                if let Ok(Some(er)) = rx::get_effratio(&conn, &sym_upper) {
                    format::write_effratio(p, &er);
                }
                if let Ok(Some(wb)) = rx::get_wickbias(&conn, &sym_upper) {
                    format::write_wickbias(p, &wb);
                }
                if let Ok(Some(vv)) = rx::get_volofvol(&conn, &sym_upper) {
                    format::write_volofvol(p, &vv);
                }
            }
        }
    }
}
