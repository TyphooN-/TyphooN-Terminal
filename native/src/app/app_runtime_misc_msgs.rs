use super::*;

impl TyphooNApp {
    pub(super) fn handle_misc_broker_msg(&mut self, msg: BrokerMsg) {
        match msg {
            BrokerMsg::UnusualVolumeResults(results) => {
                self.log.push_back(LogEntry::info(format!(
                    "Unusual volume: {} symbols flagged",
                    results.len()
                )));
                self.unusual_volume_results = results;
            }
            BrokerMsg::MarketClock(msg) => {
                self.market_clock_status = msg.clone();
                self.log.push_back(LogEntry::info(msg));
            }
            _ => {}
        }
    }
}
