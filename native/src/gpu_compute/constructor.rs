//! GpuCompute constructor and pipeline initialization.

use std::sync::Arc;

use super::{
    ADX_SHADER, ANCHORED_VWAP_SHADER, ATR_PROJECTION_SHADER, ATR_SHADER, BETTER_VOLUME_SHADER,
    BOLLINGER_SHADER, BOP_SHADER, CCI_GPU_SHADER, CCI_SHADER, CMO_SHADER, DISPARITY_SHADER,
    DONCHIAN_SHADER, EHLERS_CG_SHADER, EHLERS_CYBER_SHADER, EHLERS_DECYCLER_SHADER,
    EHLERS_EBSW_SHADER, EHLERS_ITL_SHADER, EHLERS_MAMA_SHADER, EHLERS_ROOF_SHADER,
    EHLERS_SUPERSMOOTHER_SHADER, EMA_SHADER, FISHER_SHADER, FRACTALS_SHADER, GpuCompute,
    HMA_SHADER, ICHIMOKU_SHADER, KAMA_SHADER, KELTNER_SHADER, MACD_SHADER, MFI_SHADER,
    MOMENTUM_SHADER, OBV_GPU_SHADER, OBV_SHADER, PPO_SHADER, PREV_LEVELS_SHADER, PSAR_SHADER,
    QSTICK_SHADER, REGRESSION_SHADER, RSI_SHADER, SMA_SHADER, SQUEEZE_SHADER, STDDEV_SHADER,
    STOCHASTIC_SHADER, STOCHRSI_SHADER, SUPERTREND_SHADER, SUPPLY_DEMAND_SHADER, TRIX_SHADER,
    ULTOSC_SHADER, VAR_OSCILLATOR_SHADER, WILLIAMS_R_SHADER, WMA_SHADER,
    create_indicator_bind_group_layout, create_indicator_pipeline_layout,
    create_multi_indicator_bind_group_layout, create_multi_indicator_pipeline_layout,
    make_indicator_pipeline, make_multi_indicator_pipeline,
};

impl GpuCompute {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        let device_ref = device.as_ref();
        let bind_group_layout = create_indicator_bind_group_layout(device_ref);
        let multi_bind_group_layout = create_multi_indicator_bind_group_layout(device_ref);
        let pipeline_layout = create_indicator_pipeline_layout(device_ref, &bind_group_layout);
        let multi_pipeline_layout =
            create_multi_indicator_pipeline_layout(device_ref, &multi_bind_group_layout);

