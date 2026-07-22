#[test]
fn live_depth_summary_combines_sides_without_materializing_a_book() {
    let bids = vec![(99.0, 2.0), (98.0, 4.0)];
    let asks = vec![(101.0, 3.0), (102.0, 1.0), (103.0, 2.5)];

    let summary = super::live_depth_summary(&bids, &asks).unwrap();

    assert_eq!(summary.level_count, 5);
    assert_eq!(summary.max_size, 4.0);
    assert!(super::live_depth_summary(&[], &[]).is_none());
}

#[test]
fn price_view_geometry_round_trips_linear_and_log() {
    let rect = egui::Rect::from_min_size(egui::pos2(0.0, 100.0), egui::vec2(800.0, 400.0));
    for log_scale in [false, true] {
        let g = super::PriceViewGeometry {
            chart_rect: rect,
            price_min: 50.0,
            price_max: 150.0,
            log_scale,
            data_left: rect.left(),
            bar_w: 8.0,
            start_idx: 100,
        };
        // Top of the pane is price_max, bottom is price_min.
        assert!(
            (g.price_to_y(150.0) - 100.0).abs() < 0.001,
            "log={log_scale}"
        );
        assert!(
            (g.price_to_y(50.0) - 500.0).abs() < 0.001,
            "log={log_scale}"
        );
        for p in [50.0, 75.0, 100.0, 149.0] {
            let back = g.price_from_y(g.price_to_y(p));
            // y is f32 pixels, so the round trip carries float error well
            // under the 8px grab tolerance the mapping serves.
            assert!(
                ((back - p) / p).abs() < 1e-4,
                "round trip failed log={log_scale} p={p} back={back}"
            );
        }
        // Dragging down (positive dy) lowers the price on both scales.
        assert!(g.drag_price(100.0, 40.0) < 100.0, "log={log_scale}");
        assert!(g.drag_price(100.0, -40.0) > 100.0, "log={log_scale}");
    }
}

#[test]
fn format_size_is_compact_not_price_padded() {
    use super::time_axis::format_size;
    // Whole share/contract quantities render without the price-style ".0000"
    // that used to overflow the right-axis bid/ask flags off the window edge.
    assert_eq!(format_size(1200.0), "1200");
    assert_eq!(format_size(400.0), "400");
    // Large sizes abbreviate.
    assert_eq!(format_size(150_000.0), "150K");
    assert_eq!(format_size(1_500_000.0), "1.5M");
    // Fractional (crypto) sizes keep significant digits, trailing zeros trimmed.
    assert_eq!(format_size(0.005_600), "0.0056");
    assert_eq!(format_size(0.0), "0");
}

#[test]
fn indicator_value_lookup_returns_none_when_series_lags_bars() {
    let series = vec![Some(1.0), None, Some(3.0)];

    assert_eq!(super::indicator_value_at(&series, 0), Some(1.0));
    assert_eq!(super::indicator_value_at(&series, 1), None);
    assert_eq!(super::indicator_value_at(&series, 3), None);
}

#[test]
fn indicator_line_clipping_keeps_price_scale_crossing_segments() {
    let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0));
    let clipped =
        super::clip_line_segment_to_rect(egui::pos2(10.0, -50.0), egui::pos2(90.0, 150.0), rect)
            .expect("segment crosses chart pane even when both sampled y values are offscreen");

    assert!((clipped[0].x - 30.0).abs() < 0.001);
    assert!((clipped[0].y - 0.0).abs() < 0.001);
    assert!((clipped[1].x - 70.0).abs() < 0.001);
    assert!((clipped[1].y - 100.0).abs() < 0.001);
}

#[test]
fn indicator_line_clipping_rejects_fully_offscreen_segments() {
    let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0));
    assert!(
        super::clip_line_segment_to_rect(egui::pos2(10.0, -50.0), egui::pos2(90.0, -10.0), rect,)
            .is_none()
    );
}

#[test]
fn clamp_f32_bounds_accepts_inverted_tiny_pane_bounds() {
    assert_eq!(super::clamp_f32_bounds(9.0, 8.0, 7.46875), 8.0);
    assert_eq!(super::clamp_f32_bounds(7.0, 8.0, 7.46875), 7.46875);
}

#[test]
fn projection_candle_sits_in_next_slot_not_far_right_empty_space() {
    let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1000.0, 400.0));
    let x = super::adjacent_projection_candle_x(0.0, 1, 10.0, 3.5, rect)
        .expect("one empty slot after the visible bar is enough for projection candle");

    assert!((x - 15.0).abs() < 0.001);
    assert!(
        x < rect.right() - 900.0,
        "projection candle must not be pinned to far-right chart edge; x={x}"
    );
}

