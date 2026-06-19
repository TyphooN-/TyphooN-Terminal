use super::*;

mod technical_indicators;
mod valuation;
mod analytics;
mod volatility;
mod risk;
mod squeeze;
mod breakout;

pub(super) fn handle_research_compute_command(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        cmd @ (BrokerCmd::ComputeDdmSnapshot { .. } | BrokerCmd::ComputeRelativeValuation { .. } | BrokerCmd::ComputeDcfSnapshot { .. } | BrokerCmd::ComputeSvmSnapshot { .. }) => {
            valuation::handle_valuation_compute(cmd, broker_msg_tx_clone.clone(), shared_cache_broker.clone());
        }
        cmd @ BrokerCmd::ComputeIvolSnapshot { .. } => {
            volatility::handle_volatility_compute(cmd, broker_msg_tx_clone.clone(), shared_cache_broker.clone());
        }
        // ── Round 9 analytics
        cmd @ (BrokerCmd::ComputeSeasonalitySnapshot { .. } | BrokerCmd::ComputeCorrelationMatrix { .. } | BrokerCmd::ComputeTotalReturnSnapshot { .. } | BrokerCmd::ComputeTechnicalsSnapshot { .. } | BrokerCmd::ComputeVolSkewSnapshot { .. }) => {
            analytics::handle_analytics_compute(cmd, broker_msg_tx_clone.clone(), shared_cache_broker.clone());
        }
        // ── risk / round10+ computes (leverage, accruals, realized vol, fcf, short, altman, dagostino, bai-perron, kupiec)
        cmd @ (BrokerCmd::ComputeLeverageSnapshot { .. } | BrokerCmd::ComputeAccrualsSnapshot { .. } | BrokerCmd::ComputeRealizedVolSnapshot { .. } | BrokerCmd::ComputeFcfYieldSnapshot { .. } | BrokerCmd::ComputeShortInterestSnapshot { .. } | BrokerCmd::ComputeAltmanZSnapshot { .. } | BrokerCmd::ComputeDagostinoSnapshot { .. } | BrokerCmd::ComputeBaiPerronSnapshot { .. } | BrokerCmd::ComputeKupiecPofSnapshot { .. }) => {
            risk::handle_risk_compute(cmd, broker_msg_tx_clone.clone(), shared_cache_broker.clone());
        }
        // ── web article ingestion handler ──
        _ => unreachable!("non research-compute command routed to research_compute"),
    }
}