        let sma_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "sma_pipeline", SMA_SHADER);
        let ema_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "ema_pipeline", EMA_SHADER);
        let rsi_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "rsi_pipeline", RSI_SHADER);
        let kama_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "kama_pipeline", KAMA_SHADER);
        let atr_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "atr_pipeline", ATR_SHADER);
        let bollinger_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "bollinger_pipeline",
            BOLLINGER_SHADER,
        );
        let macd_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "macd_pipeline", MACD_SHADER);
        let fisher_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "fisher_pipeline",
            FISHER_SHADER,
        );
        let stochastic_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "stochastic_pipeline",
            STOCHASTIC_SHADER,
        );
        let adx_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "adx_pipeline", ADX_SHADER);
        let wma_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "wma_pipeline", WMA_SHADER);
        let cci_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "cci_pipeline", CCI_SHADER);
        let williams_r_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "williams_r_pipeline",
            WILLIAMS_R_SHADER,
        );
        let obv_pipeline = make_multi_indicator_pipeline(
            device_ref,
            &multi_pipeline_layout,
            "obv_pipeline",
            OBV_SHADER,
        );
        let momentum_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "momentum_pipeline",
            MOMENTUM_SHADER,
        );
        let cmo_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "cmo_pipeline", CMO_SHADER);
        let qstick_pipeline = make_multi_indicator_pipeline(
            device_ref,
            &multi_pipeline_layout,
            "qstick_pipeline",
            QSTICK_SHADER,
        );
        let disparity_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "disparity_pipeline",
            DISPARITY_SHADER,
        );
        let bop_pipeline = make_multi_indicator_pipeline(
            device_ref,
            &multi_pipeline_layout,
            "bop_pipeline",
            BOP_SHADER,
        );
        let stddev_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "stddev_pipeline",
            STDDEV_SHADER,
        );
        let mfi_pipeline = make_multi_indicator_pipeline(
            device_ref,
            &multi_pipeline_layout,
            "mfi_pipeline",
            MFI_SHADER,
        );
        let trix_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "trix_pipeline", TRIX_SHADER);
        let ppo_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "ppo_pipeline", PPO_SHADER);
        let ultosc_pipeline = make_multi_indicator_pipeline(
            device_ref,
            &multi_pipeline_layout,
            "ultosc_pipeline",
            ULTOSC_SHADER,
        );
        let stochrsi_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "stochrsi_pipeline",
            STOCHRSI_SHADER,
        );
        let var_osc_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "var_osc_pipeline",
            VAR_OSCILLATOR_SHADER,
        );
        let psar_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "psar_pipeline", PSAR_SHADER);
        let ichimoku_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "ichimoku_pipeline",
            ICHIMOKU_SHADER,
        );
        let cci_ohlc_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "cci_ohlc_pipeline",
            CCI_GPU_SHADER,
        );
        let obv_gpu_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "obv_gpu_pipeline",
            OBV_GPU_SHADER,
        );
        let ehlers_ss_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "ehlers_ss_pipeline",
            EHLERS_SUPERSMOOTHER_SHADER,
        );
        let ehlers_dec_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "ehlers_dec_pipeline",
            EHLERS_DECYCLER_SHADER,
        );
        let fractals_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "fractals_pipeline",
            FRACTALS_SHADER,
        );
        let ehlers_itl_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "ehlers_itl_pipeline",
            EHLERS_ITL_SHADER,
        );
        let ehlers_cyber_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "ehlers_cyber_pipeline",
            EHLERS_CYBER_SHADER,
        );
        let ehlers_cg_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "ehlers_cg_pipeline",
            EHLERS_CG_SHADER,
        );
        let ehlers_roof_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "ehlers_roof_pipeline",
            EHLERS_ROOF_SHADER,
        );
        let ehlers_ebsw_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "ehlers_ebsw_pipeline",
            EHLERS_EBSW_SHADER,
        );
        let ehlers_mama_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "ehlers_mama_pipeline",
            EHLERS_MAMA_SHADER,
        );
        let hma_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "hma_pipeline", HMA_SHADER);
        let sd_zones_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "sd_zones_pipeline",
            SUPPLY_DEMAND_SHADER,
        );
        let atr_proj_pipeline = make_multi_indicator_pipeline(
            device_ref,
            &multi_pipeline_layout,
            "atr_proj_pipeline",
            ATR_PROJECTION_SHADER,
        );
        let better_vol_pipeline = make_multi_indicator_pipeline(
            device_ref,
            &multi_pipeline_layout,
            "better_vol_pipeline",
            BETTER_VOLUME_SHADER,
        );
        let anchored_vwap_pipeline = make_multi_indicator_pipeline(
            device_ref,
            &multi_pipeline_layout,
            "anchored_vwap_pipeline",
            ANCHORED_VWAP_SHADER,
        );
        // ADR-094: GPU parity shaders
        let supertrend_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "supertrend_pipeline",
            SUPERTREND_SHADER,
        );
        let donchian_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "donchian_pipeline",
            DONCHIAN_SHADER,
        );
        let keltner_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "keltner_pipeline",
            KELTNER_SHADER,
        );
        let regression_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "regression_pipeline",
            REGRESSION_SHADER,
        );
        let squeeze_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "squeeze_pipeline",
            SQUEEZE_SHADER,
        );
        let prev_levels_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "prev_levels_pipeline",
            PREV_LEVELS_SHADER,
        );

        Self {
            device,
            queue,
            open_buffer: None,
            bar_buffer: None,
            ohlc_buffer: None,
            mid_buffer: None,
            vol_buffer: None,
            bar_count: 0,
            pooled_bar_count: 0,
            sma_buffer: None,
            ema_buffer: None,
            sma_pipeline,
            ema_pipeline,
            rsi_pipeline,
            kama_pipeline,
            atr_pipeline,
            bollinger_pipeline,
            macd_pipeline,
            fisher_pipeline,
            stochastic_pipeline,
            adx_pipeline,
            wma_pipeline,
            cci_pipeline,
            williams_r_pipeline,
            obv_pipeline,
            momentum_pipeline,
            cmo_pipeline,
            qstick_pipeline,
            disparity_pipeline,
            bop_pipeline,
            stddev_pipeline,
            mfi_pipeline,
            trix_pipeline,
            ppo_pipeline,
            ultosc_pipeline,
            stochrsi_pipeline,
            var_osc_pipeline,
            psar_pipeline,
            ichimoku_pipeline,
            cci_ohlc_pipeline,
            obv_gpu_pipeline,
            ehlers_ss_pipeline,
            ehlers_dec_pipeline,
            fractals_pipeline,
            ehlers_itl_pipeline,
            ehlers_cyber_pipeline,
            ehlers_cg_pipeline,
            ehlers_roof_pipeline,
            ehlers_ebsw_pipeline,
            ehlers_mama_pipeline,
            hma_pipeline,
            sd_zones_pipeline,
            atr_proj_pipeline,
            better_vol_pipeline,
            anchored_vwap_pipeline,
            supertrend_pipeline,
            donchian_pipeline,
            keltner_pipeline,
            regression_pipeline,
            squeeze_pipeline,
            prev_levels_pipeline,
            bind_group_layout,
            multi_bind_group_layout,
            readback_buffer: None,
            ind_out_buffer: None,
            ind_params_buffer: None,
            custom_in_buffer: None,
            custom_out_buffer: None,
            multi_params_buffer: None,
            indicator_bind_group: None,
            custom_bind_group: None,
            multi_bind_group: None,
        }
    }
}