#[test]
fn projection_candle_is_hidden_when_next_slot_is_offscreen() {
    let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 400.0));

    assert!(super::adjacent_projection_candle_x(0.0, 10, 10.0, 3.5, rect).is_none());
}

#[test]
fn candle_countdown_uses_current_bar_boundary() {
    let last_bar = 1_700_000_000_000_i64;
    let now = last_bar + 3 * 60_000 + 15_000;

    assert_eq!(
        super::next_candle_remaining_ms_at(last_bar, super::Timeframe::M5, now),
        Some(105_000)
    );
}

#[test]
fn candle_countdown_hides_for_stale_or_closed_session_bar_data() {
    let last_bar = 1_700_000_000_000_i64;
    let now = last_bar + 17 * 60_000 + 10_000;

    assert_eq!(
        super::next_candle_remaining_ms_at(last_bar, super::Timeframe::M5, now),
        None
    );
}

#[test]
fn candle_countdown_hides_for_closed_equity_weekend_but_not_crypto() {
    let saturday_et = chrono::DateTime::parse_from_rfc3339("2026-06-13T12:00:00Z")
        .unwrap()
        .timestamp_millis();

    assert!(!super::chart_candle_countdown_allowed_at(
        "kraken-equities",
        "WOK",
        saturday_et
    ));
    assert!(!super::chart_candle_countdown_allowed_at(
        "kraken",
        "WOK",
        saturday_et
    ));
    assert!(super::chart_candle_countdown_allowed_at(
        "kraken",
        "BTC/USD",
        saturday_et
    ));
}

#[test]
fn candle_countdown_respects_friday_and_sunday_equity_weekend_boundary() {
    let friday_before_xstock_close = chrono::DateTime::parse_from_rfc3339("2026-06-12T23:59:00Z")
        .unwrap()
        .timestamp_millis();
    let friday_after_xstock_close = chrono::DateTime::parse_from_rfc3339("2026-06-13T00:01:00Z")
        .unwrap()
        .timestamp_millis();
    let sunday_before_xstock_open = chrono::DateTime::parse_from_rfc3339("2026-06-14T23:59:00Z")
        .unwrap()
        .timestamp_millis();
    let sunday_after_xstock_open = chrono::DateTime::parse_from_rfc3339("2026-06-15T00:01:00Z")
        .unwrap()
        .timestamp_millis();

    assert!(super::chart_candle_countdown_allowed_at(
        "kraken-equities",
        "WOK",
        friday_before_xstock_close
    ));
    assert!(!super::chart_candle_countdown_allowed_at(
        "kraken-equities",
        "WOK",
        friday_after_xstock_close
    ));
    assert!(!super::chart_candle_countdown_allowed_at(
        "kraken-equities",
        "WOK",
        sunday_before_xstock_open
    ));
    assert!(super::chart_candle_countdown_allowed_at(
        "kraken-equities",
        "WOK",
        sunday_after_xstock_open
    ));
}

#[test]
fn candle_countdown_formats_like_chart_axis_timer() {
    assert_eq!(super::format_candle_countdown(4_000), "00:04");
    assert_eq!(super::format_candle_countdown(65_000), "01:05");
    assert_eq!(super::format_candle_countdown(3_661_000), "1:01:01");
    assert_eq!(super::format_candle_countdown(90_000_000), "1d 01:00");
}

#[test]
fn time_axis_labels_include_dates_and_years_across_timeframes() {
    let ts = chrono::DateTime::parse_from_rfc3339("2026-06-11T14:35:00Z")
        .unwrap()
        .timestamp_millis();

    assert_eq!(
        super::format_ts(ts, super::Timeframe::M5),
        "11 Jun'26 14:35"
    );
    assert_eq!(
        super::format_ts(ts, super::Timeframe::H1),
        "11 Jun'26 14:35"
    );
    assert_eq!(super::format_ts(ts, super::Timeframe::D1), "11 Jun'26");
    assert_eq!(super::format_ts(ts, super::Timeframe::W1), "11 Jun'26");
    assert_eq!(super::format_ts(ts, super::Timeframe::MN1), "Jun 2026");
}

#[test]
fn intraday_axis_stride_climbs_the_ladder_as_bars_shrink() {
    // Wide bars (zoomed in) → fine stride; thin bars (zoomed out) → coarse.
    // H1 (60m bars): 22px bars want ~3h between labels; 1.3px bars want days.
    assert_eq!(super::intraday_axis_stride_minutes(60, 22.0, 64.0), 180);
    assert_eq!(super::intraday_axis_stride_minutes(60, 1.3, 64.0), 4320);
    // Stride never drops below the timeframe itself (H4 = 240m floor).
    assert_eq!(super::intraday_axis_stride_minutes(240, 100.0, 64.0), 240);
    // H4 over a month (≈4.5px bars) → multi-day date ticks, no intraday smear.
    assert_eq!(super::intraday_axis_stride_minutes(240, 4.5, 64.0), 4320);
}

