use super::*;

impl TyphooNApp {
    pub(super) fn handle_sentiment_transcripts_tape_commands(
        &mut self,
        cmd_upper: &String,
    ) -> bool {
        match cmd_upper.as_str() {
            _ if self.handle_earnings_peers_commands(cmd_upper) => {}
            "SENTIMENT" | "SOCIAL" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.sentiment_symbol = sym;
                }
                self.show_sentiment = true;
                if !self.finnhub_key.is_empty() && !self.sentiment_symbol.is_empty() {
                    self.sentiment_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchSocialSentiment {
                        symbol: self.sentiment_symbol.to_uppercase(),
                        finnhub_key: self.finnhub_key.clone(),
                    });
                }
            }
            "TRANSCRIPTS" | "CALLS" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.transcripts_symbol = sym;
                }
                self.show_transcripts = true;
                if !self.fmp_key.is_empty() && !self.transcripts_symbol.is_empty() {
                    self.transcripts_loading_list = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchTranscriptList {
                        symbol: self.transcripts_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "GLCO" | "COMMODITIES" => {
                self.show_commodities = true;
                self.commodities_loading = true;
                let _ = self.broker_tx.send(BrokerCmd::FetchCommoditiesQuotes);
            }
            "RESEARCH_SCRAPE" | "RSCRAPE" => {
                let _ = self.broker_tx.send(BrokerCmd::ResearchScrape {
                    use_alpaca: true,
                    finnhub_key: self.finnhub_key.clone(),
                    fmp_key: self.fmp_key.clone(),
                });
                self.log.push_back(LogEntry::info(
                    "Research scrape started across Alpaca universe",
                ));
            }
            "TAS" | "TIME_SALES" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.tas_symbol = sym;
                }
                self.tas_rows.clear();
                self.tas_paused = false;
                self.show_tas = true;
            }
            _ => return false,
        }
        true
    }
}
