use super::prelude::*;

mod adaptive_moving_average;
mod alligator_gator;
mod bands_accumulation;
mod candlestick_patterns;
mod classic_momentum_trend;
mod directional_movement;
mod ht_transforms;
mod linear_regression;
mod macd_variants;
mod momentum_breadth_oscillators;
mod momentum_oscillators;
mod momentum_tail;
mod moving_average_variants;
mod oscillator_flow;
mod participation_pressure;
mod prelude;
mod price_stat_transforms;
mod price_trend_transforms;
mod rate_of_change;
mod residual_trend_oscillators;
mod statistical_math;
mod trend_channels;
mod trend_projection_overlays;
mod trend_volume_momentum;
mod volatility_pressure;
mod volume_flow_oscillators;
mod volume_volatility_bands;

pub(super) fn handle_technical_indicator_command(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        // ── squeeze / volatility-breakout group
        cmd @ (BrokerCmd::ComputeSqueezeSnapshot { .. }
        | BrokerCmd::ComputeSqueezeRankSnapshot { .. }
        | BrokerCmd::RefreshSqueezeWatchlist { .. }) => {
            super::squeeze::handle_squeeze_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // ── breakout / channel
        cmd @ (BrokerCmd::ComputeBbsqueezeSnapshot { .. }
        | BrokerCmd::ComputeDonchianSnapshot { .. }) => {
            breakout::handle_breakout_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }

        cmd @ (BrokerCmd::ComputeKamaSnapshot { .. }
        | BrokerCmd::ComputeIchimokuSnapshot { .. }
        | BrokerCmd::ComputeSupertrendSnapshot { .. }
        | BrokerCmd::ComputeKeltnerSnapshot { .. }) => {
            trend_channels::handle_trend_channel_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        cmd @ (BrokerCmd::ComputeFisherSnapshot { .. }
        | BrokerCmd::ComputeAroonSnapshot { .. }
        | BrokerCmd::ComputeAdxSnapshot { .. }
        | BrokerCmd::ComputeCciSnapshot { .. }
        | BrokerCmd::ComputeCmfSnapshot { .. }
        | BrokerCmd::ComputeMfiSnapshot { .. }
        | BrokerCmd::ComputePsarSnapshot { .. }) => {
            oscillator_flow::handle_oscillator_flow_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        cmd @ (BrokerCmd::ComputeVortexSnapshot { .. }
        | BrokerCmd::ComputeChopSnapshot { .. }
        | BrokerCmd::ComputeObvSnapshot { .. }
        | BrokerCmd::ComputeTrixSnapshot { .. }
        | BrokerCmd::ComputeHmaSnapshot { .. }) => {
            trend_volume_momentum::handle_trend_volume_momentum_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        cmd @ (BrokerCmd::ComputePpoSnapshot { .. }
        | BrokerCmd::ComputeDpoSnapshot { .. }
        | BrokerCmd::ComputeKstSnapshot { .. }
        | BrokerCmd::ComputeUltoscSnapshot { .. }
        | BrokerCmd::ComputeWillrSnapshot { .. }) => {
            momentum_oscillators::handle_momentum_oscillator_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        cmd @ (BrokerCmd::ComputeMassSnapshot { .. }
        | BrokerCmd::ComputeChaikoscSnapshot { .. }
        | BrokerCmd::ComputeKlingerSnapshot { .. }
        | BrokerCmd::ComputeStochRsiSnapshot { .. }
        | BrokerCmd::ComputeAwesomeSnapshot { .. }) => {
            volume_flow_oscillators::handle_volume_flow_oscillator_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        cmd @ (BrokerCmd::ComputeEfiSnapshot { .. }
        | BrokerCmd::ComputeEmvSnapshot { .. }
        | BrokerCmd::ComputeNviSnapshot { .. }
        | BrokerCmd::ComputePviSnapshot { .. }
        | BrokerCmd::ComputeCoppockSnapshot { .. }
        | BrokerCmd::ComputeCmoSnapshot { .. }
        | BrokerCmd::ComputeQstickSnapshot { .. }
        | BrokerCmd::ComputeDisparitySnapshot { .. }) => {
            participation_pressure::handle_participation_pressure_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        cmd @ (BrokerCmd::ComputeBopSnapshot { .. }
        | BrokerCmd::ComputeSchaffSnapshot { .. }
        | BrokerCmd::ComputeStochSnapshot { .. }
        | BrokerCmd::ComputeMacdSnapshot { .. }
        | BrokerCmd::ComputeVwapSnapshot { .. }
        | BrokerCmd::ComputeMcgdSnapshot { .. }
        | BrokerCmd::ComputeRwiSnapshot { .. }) => {
            classic_momentum_trend::handle_classic_momentum_trend_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        cmd @ (BrokerCmd::ComputeDemaSnapshot { .. }
        | BrokerCmd::ComputeTemaSnapshot { .. }
        | BrokerCmd::ComputeLinregSnapshot { .. }
        | BrokerCmd::ComputePivotsSnapshot { .. }
        | BrokerCmd::ComputeHeikinSnapshot { .. }) => {
            price_trend_transforms::handle_price_trend_transform_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // ── compute handlers ──
        cmd @ (BrokerCmd::ComputeAlmaSnapshot { .. }
        | BrokerCmd::ComputeZlemaSnapshot { .. }
        | BrokerCmd::ComputeTrimaSnapshot { .. }
        | BrokerCmd::ComputeT3Snapshot { .. }
        | BrokerCmd::ComputeVidyaSnapshot { .. }
        | BrokerCmd::ComputeSmmaSnapshot { .. }
        | BrokerCmd::ComputeGmmaSnapshot { .. }
        | BrokerCmd::ComputeMaenvSnapshot { .. }
        | BrokerCmd::ComputeMamaSnapshot { .. }
        | BrokerCmd::ComputeFramaSnapshot { .. }) => {
            moving_average_variants::handle_moving_average_variant_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }

        cmd @ (BrokerCmd::ComputeElderRaySnapshot { .. }
        | BrokerCmd::ComputeTsfSnapshot { .. }
        | BrokerCmd::ComputeRviSnapshot { .. }
        | BrokerCmd::ComputeSmiSnapshot { .. }
        | BrokerCmd::ComputeElderImpSnapshot { .. }
        | BrokerCmd::ComputeRmiSnapshot { .. }) => {
            trend_projection_overlays::handle_trend_projection_overlay_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }

        cmd @ (BrokerCmd::ComputePvtSnapshot { .. }
        | BrokerCmd::ComputeChvolSnapshot { .. }
        | BrokerCmd::ComputeBbwidthSnapshot { .. }
        | BrokerCmd::ComputeAdlSnapshot { .. }
        | BrokerCmd::ComputeVrocSnapshot { .. }
        | BrokerCmd::ComputeVhfSnapshot { .. }) => {
            volume_volatility_bands::handle_volume_volatility_bands_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }

        cmd @ (BrokerCmd::ComputeAcSnapshot { .. }
        | BrokerCmd::ComputeCrsiSnapshot { .. }
        | BrokerCmd::ComputeSebSnapshot { .. }
        | BrokerCmd::ComputeImiSnapshot { .. }
        | BrokerCmd::ComputeKdjSnapshot { .. }
        | BrokerCmd::ComputeQqeSnapshot { .. }
        | BrokerCmd::ComputePmoSnapshot { .. }
        | BrokerCmd::ComputeCfoSnapshot { .. }
        | BrokerCmd::ComputeTmfSnapshot { .. }) => {
            momentum_breadth_oscillators::handle_momentum_breadth_oscillators_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }

        BrokerCmd::ComputeSymbolExpirations { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let snap =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            let computed = research::compute_symbol_expirations(&conn, &symbol)
                                .unwrap_or_default();
                            let _ = research::upsert_symbol_expirations(&conn, &symbol, &computed);
                            computed
                        } else {
                            Default::default()
                        }
                    } else {
                        Default::default()
                    };
                let _ = msg_tx.send(BrokerMsg::SymbolExpirationsMsg(symbol, snap));
            });
        }

        cmd @ (BrokerCmd::ComputeAlligatorSnapshot { .. }
        | BrokerCmd::ComputeGatorSnapshot { .. }
        | BrokerCmd::ComputeBwMfiSnapshot { .. }) => {
            alligator_gator::handle_alligator_gator_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }

        cmd @ (BrokerCmd::ComputeFractalsSnapshot { .. }
        | BrokerCmd::ComputeIftRsiSnapshot { .. }
        | BrokerCmd::ComputeCogSnapshot { .. }
        | BrokerCmd::ComputeDidiSnapshot { .. }
        | BrokerCmd::ComputeDemarkerSnapshot { .. }
        | BrokerCmd::ComputeMesaSineSnapshot { .. }
        | BrokerCmd::ComputeIbsSnapshot { .. }
        | BrokerCmd::ComputeLaguerreRsiSnapshot { .. }
        | BrokerCmd::ComputeZigzagSnapshot { .. }
        | BrokerCmd::ComputePgoSnapshot { .. }) => {
            residual_trend_oscillators::handle_residual_trend_oscillator_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }

        cmd @ (BrokerCmd::ComputeVwmaSnapshot { .. }
        | BrokerCmd::ComputeStddevSnapshot { .. }
        | BrokerCmd::ComputeWmaSnapshot { .. }
        | BrokerCmd::ComputeRainbowSnapshot { .. }
        | BrokerCmd::ComputeMidpointSnapshot { .. }
        | BrokerCmd::ComputeMidpriceSnapshot { .. }
        | BrokerCmd::ComputeAvgpriceSnapshot { .. }
        | BrokerCmd::ComputeMedpriceSnapshot { .. }
        | BrokerCmd::ComputeTypPriceSnapshot { .. }
        | BrokerCmd::ComputeWclPriceSnapshot { .. }
        | BrokerCmd::ComputeVarianceSnapshot { .. }) => {
            price_stat_transforms::handle_price_stat_transform_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }

        cmd @ (BrokerCmd::ComputeHtTrendlineSnapshot { .. }
        | BrokerCmd::ComputeHtDcperiodSnapshot { .. }
        | BrokerCmd::ComputeHtTrendmodeSnapshot { .. }
        | BrokerCmd::ComputeHtDcphaseSnapshot { .. }
        | BrokerCmd::ComputeHtSineSnapshot { .. }
        | BrokerCmd::ComputeHtPhasorSnapshot { .. }) => {
            ht_transforms::handle_ht_transforms_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // ── handlers ──
        cmd @ (BrokerCmd::ComputeMassIndexSnapshot { .. }
        | BrokerCmd::ComputeNatrSnapshot { .. }
        | BrokerCmd::ComputeTtmSqueezeSnapshot { .. }
        | BrokerCmd::ComputeForceIndexSnapshot { .. }
        | BrokerCmd::ComputeTrangeSnapshot { .. }) => {
            volatility_pressure::handle_volatility_pressure_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // ── handlers ──
        cmd @ (BrokerCmd::ComputeLinearregSlopeSnapshot { .. }
        | BrokerCmd::ComputeLinearregSnapshot { .. }
        | BrokerCmd::ComputeLinearregAngleSnapshot { .. }
        | BrokerCmd::ComputeLinearRegInterceptSnapshot { .. }) => {
            linear_regression::handle_linear_regression_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        cmd @ (BrokerCmd::ComputeAccbandsSnapshot { .. }
        | BrokerCmd::ComputeBbandsSnapshot { .. }
        | BrokerCmd::ComputeAdSnapshot { .. }
        | BrokerCmd::ComputeAdoscSnapshot { .. }) => {
            bands_accumulation::handle_bands_accumulation_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        cmd @ (BrokerCmd::ComputeStochfSnapshot { .. }
        | BrokerCmd::ComputeApoSnapshot { .. }
        | BrokerCmd::ComputeMomSnapshot { .. }
        | BrokerCmd::ComputeAroonoscSnapshot { .. }) => {
            momentum_tail::handle_momentum_tail_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // ── handlers ──
        // ── handlers ──
        cmd @ (BrokerCmd::ComputeSarextSnapshot { .. }
        | BrokerCmd::ComputeAdxrSnapshot { .. }
        | BrokerCmd::ComputePlusDiSnapshot { .. }
        | BrokerCmd::ComputeMinusDiSnapshot { .. }
        | BrokerCmd::ComputePlusDmSnapshot { .. }
        | BrokerCmd::ComputeMinusDmSnapshot { .. }
        | BrokerCmd::ComputeDxSnapshot { .. }) => {
            directional_movement::handle_directional_movement_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // ── AVGPRICE / MEDPRICE / TYPPRICE / WCLPRICE / VARIANCE ──
        // ── PLUS_DI / MINUS_DI / PLUS_DM / MINUS_DM / DX ──
        // ── handlers ──
        cmd @ (BrokerCmd::ComputeRocSnapshot { .. }
        | BrokerCmd::ComputeRocpSnapshot { .. }
        | BrokerCmd::ComputeRocrSnapshot { .. }
        | BrokerCmd::ComputeRocr100Snapshot { .. }) => {
            rate_of_change::handle_rate_of_change_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        cmd @ (BrokerCmd::ComputeCorrelSnapshot { .. }
        | BrokerCmd::ComputeMinSnapshot { .. }
        | BrokerCmd::ComputeMaxSnapshot { .. }
        | BrokerCmd::ComputeMinMaxSnapshot { .. }
        | BrokerCmd::ComputeMinIndexSnapshot { .. }
        | BrokerCmd::ComputeMaxIndexSnapshot { .. }
        | BrokerCmd::ComputeSumSnapshot { .. }
        | BrokerCmd::ComputeMinMaxIndexSnapshot { .. }) => {
            statistical_math::handle_statistical_math_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // ── handlers ──
        // ── broker handlers ──
        // ── handlers ──
        cmd @ (BrokerCmd::ComputeMacdextSnapshot { .. }
        | BrokerCmd::ComputeMacdfixSnapshot { .. }) => {
            macd_variants::handle_macd_variant_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        cmd @ BrokerCmd::ComputeMavpSnapshot { .. } => {
            adaptive_moving_average::handle_adaptive_moving_average_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // ── CDL* handlers ──
        cmd @ (BrokerCmd::ComputeCdlDojiSnapshot { .. }
        | BrokerCmd::ComputeCdlHammerSnapshot { .. }
        | BrokerCmd::ComputeCdlShootingStarSnapshot { .. }
        | BrokerCmd::ComputeCdlEngulfingSnapshot { .. }
        | BrokerCmd::ComputeCdlHaramiSnapshot { .. }
        | BrokerCmd::ComputeCdlMorningStarSnapshot { .. }
        | BrokerCmd::ComputeCdlEveningStarSnapshot { .. }
        | BrokerCmd::ComputeCdlThreeBlackCrowsSnapshot { .. }
        | BrokerCmd::ComputeCdlThreeWhiteSoldiersSnapshot { .. }
        | BrokerCmd::ComputeCdlDarkCloudCoverSnapshot { .. }
        | BrokerCmd::ComputeCdlPiercingSnapshot { .. }
        | BrokerCmd::ComputeCdlDragonflyDojiSnapshot { .. }
        | BrokerCmd::ComputeCdlGravestoneDojiSnapshot { .. }
        | BrokerCmd::ComputeCdlHangingManSnapshot { .. }
        | BrokerCmd::ComputeCdlInvertedHammerSnapshot { .. }
        | BrokerCmd::ComputeCdlHaramiCrossSnapshot { .. }
        | BrokerCmd::ComputeCdlLongLeggedDojiSnapshot { .. }
        | BrokerCmd::ComputeCdlMarubozuSnapshot { .. }
        | BrokerCmd::ComputeCdlSpinningTopSnapshot { .. }
        | BrokerCmd::ComputeCdlTristarSnapshot { .. }
        | BrokerCmd::ComputeCdlDojiStarSnapshot { .. }
        | BrokerCmd::ComputeCdlMorningDojiStarSnapshot { .. }
        | BrokerCmd::ComputeCdlEveningDojiStarSnapshot { .. }
        | BrokerCmd::ComputeCdlAbandonedBabySnapshot { .. }
        | BrokerCmd::ComputeCdlThreeInsideSnapshot { .. }
        | BrokerCmd::ComputeCdlBeltHoldSnapshot { .. }
        | BrokerCmd::ComputeCdlClosingMarubozuSnapshot { .. }
        | BrokerCmd::ComputeCdlHighWaveSnapshot { .. }
        | BrokerCmd::ComputeCdlLongLineSnapshot { .. }
        | BrokerCmd::ComputeCdlShortLineSnapshot { .. }
        | BrokerCmd::ComputeCdlCounterattackSnapshot { .. }
        | BrokerCmd::ComputeCdlHomingPigeonSnapshot { .. }
        | BrokerCmd::ComputeCdlInNeckSnapshot { .. }
        | BrokerCmd::ComputeCdlOnNeckSnapshot { .. }
        | BrokerCmd::ComputeCdlThrustingSnapshot { .. }
        | BrokerCmd::ComputeCdlTwoCrowsSnapshot { .. }
        | BrokerCmd::ComputeCdlThreeLineStrikeSnapshot { .. }
        | BrokerCmd::ComputeCdlThreeOutsideSnapshot { .. }
        | BrokerCmd::ComputeCdlMatchingLowSnapshot { .. }
        | BrokerCmd::ComputeCdlSeparatingLinesSnapshot { .. }
        | BrokerCmd::ComputeCdlStickSandwichSnapshot { .. }
        | BrokerCmd::ComputeCdlRickshawManSnapshot { .. }
        | BrokerCmd::ComputeCdlTakuriSnapshot { .. }
        | BrokerCmd::ComputeCdlThreeStarsInSouthSnapshot { .. }
        | BrokerCmd::ComputeCdlIdenticalThreeCrowsSnapshot { .. }
        | BrokerCmd::ComputeCdlKickingSnapshot { .. }
        | BrokerCmd::ComputeCdlKickingByLengthSnapshot { .. }
        | BrokerCmd::ComputeCdlLadderBottomSnapshot { .. }
        | BrokerCmd::ComputeCdlUniqueThreeRiverSnapshot { .. }
        | BrokerCmd::ComputeCdlAdvanceBlockSnapshot { .. }
        | BrokerCmd::ComputeCdlBreakawaySnapshot { .. }
        | BrokerCmd::ComputeCdlGapSideSideWhiteSnapshot { .. }
        | BrokerCmd::ComputeCdlUpsideGapTwoCrowsSnapshot { .. }
        | BrokerCmd::ComputeCdlXSideGapThreeMethodsSnapshot { .. }
        | BrokerCmd::ComputeCdlConcealBabySwallowSnapshot { .. }
        | BrokerCmd::ComputeCdlHikkakeSnapshot { .. }
        | BrokerCmd::ComputeCdlHikkakeModSnapshot { .. }
        | BrokerCmd::ComputeCdlMatHoldSnapshot { .. }
        | BrokerCmd::ComputeCdlRiseFallThreeMethodsSnapshot { .. }
        | BrokerCmd::ComputeCdlStalledPatternSnapshot { .. }
        | BrokerCmd::ComputeCdlTasukiGapSnapshot { .. }) => {
            candlestick_patterns::handle_candlestick_pattern_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // ── handlers — CDL* 3-bar / 2-bar patterns ──
        // ── handlers — CDL* piercing / doji variants / hammer mirrors ──
        // ── handlers — CDL* harami cross / long-legged doji / marubozu / spinning top / tristar ──
        // ── handlers — CDL* doji star / morning doji star / evening doji star / abandoned baby / three inside ──
        // ── handlers — CDL* belt hold / closing marubozu / high wave / long line / short line ──
        // ── handlers — CDL* counterattack / homing pigeon / in-neck / on-neck / thrusting ──
        // ── handlers — additional CDL* parity windows ──
        // ── handlers — harder CDL* parity windows ──
        // ── handlers — additional multi-bar CDL* parity windows ──
        // ── handlers — stateful CDL* parity windows ──
        // ── handlers — final CDL* parity windows ──
        // ── (Quant Stats) handlers ──
        _ => unreachable!("non-technical-indicator command routed to technical indicator handler"),
    }
}