#[test]
fn extended_hours_axis_labels_are_explicit() {
    assert_eq!(super::format_axis_price_label("EXT", 0.0924), "EXT 0.0924");
    assert_eq!(super::format_axis_price_label("C", 194.32), "C 194.3200");
}

#[test]
fn close_reference_color_uses_previous_daily_close() {
    assert_eq!(
        super::close_reference_color(0.09265, 0.09, Some(0.10065)),
        super::DOWN
    );
    assert_eq!(
        super::close_reference_color(0.102, 0.09, Some(0.10065)),
        super::UP
    );
}

#[test]
fn close_reference_color_falls_back_to_bar_open_without_prior_day() {
    assert_eq!(super::close_reference_color(9.5, 10.0, None), super::DOWN);
    assert_eq!(super::close_reference_color(10.5, 10.0, None), super::UP);
}

#[test]
fn extended_hours_symbol_badge_lists_close_ext_and_move() {
    assert_eq!(
        super::format_ext_hours_symbol_badge(100.0, 101.25, Some(98.0)),
        "Daily Close 100.0000 (+3.32%)  EXT last 101.2500  Δ/C +1.2500 (+1.25%)"
    );
    assert_eq!(
        super::format_ext_hours_symbol_badge(100.0, 99.5, Some(100.0)),
        "Daily Close 100.0000 (-0.50%)  EXT last 99.5000  Δ/C -0.5000 (-0.50%)"
    );
    assert_eq!(
        super::format_ext_hours_symbol_badge(0.0925, 0.0924, Some(0.0900)),
        "Daily Close 0.0925 (+2.67%)  EXT last 0.0924  Δ/C -0.0001 (-0.11%)"
    );
    assert_eq!(
        super::format_ext_hours_symbol_badge(0.0925, 0.0924, None),
        "Daily Close 0.0925  EXT last 0.0924  Δ/C -0.0001 (-0.11%)"
    );
}

#[test]
fn prev_candle_levels_only_show_higher_timeframes() {
    use crate::types::Timeframe;
    // (label, max chart group_rank at which the level still draws) — must
    // mirror the draw site: a level shows iff chart.group_rank() <= max_rank.
    let levels = [
        ("Prev H1", 0u8),
        ("Prev H4", 0),
        ("Prev D", 1),
        ("Prev W", 2),
        ("Prev MN", 3),
        ("Cur D", 2),
        ("Cur W", 2),
        ("Cur MN", 3),
    ];
    let visible = |tf: Timeframe| -> Vec<&'static str> {
        let rank = tf.group_rank();
        levels
            .iter()
            .filter(|(_, m)| rank <= *m)
            .map(|(l, _)| *l)
            .collect()
    };
    // Sub-hour chart shows every previous + every current level.
    assert_eq!(
        visible(Timeframe::M15),
        vec![
            "Prev H1", "Prev H4", "Prev D", "Prev W", "Prev MN", "Cur D", "Cur W", "Cur MN"
        ]
    );
    // Hourly charts drop their own H1/H4 previous; keep daily+ and all current.
    assert_eq!(
        visible(Timeframe::H1),
        vec!["Prev D", "Prev W", "Prev MN", "Cur D", "Cur W", "Cur MN"]
    );
    assert_eq!(
        visible(Timeframe::H4),
        vec!["Prev D", "Prev W", "Prev MN", "Cur D", "Cur W", "Cur MN"]
    );
    assert_eq!(
        visible(Timeframe::D1),
        vec!["Prev W", "Prev MN", "Cur D", "Cur W", "Cur MN"]
    );
    // Weekly chart keeps only MN previous + MN current; monthly+ show nothing.
    assert_eq!(visible(Timeframe::W1), vec!["Prev MN", "Cur MN"]);
    assert!(visible(Timeframe::MN1).is_empty());
}

#[test]
fn nnfx_view_uses_mql_mtf_names_when_projected_overlays_exist() {
    assert_eq!(
        super::nnfx_trend_legend_labels(true, true),
        ("MTF_MA", "MultiKAMA")
    );
    assert_eq!(
        super::nnfx_trend_legend_labels(false, false),
        ("SMA200", "KAMA(10,2,30)")
    );
}

#[test]
fn nnfx_view_suppresses_generic_current_tf_lines_when_mql_mtf_overlays_exist() {
    assert!(!super::draw_current_sma200_overlay(true, true));
    assert!(super::draw_current_sma200_overlay(true, false));
    assert!(!super::draw_current_kama_overlay(true, true));
    assert!(super::draw_current_kama_overlay(true, false));
}
