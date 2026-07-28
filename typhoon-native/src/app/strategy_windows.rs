use super::*;

impl TyphooNApp {
    fn drain_strategy_workflow(&mut self) {
        let received = self
            .strategy_result_workflow_rx
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        let Some(result) = received else {
            return;
        };
        self.strategy_result_workflow_rx = None;
        use strategy_report_view::WorkflowWorkerResult;
        match result {
            WorkflowWorkerResult::Sealed(result) => match result {
                Ok(log) => {
                    self.strategy_result_workflow.intervention.status = format!(
                        "Sealed candidate log {} with verified content identity; ledger replay is not verified until this id is manifest-bound and the simulation rerun succeeds",
                        log.log_id()
                    );
                    self.strategy_result_workflow.intervention.sealed = Some(log);
                }
                Err(error) => {
                    self.strategy_result_workflow.intervention.status = format!("Error: {error}");
                }
            },
            WorkflowWorkerResult::Loaded(result) => match result {
                Ok(log) => {
                    self.strategy_result_workflow.intervention.entries =
                        log.interventions().to_vec();
                    self.strategy_result_workflow.intervention.status = format!(
                        "Loaded sealed log {} with verified content identity; no ledger replay was attempted because run inputs are not loaded",
                        log.log_id()
                    );
                    self.strategy_result_workflow.intervention.sealed = Some(log);
                }
                Err(error) => {
                    self.strategy_result_workflow.intervention.status = format!("Error: {error}");
                }
            },
            WorkflowWorkerResult::Exported {
                kind,
                identity,
                path,
                result,
            } => {
                self.strategy_result_workflow.export_status = match result {
                    Ok(()) => strategy_report_view::ExportStatus::Saved {
                        kind,
                        identity,
                        path,
                    },
                    Err(error) => strategy_report_view::ExportStatus::Error(error),
                };
            }
        }
    }

    fn start_strategy_workflow_command(
        &mut self,
        ctx: &egui::Context,
        command: strategy_report_view::ResultCommand,
    ) {
        if self.strategy_result_workflow_rx.is_some() {
            return;
        }
        let Some(view) = self.strategy_result_view.as_ref() else {
            return;
        };
        let (sender, receiver) = std::sync::mpsc::channel();
        use strategy_report_view::{ExportKind, ResultCommand, WorkflowWorkerResult};
        match command {
            ResultCommand::Seal(entries) => {
                let decisions = view.decisions.clone();
                self.strategy_result_workflow_rx = Some(receiver);
                let repaint = ctx.clone();
                std::thread::spawn(move || {
                    let result = strategy_report_view::seal_interventions(entries, &decisions);
                    let _ = sender.send(WorkflowWorkerResult::Sealed(result));
                    repaint.request_repaint();
                });
            }
            ResultCommand::LoadIntervention => {
                let Some(path) = rfd::FileDialog::new()
                    .set_title("Load sealed intervention log")
                    .add_filter("Intervention log JSON", &["json"])
                    .pick_file()
                else {
                    return;
                };
                let decisions = view.decisions.clone();
                self.strategy_result_workflow_rx = Some(receiver);
                let repaint = ctx.clone();
                std::thread::spawn(move || {
                    let result = strategy_report_view::load_intervention_log(&path, &decisions);
                    let _ = sender.send(WorkflowWorkerResult::Loaded(result));
                    repaint.request_repaint();
                });
            }
            ResultCommand::Export(kind) => {
                let (identity, source, stem) = match kind {
                    ExportKind::Report => (
                        view.report_id.clone(),
                        strategy_report_view::ExportSource::VerifiedBytes(
                            view.report_artifact_json.clone(),
                        ),
                        "strategy_report",
                    ),
                    ExportKind::Simulation => (
                        view.run_id.clone(),
                        strategy_report_view::ExportSource::VerifiedBytes(
                            view.simulation_report_json.clone(),
                        ),
                        "simulation_report",
                    ),
                    ExportKind::Intervention => {
                        let Some(log) = self.strategy_result_workflow.intervention.sealed.as_ref()
                        else {
                            self.strategy_result_workflow.export_status =
                                strategy_report_view::ExportStatus::Error(
                                    "seal or load and verify the intervention log before export"
                                        .into(),
                                );
                            return;
                        };
                        (
                            log.log_id().to_string(),
                            strategy_report_view::ExportSource::Intervention(log.clone()),
                            "intervention_log",
                        )
                    }
                };
                let short = identity.get(..12).unwrap_or(&identity);
                let Some(path) = rfd::FileDialog::new()
                    .set_title(format!("Export {kind:?} with identity"))
                    .add_filter("JSON", &["json"])
                    .set_file_name(format!("{stem}_{short}.json"))
                    .save_file()
                else {
                    return;
                };
                self.strategy_result_workflow.export_status =
                    strategy_report_view::ExportStatus::Working { kind };
                self.strategy_result_workflow_rx = Some(receiver);
                let repaint = ctx.clone();
                std::thread::spawn(move || {
                    let result = strategy_report_view::export_verified_source(&path, source);
                    let _ = sender.send(WorkflowWorkerResult::Exported {
                        kind,
                        identity,
                        path,
                        result,
                    });
                    repaint.request_repaint();
                });
            }
        }
    }

    fn drain_strategy_result_load(&mut self) {
        let received =
            self.strategy_result_load_rx
                .as_ref()
                .and_then(|receiver| match receiver.try_recv() {
                    Ok(result) => Some(result),
                    Err(std::sync::mpsc::TryRecvError::Empty) => None,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => Some(Err(
                        "strategy report loader stopped unexpectedly".to_string(),
                    )),
                });
        let Some(result) = received else {
            return;
        };
        self.strategy_result_load_rx = None;
        match result {
            Ok((chart_index, bars_generation, view)) => {
                let chart_is_current = self.charts.get(chart_index).is_some_and(|chart| {
                    chart.bars_generation == bars_generation
                        && chart.bars.len() == view.chart_bar_count
                        && chart.symbol_matches(&view.symbol)
                });
                if !chart_is_current {
                    self.strategy_result_status =
                        "Error: the selected chart changed while the report was loading".into();
                    return;
                }
                self.strategy_result_selected_trade =
                    strategy_report_view::clamp_selected_trade(None, view.trades.len());
                self.replay_active = false;
                self.replay_playing = false;
                self.replay_bar_idx = strategy_report_view::reset_replay_bar(view.chart_bar_count);
                self.strategy_result_status = format!(
                    "Loaded and verified report {} for run {}",
                    view.report_id.get(..8).unwrap_or(&view.report_id),
                    view.run_id.get(..8).unwrap_or(&view.run_id)
                );
                self.strategy_result_chart_tab = Some(chart_index);
                self.strategy_result_workflow =
                    strategy_report_view::ResultWorkflowState::default();
                self.strategy_result_view = Some(view);
                if let Some(chart) = self.charts.get_mut(chart_index) {
                    chart.cached_trade_overlay_frame = 0;
                }
            }
            Err(error) => {
                self.strategy_result_status = format!("Error: {error}");
            }
        }
    }

    fn start_strategy_result_load(&mut self, ctx: &egui::Context) {
        let Some(paths) = rfd::FileDialog::new()
            .set_title("Select the sealed report artifact and paired SimulationReport JSON")
            .add_filter("JSON report pair", &["json"])
            .pick_files()
        else {
            return;
        };
        if paths.len() != 2 {
            self.strategy_result_status =
                "Error: select exactly two files: one artifact and one SimulationReport JSON"
                    .into();
            return;
        }
        let Some(chart) = self.charts.get(self.active_tab) else {
            self.strategy_result_status = "Error: no active chart is available".into();
            return;
        };
        if chart.bars.is_empty() {
            self.strategy_result_status = "Error: the active chart has no timeline".into();
            return;
        }
        let chart_index = self.active_tab;
        let bars_generation = chart.bars_generation;
        let chart_symbol = chart.symbol.clone();
        let chart_bar_times_ms: Vec<_> = chart.bars.iter().map(|bar| bar.ts_ms).collect();

        self.replay_active = false;
        self.replay_playing = false;
        self.strategy_result_view = None;
        self.strategy_result_chart_tab = None;
        self.strategy_result_selected_trade = None;
        for chart in &mut self.charts {
            chart.cached_trade_overlay_frame = 0;
        }

        let paths = [paths[0].clone(), paths[1].clone()];
        let (sender, receiver) = std::sync::mpsc::channel();
        self.strategy_result_load_rx = Some(receiver);
        self.strategy_result_status = "Loading and verifying report pair…".into();
        let repaint = ctx.clone();
        std::thread::spawn(move || {
            let result =
                strategy_report_view::load_prepared_pair(paths, &chart_symbol, &chart_bar_times_ms)
                    .map(|view| (chart_index, bars_generation, view));
            let _ = sender.send(result);
            repaint.request_repaint();
        });
    }

    fn drain_sub_bar_runs(&mut self) {
        let events = self
            .strategy_run_worker
            .as_ref()
            .map(|worker| worker.poll())
            .unwrap_or_default();
        for event in events {
            let identity = event.identity();
            match event {
                strategy_sub_bar_run::StrategyRunEvent::Completed { output, .. } => {
                    let current = self
                        .charts
                        .get(output.chart.chart_index)
                        .is_some_and(|chart| {
                            chart.bars_generation == output.chart.bars_generation
                                && chart.symbol_matches(&output.chart.symbol)
                                && chart.bars.len() == output.chart.bar_times_ms.len()
                                && chart
                                    .bars
                                    .iter()
                                    .zip(output.chart.bar_times_ms.iter())
                                    .all(|(bar, time)| bar.ts_ms == *time)
                        });
                    if !current {
                        let _ = self.sub_bar_run_state.accept_terminal(
                            identity,
                            Err("active chart identity/timeline changed; stale result ignored"),
                        );
                        continue;
                    }
                    let report_id = output.view.report_id.clone();
                    let run_id = output.manifest.run_id().to_owned();
                    if self
                        .sub_bar_run_state
                        .accept_terminal(identity, Ok(&report_id))
                    {
                        self.sub_bar_run_state.status = format!(
                            "Verified run {} · report {} installed",
                            run_id.get(..12).unwrap_or(&run_id),
                            report_id.get(..12).unwrap_or(&report_id),
                        );
                        self.strategy_result_selected_trade = None;
                        self.strategy_result_chart_tab = Some(output.chart.chart_index);
                        self.strategy_result_status = self.sub_bar_run_state.status.clone();
                        self.strategy_result_workflow =
                            strategy_report_view::ResultWorkflowState::default();
                        self.strategy_result_view = Some(output.view);
                        if let Some(chart) = self.charts.get_mut(output.chart.chart_index) {
                            chart.cached_trade_overlay_frame = 0;
                        }
                    }
                }
                strategy_sub_bar_run::StrategyRunEvent::Failed { message, .. } => {
                    let _ = self
                        .sub_bar_run_state
                        .accept_terminal(identity, Err(&message));
                }
                strategy_sub_bar_run::StrategyRunEvent::Cancelled { .. } => {}
            }
        }
    }

    fn submit_sub_bar_run(&mut self) {
        let selection = match strategy_sub_bar_run::validate_run_selection(
            &self.sub_bar_run_ui.parent_dataset_id,
            &self.sub_bar_run_ui.finer_dataset_id,
            &self.dataset_inspector.records,
        ) {
            Ok(value) => value,
            Err(error) => {
                self.sub_bar_run_state.status = format!("Error: {error}");
                return;
            }
        };
        let (strategy, config, manifest) =
            match strategy_sub_bar_run::load_sealed_artifacts(&self.sub_bar_run_ui) {
                Ok(value) => value,
                Err(error) => {
                    self.sub_bar_run_state.status = format!("Error: {error}");
                    return;
                }
            };
        let binding = manifest.binding();
        if !binding
            .datasets
            .iter()
            .any(|input| input.dataset_id == selection.parent.dataset_id)
            || !binding
                .sub_bar_datasets
                .iter()
                .any(|input| input.dataset_id == selection.finer.dataset_id)
        {
            self.sub_bar_run_state.status =
                "Error: sealed run manifest must bind the selected parent and finer dataset IDs"
                    .into();
            return;
        }
        let Some(chart) = self.charts.get(self.active_tab) else {
            return;
        };
        if chart.bars.is_empty() || !chart.symbol_matches(&selection.parent.symbol) {
            self.sub_bar_run_state.status = format!(
                "Error: active chart must have a {} timeline",
                selection.parent.symbol
            );
            return;
        }
        let identity = self.sub_bar_run_state.begin_request();
        let job = strategy_sub_bar_run::StrategyRunJob {
            identity,
            strategy,
            config,
            manifest,
            chart: strategy_sub_bar_run::RunChartContext {
                chart_index: self.active_tab,
                bars_generation: chart.bars_generation,
                symbol: chart.symbol.clone(),
                bar_times_ms: std::sync::Arc::from(
                    chart.bars.iter().map(|bar| bar.ts_ms).collect::<Vec<_>>(),
                ),
            },
        };
        match self
            .strategy_run_worker
            .as_ref()
            .ok_or("verified-run worker did not start")
            .and_then(|worker| {
                worker
                    .submit(job)
                    .map_err(|_| "verified-run worker queue is busy")
            }) {
            Ok(()) => {
                self.sub_bar_run_state.status = format!(
                    "Running request {} · {} → {}s sub-bars",
                    identity.request_id, selection.parent.symbol, selection.sub_bar_seconds
                )
            }
            Err(error) => {
                let _ = self.sub_bar_run_state.accept_terminal(identity, Err(error));
            }
        }
    }

    fn drain_intervention_runs(&mut self) {
        let events = self
            .intervention_run_worker
            .as_ref()
            .map(|worker| worker.poll())
            .unwrap_or_default();
        for event in events {
            let identity = event.identity();
            match event {
                strategy_intervention_run::InterventionRunEvent::Completed { output, .. } => {
                    let current = self
                        .charts
                        .get(output.chart.chart_index)
                        .is_some_and(|chart| {
                            chart.bars_generation == output.chart.bars_generation
                                && chart.symbol_matches(&output.chart.symbol)
                                && chart.bars.len() == output.chart.bar_times_ms.len()
                                && chart
                                    .bars
                                    .iter()
                                    .zip(output.chart.bar_times_ms.iter())
                                    .all(|(bar, time)| bar.ts_ms == *time)
                        });
                    if !current {
                        let _ = self.intervention_run_state.accept_terminal(
                            identity,
                            Err("active chart identity/timeline changed; stale replay ignored"),
                        );
                        continue;
                    }
                    let identities = strategy_intervention_run::PromotedRunIdentities {
                        run_id: output.manifest.run_id().to_owned(),
                        log_id: output.log_id.clone(),
                        report_id: output.view.report_id.clone(),
                    };
                    if self
                        .intervention_run_state
                        .accept_terminal(identity, Ok(&identities))
                    {
                        self.strategy_result_selected_trade = None;
                        self.strategy_result_chart_tab = Some(output.chart.chart_index);
                        self.strategy_result_status = self.intervention_run_state.status.clone();
                        self.strategy_result_workflow =
                            strategy_report_view::ResultWorkflowState::default();
                        self.strategy_result_view = Some(output.view);
                        if let Some(chart) = self.charts.get_mut(output.chart.chart_index) {
                            chart.cached_trade_overlay_frame = 0;
                        }
                    }
                }
                strategy_intervention_run::InterventionRunEvent::Failed { message, .. } => {
                    let _ = self
                        .intervention_run_state
                        .accept_terminal(identity, Err(&message));
                }
                strategy_intervention_run::InterventionRunEvent::Cancelled { .. } => {}
            }
        }
    }

    fn submit_intervention_run(&mut self) {
        if self.intervention_run_ui.selected_dataset_ids.is_empty() {
            self.intervention_run_state.status =
                "Error: select every parent dataset bound by the run manifest".into();
            return;
        }
        let Some(chart) = self.charts.get(self.active_tab) else {
            return;
        };
        if chart.bars.is_empty() {
            self.intervention_run_state.status =
                "Error: active chart must have a timeline for report snapshot generation".into();
            return;
        }
        let identity = self.intervention_run_state.begin_request();
        let job = strategy_intervention_run::InterventionRunJob {
            identity,
            selected_dataset_ids: self
                .intervention_run_ui
                .selected_dataset_ids
                .iter()
                .cloned()
                .collect(),
            strategy_path: self.intervention_run_ui.strategy_path.clone(),
            config_path: self.intervention_run_ui.config_path.clone(),
            manifest_path: self.intervention_run_ui.manifest_path.clone(),
            intervention_log_path: self.intervention_run_ui.intervention_log_path.clone(),
            chart: strategy_sub_bar_run::RunChartContext {
                chart_index: self.active_tab,
                bars_generation: chart.bars_generation,
                symbol: chart.symbol.clone(),
                bar_times_ms: std::sync::Arc::from(
                    chart.bars.iter().map(|bar| bar.ts_ms).collect::<Vec<_>>(),
                ),
            },
        };
        match self
            .intervention_run_worker
            .as_ref()
            .ok_or("intervention-run worker did not start")
            .and_then(|worker| {
                worker
                    .submit(job)
                    .map_err(|_| "intervention-run worker queue is busy")
            }) {
            Ok(()) => {
                self.intervention_run_state.status = format!(
                    "Running exact manifest-bound intervention replay request {}",
                    identity.request_id
                );
            }
            Err(error) => {
                let _ = self
                    .intervention_run_state
                    .accept_terminal(identity, Err(error));
            }
        }
    }

    pub(super) fn render_backtest_window(&mut self, ctx: &egui::Context) {
        self.drain_strategy_result_load();
        self.drain_strategy_workflow();
        self.drain_sub_bar_runs();
        self.drain_intervention_runs();
        if !self.show_backtest {
            return;
        }
        let mut show_backtest = self.show_backtest;
        let mut workflow_commands = Vec::new();
        egui::Window::new("Backtest Engine")
            .open(&mut show_backtest)
            .resizable(true)
            .default_size([600.0, 500.0])
            .show(ctx, |ui| {
                ui.heading("Strategy Backtest");
                ui.separator();
                let chart = self.charts.get(self.active_tab);
                let n_bars = chart.map(|c| c.bars.len()).unwrap_or(0);
                let tf = chart.map(|c| c.timeframe.label()).unwrap_or("—");
                ui.horizontal(|ui| {
                    ui.label("Symbol:");
                    ui.label(egui::RichText::new(&self.symbol_input).strong());
                    ui.label("TF:");
                    ui.label(egui::RichText::new(tf).strong());
                    ui.label("Bars:");
                    ui.label(egui::RichText::new(format!("{}", n_bars)).strong());
                });
                ui.horizontal(|ui| {
                    let loading = self.strategy_result_load_rx.is_some();
                    if ui
                        .add_enabled(
                            !loading && n_bars > 0,
                            egui::Button::new("Load verified report pair…"),
                        )
                        .on_hover_text(
                            "Select one sealed StrategyReport artifact and its paired SimulationReport JSON",
                        )
                        .clicked()
                    {
                        self.start_strategy_result_load(ctx);
                    }
                    if loading {
                        ui.spinner();
                    }
                    if let Some(chart_tab) = self.strategy_result_chart_tab
                        && chart_tab != self.active_tab
                        && ui.button("Focus report chart").clicked()
                    {
                        self.active_tab = chart_tab;
                    }
                });
                if !self.strategy_result_status.is_empty() {
                    let color = if self.strategy_result_status.starts_with("Error:") {
                        DOWN
                    } else if self.strategy_result_load_rx.is_some() {
                        egui::Color32::from_rgb(255, 200, 50)
                    } else {
                        UP
                    };
                    ui.label(
                        egui::RichText::new(&self.strategy_result_status)
                            .color(color)
                            .small(),
                    );
                }
                ui.add_space(5.0);
                ui.collapsing("Verified identity-bound sub-bar run", |ui| {
                    ui.small("Requires existing sealed Strategy IR, execution-config (SubBar fidelity), and run-manifest JSON. No artifacts are generated here.");
                    let records = &self.dataset_inspector.records;
                    egui::ComboBox::from_label("Parent dataset")
                        .selected_text(if self.sub_bar_run_ui.parent_dataset_id.is_empty() { "Select from Dataset Inspector" } else { &self.sub_bar_run_ui.parent_dataset_id })
                        .show_ui(ui, |ui| for record in records { ui.selectable_value(&mut self.sub_bar_run_ui.parent_dataset_id, record.dataset_id.clone(), format!("{} {} · {}", record.symbol, record.timeframe, &record.dataset_id[..12])); });
                    egui::ComboBox::from_label("Finer dataset")
                        .selected_text(if self.sub_bar_run_ui.finer_dataset_id.is_empty() { "Select finer sealed dataset" } else { &self.sub_bar_run_ui.finer_dataset_id })
                        .show_ui(ui, |ui| for record in records { ui.selectable_value(&mut self.sub_bar_run_ui.finer_dataset_id, record.dataset_id.clone(), format!("{} {} · {}", record.symbol, record.timeframe, &record.dataset_id[..12])); });
                    for (label, path) in [("Strategy JSON", &mut self.sub_bar_run_ui.strategy_path), ("Execution config JSON", &mut self.sub_bar_run_ui.config_path), ("Run manifest JSON", &mut self.sub_bar_run_ui.manifest_path)] {
                        ui.horizontal(|ui| { ui.label(label); ui.add(egui::TextEdit::singleline(path).char_limit(4096).desired_width(330.0)); });
                    }
                    ui.horizontal(|ui| {
                        if ui.add_enabled(!self.sub_bar_run_state.is_busy(), egui::Button::new("Run verified sub-bar")).clicked() { self.submit_sub_bar_run(); }
                        if ui.add_enabled(self.sub_bar_run_state.is_busy(), egui::Button::new("Cancel")).clicked() {
                            let generation = self.sub_bar_run_state.cancel();
                            if let Some(worker) = &self.strategy_run_worker { worker.supersede_with(generation); }
                        }
                        if self.sub_bar_run_state.is_busy() { ui.spinner(); }
                    });
                    if !self.sub_bar_run_state.status.is_empty() { ui.small(&self.sub_bar_run_state.status); }
                });
                ui.add_space(5.0);
                ui.collapsing("Manifest-bound candidate intervention replay", |ui| {
                    ui.small("Select every sealed parent dataset bound by the run manifest plus the sealed Strategy IR, execution config, run manifest, and candidate InterventionLog JSON. Artifacts are loaded and replayed only on the bounded worker; the current report is preserved unless exact replay and report verification succeed.");
                    let dataset_rows: Vec<_> = self
                        .dataset_inspector
                        .records
                        .iter()
                        .map(|record| {
                            (
                                record.dataset_id.clone(),
                                format!(
                                    "{} {} · {}",
                                    record.symbol,
                                    record.timeframe,
                                    &record.dataset_id[..12]
                                ),
                            )
                        })
                        .collect();
                    ui.label("Bound parent datasets");
                    egui::ScrollArea::vertical()
                        .id_salt("intervention_parent_datasets")
                        .max_height(120.0)
                        .show(ui, |ui| {
                            for (dataset_id, label) in dataset_rows {
                                let mut selected = self
                                    .intervention_run_ui
                                    .selected_dataset_ids
                                    .contains(&dataset_id);
                                if ui.checkbox(&mut selected, label).changed() {
                                    if selected {
                                        self.intervention_run_ui
                                            .selected_dataset_ids
                                            .insert(dataset_id);
                                    } else {
                                        self.intervention_run_ui
                                            .selected_dataset_ids
                                            .remove(&dataset_id);
                                    }
                                }
                            }
                        });
                    for (label, path) in [
                        ("Strategy JSON", &mut self.intervention_run_ui.strategy_path),
                        (
                            "Execution config JSON",
                            &mut self.intervention_run_ui.config_path,
                        ),
                        (
                            "Run manifest JSON",
                            &mut self.intervention_run_ui.manifest_path,
                        ),
                        (
                            "Candidate InterventionLog JSON",
                            &mut self.intervention_run_ui.intervention_log_path,
                        ),
                    ] {
                        ui.horizontal(|ui| {
                            ui.label(label);
                            ui.add(
                                egui::TextEdit::singleline(path)
                                    .char_limit(4096)
                                    .desired_width(330.0),
                            );
                        });
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !self.intervention_run_state.is_busy(),
                                egui::Button::new("Replay and promote candidate"),
                            )
                            .clicked()
                        {
                            self.submit_intervention_run();
                        }
                        if ui
                            .add_enabled(
                                self.intervention_run_state.is_busy(),
                                egui::Button::new("Cancel replay"),
                            )
                            .clicked()
                        {
                            let generation = self.intervention_run_state.cancel();
                            if let Some(worker) = &self.intervention_run_worker {
                                worker.supersede_with(generation);
                            }
                        }
                        if self.intervention_run_state.is_busy() {
                            ui.spinner();
                        }
                    });
                    if !self.intervention_run_state.status.is_empty() {
                        let color = if self.intervention_run_state.status.starts_with("Error:") {
                            DOWN
                        } else {
                            UP
                        };
                        ui.label(
                            egui::RichText::new(&self.intervention_run_state.status)
                                .color(color)
                                .small(),
                        );
                    }
                    if let Some(installed) = &self.intervention_run_state.installed {
                        egui::Grid::new("intervention_promoted_identities")
                            .num_columns(2)
                            .show(ui, |ui| {
                                for (label, identity) in [
                                    ("Run", &installed.run_id),
                                    ("Intervention log", &installed.log_id),
                                    ("Report", &installed.report_id),
                                ] {
                                    ui.label(label);
                                    ui.monospace(identity);
                                    ui.end_row();
                                }
                            });
                    }
                });
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label("Strategy:");
                    ui.radio_value(&mut self.bt_strategy, 0, "SMA Cross");
                    ui.radio_value(&mut self.bt_strategy, 1, "NNFX");
                    ui.radio_value(&mut self.bt_strategy, 2, "KAMA Cross");
                    ui.radio_value(&mut self.bt_strategy, 3, "Fisher Cross");
                    ui.radio_value(&mut self.bt_strategy, 4, "RSI Mean-Rev");
                });
                ui.horizontal(|ui| {
                    ui.label("Fast Period:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.bt_fast_period).desired_width(50.0),
                    );
                    ui.label("Slow Period:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.bt_slow_period).desired_width(50.0),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Initial Equity:");
                    ui.add(egui::TextEdit::singleline(&mut self.bt_equity).desired_width(80.0));
                });

                ui.add_space(5.0);
                if ui.button("Run Backtest").clicked() && n_bars > 0 {
                    if let Some(chart) = self.charts.get(self.active_tab) {
                        let engine_bars: Vec<EngineBar> = chart
                            .bars
                            .iter()
                            .map(|b| EngineBar {
                                timestamp: format_ts(b.ts_ms, chart.timeframe),
                                open: b.open,
                                high: b.high,
                                low: b.low,
                                close: b.close,
                                volume: b.volume,
                            })
                            .collect();
                        let fast: usize = self.bt_fast_period.parse().unwrap_or(10);
                        let slow: usize = self.bt_slow_period.parse().unwrap_or(50);
                        let equity: f64 = self
                            .bt_equity
                            .replace(['$', ','], "")
                            .parse()
                            .unwrap_or(10000.0);

                        let result = match self.bt_strategy {
                            0 => {
                                let mut strat = backtest::SMACrossStrategy::new(fast, slow);
                                backtest::run_backtest(&engine_bars, &mut strat, equity)
                            }
                            1 => {
                                let mut strat = backtest::NNFXStrategy::new(fast, slow);
                                backtest::run_backtest(&engine_bars, &mut strat, equity)
                            }
                            2 => {
                                let mut strat = backtest::KAMACrossStrategy::new(fast, 2, 30);
                                backtest::run_backtest(&engine_bars, &mut strat, equity)
                            }
                            3 => {
                                let mut strat = backtest::FisherCrossStrategy::new(fast.max(10));
                                backtest::run_backtest(&engine_bars, &mut strat, equity)
                            }
                            4 => {
                                let mut strat =
                                    backtest::RSIMeanRevStrategy::new(fast.max(5), 30.0, 70.0);
                                backtest::run_backtest(&engine_bars, &mut strat, equity)
                            }
                            _ => {
                                let mut strat = backtest::SMACrossStrategy::new(fast, slow);
                                backtest::run_backtest(&engine_bars, &mut strat, equity)
                            }
                        };
                        self.bt_result = Some(result.report);
                        self.bt_trades = result.trades;
                        self.bt_equity_curve = result.equity_curve;
                        self.log.push_back(LogEntry::info(format!(
                            "Backtest complete: {} trades, PF={:.2}, WR={:.1}%",
                            self.bt_trades.len(),
                            self.bt_result
                                .as_ref()
                                .map(|r| r.profit_factor)
                                .unwrap_or(0.0),
                            self.bt_result.as_ref().map(|r| r.win_rate).unwrap_or(0.0),
                        )));
                    }
                }

                if let Some(ref report) = self.bt_result {
                    ui.add_space(10.0);
                    ui.heading("Results");
                    ui.separator();
                    egui::Grid::new("bt_report")
                        .striped(true)
                        .num_columns(4)
                        .show(ui, |ui| {
                            ui.label("Trades:");
                            ui.label(format!("{}", report.total_trades));
                            ui.label("Win Rate:");
                            {
                                let wr_c = if report.win_rate >= 50.0 {
                                    UP
                                } else if report.win_rate >= 40.0 {
                                    egui::Color32::from_rgb(255, 200, 50)
                                } else {
                                    DOWN
                                };
                                ui.label(
                                    egui::RichText::new(format!("{:.1}%", report.win_rate))
                                        .color(wr_c),
                                );
                            }
                            ui.end_row();
                            ui.label("Profit Factor:");
                            ui.label(format!("{:.2}", report.profit_factor));
                            ui.label("Sharpe:");
                            ui.label(format!("{:.3}", report.sharpe_ratio));
                            ui.end_row();
                            let pnl_c = if report.total_pnl >= 0.0 { UP } else { DOWN };
                            ui.label("Total P&L:");
                            ui.label(
                                egui::RichText::new(format!("${:.2}", report.total_pnl))
                                    .color(pnl_c),
                            );
                            ui.label("Max DD:");
                            ui.label(
                                egui::RichText::new(format!("{:.2}%", report.max_drawdown_pct))
                                    .color(DOWN),
                            );
                            ui.end_row();
                            ui.label("Avg Win:");
                            ui.label(format!("${:.2}", report.avg_win));
                            ui.label("Avg Loss:");
                            ui.label(format!("${:.2}", report.avg_loss));
                            ui.end_row();
                            ui.label("Max Win Streak:");
                            ui.label(format!("{}", report.max_consecutive_wins));
                            ui.label("Max Loss Streak:");
                            ui.label(format!("{}", report.max_consecutive_losses));
                            ui.end_row();
                        });

                    if self.bt_equity_curve.len() > 2 {
                        ui.add_space(10.0);
                        ui.heading("Equity Curve");
                        let points: PlotPoints = PlotPoints::new(
                            self.bt_equity_curve
                                .iter()
                                .enumerate()
                                .map(|(i, &v)| [i as f64, v])
                                .collect(),
                        );
                        let line = Line::new("Equity", points).color(ACCENT);
                        Plot::new("bt_equity_plot")
                            .height(150.0)
                            .allow_drag(false)
                            .allow_zoom(false)
                            .show(ui, |plot_ui| {
                                plot_ui.line(line);
                            });
                    }

                    if !self.bt_trades.is_empty() {
                        ui.add_space(10.0);
                        ui.collapsing(format!("Trade List ({})", self.bt_trades.len()), |ui| {
                            egui::ScrollArea::vertical()
                                .auto_shrink(false)
                                .max_height(200.0)
                                .show(ui, |ui| {
                                    egui::Grid::new("bt_trades")
                                        .striped(true)
                                        .num_columns(5)
                                        .show(ui, |ui| {
                                            ui.strong("#");
                                            ui.strong("Side");
                                            ui.strong("Entry");
                                            ui.strong("Exit");
                                            ui.strong("P&L");
                                            ui.end_row();
                                            for (i, t) in self.bt_trades.iter().enumerate() {
                                                ui.label(format!("{}", i + 1));
                                                ui.label(&t.side);
                                                ui.label(format_price(t.entry_price));
                                                ui.label(format_price(t.exit_price));
                                                let c = if t.pnl >= 0.0 { UP } else { DOWN };
                                                ui.label(
                                                    egui::RichText::new(format!("{:.2}", t.pnl))
                                                        .color(c),
                                                );
                                                ui.end_row();
                                            }
                                        });
                                });
                        });
                    }
                }

                if let Some(view) = self.strategy_result_view.as_ref() {
                    let report_chart_is_active =
                        self.strategy_result_chart_tab == Some(self.active_tab);
                    if report_chart_is_active {
                        workflow_commands.extend(strategy_report_view::render_prepared_result(
                            ui,
                            view,
                            &mut self.strategy_result_workflow,
                            &mut self.strategy_result_selected_trade,
                            &mut self.replay_active,
                            &mut self.replay_bar_idx,
                            &mut self.replay_playing,
                            &mut self.replay_speed,
                        ));
                    } else {
                        ui.separator();
                        ui.label(
                            egui::RichText::new(
                                "The verified result remains installed on its source chart; focus that chart to inspect or replay it.",
                            )
                            .color(egui::Color32::from_rgb(255, 200, 50)),
                        );
                    }
                }
            });
        self.show_backtest = show_backtest;
        for command in workflow_commands {
            self.start_strategy_workflow_command(ctx, command);
        }
    }

    pub(super) fn render_optimizer_window(&mut self, ctx: &egui::Context) {
        if !self.show_optimizer {
            return;
        }
        let mut show_optimizer = self.show_optimizer;
        egui::Window::new("Optimizer")
            .open(&mut show_optimizer)
            .resizable(true)
            .default_size([750.0, 600.0])
            .show(ctx, |ui| {
                let opt_green = egui::Color32::from_rgb(46, 204, 113);
                let opt_red = egui::Color32::from_rgb(231, 76, 60);
                let opt_gold = egui::Color32::from_rgb(241, 196, 15);
                let opt_cyan = egui::Color32::from_rgb(26, 188, 156);
                let opt_dim = egui::Color32::from_rgb(100, 100, 120);

                let gpu_available = self.gpu_backtester.is_some();
                ui.horizontal(|ui| {
                    ui.heading("Strategy Optimizer");
                    if gpu_available {
                        ui.label(egui::RichText::new("GPU").color(opt_green).strong());
                    } else {
                        ui.label(egui::RichText::new("CPU").color(opt_gold));
                    }
                });
                ui.separator();

                let chart = self.charts.get(self.active_tab);
                let n_bars = chart.map(|c| c.bars.len()).unwrap_or(0);
                ui.label(
                    egui::RichText::new(format!(
                        "Symbol: {}  |  Bars: {}  |  {}",
                        self.symbol_input,
                        n_bars,
                        chart.map(|c| c.timeframe.label()).unwrap_or("?")
                    ))
                    .color(opt_cyan),
                );

                ui.add_space(4.0);
                ui.label(egui::RichText::new("Parameter Ranges").strong());
                egui::Grid::new("opt_params")
                    .num_columns(4)
                    .show(ui, |ui| {
                        ui.label("SMA Fast:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.opt_fast_range)
                                .desired_width(60.0),
                        );
                        ui.label("SMA Slow:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.opt_slow_range)
                                .desired_width(60.0),
                        );
                        ui.end_row();
                        ui.label("RSI Period:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.opt_rsi_range).desired_width(60.0),
                        );
                        ui.label("ATR SL Mult:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.opt_atr_sl_range)
                                .desired_width(60.0),
                        );
                        ui.end_row();
                        ui.label("ATR TP Mult:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.opt_atr_tp_range)
                                .desired_width(60.0),
                        );
                        ui.label("");
                        ui.label("");
                        ui.end_row();
                    });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if gpu_available
                        && ui
                            .button(
                                egui::RichText::new("Run GPU Optimization")
                                    .color(opt_green)
                                    .strong(),
                            )
                            .clicked()
                        && n_bars > 50
                    {
                        if let Some(chart) = self.charts.get(self.active_tab) {
                            let closes: Vec<f32> =
                                chart.bars.iter().map(|b| b.close as f32).collect();
                            let highs: Vec<f32> =
                                chart.bars.iter().map(|b| b.high as f32).collect();
                            let lows: Vec<f32> =
                                chart.bars.iter().map(|b| b.low as f32).collect();

                            let fast = parse_range(&self.opt_fast_range, 5, 50);
                            let slow = parse_range(&self.opt_slow_range, 20, 200);
                            let rsi_r = parse_range(&self.opt_rsi_range, 10, 20);
                            let atr_sl = parse_range_f32(&self.opt_atr_sl_range, 1.0, 3.0);
                            let atr_tp = parse_range_f32(&self.opt_atr_tp_range, 2.0, 5.0);

                            let mut combos = Vec::new();
                            let fast_step = ((fast.1 - fast.0) / 10).max(1);
                            let slow_step = ((slow.1 - slow.0) / 10).max(1);
                            let rsi_step = ((rsi_r.1 - rsi_r.0) / 5).max(1);
                            let atr_sl_step = (atr_sl.1 - atr_sl.0) / 5.0;
                            let atr_tp_step = (atr_tp.1 - atr_tp.0) / 5.0;

                            let mut f = fast.0;
                            while f <= fast.1 {
                                let mut s = slow.0;
                                while s <= slow.1 {
                                    if s > f {
                                        let mut r = rsi_r.0;
                                        while r <= rsi_r.1 {
                                            let mut sl_m = atr_sl.0;
                                            while sl_m <= atr_sl.1 + 0.001 {
                                                let mut tp_m = atr_tp.0;
                                                while tp_m <= atr_tp.1 + 0.001 {
                                                    combos.push(gpu_compute::ParamCombo {
                                                        sma_fast: f as u32,
                                                        sma_slow: s as u32,
                                                        rsi_period: r as u32,
                                                        rsi_overbought: 70.0,
                                                        rsi_oversold: 30.0,
                                                        atr_period: 14,
                                                        atr_sl_mult: sl_m as f32,
                                                        atr_tp_mult: tp_m as f32,
                                                    });
                                                    tp_m += atr_tp_step;
                                                }
                                                sl_m += atr_sl_step;
                                            }
                                            r += rsi_step;
                                        }
                                    }
                                    s += slow_step;
                                }
                                f += fast_step;
                            }

                            let combo_count = combos.len();
                            self.gpu_opt_combos = combos.clone();

                            if let Some(ref mut bt) = self.gpu_backtester {
                                let t = std::time::Instant::now();
                                bt.upload(&closes, &highs, &lows, &combos);
                                if let Some(results) = bt.evaluate() {
                                    let elapsed = t.elapsed();
                                    self.gpu_opt_results = results;
                                    let mut indexed: Vec<(usize, &gpu_compute::BacktestResult)> =
                                        self.gpu_opt_results.iter().enumerate().collect();
                                    indexed.sort_by(|a, b| {
                                        b.1.sharpe
                                            .partial_cmp(&a.1.sharpe)
                                            .unwrap_or(std::cmp::Ordering::Equal)
                                    });
                                    let sorted_results: Vec<gpu_compute::BacktestResult> = indexed
                                        .iter()
                                        .map(|(i, _)| self.gpu_opt_results[*i].clone())
                                        .collect();
                                    let sorted_combos: Vec<gpu_compute::ParamCombo> = indexed
                                        .iter()
                                        .map(|(i, _)| self.gpu_opt_combos[*i].clone())
                                        .collect();
                                    self.gpu_opt_results = sorted_results;
                                    self.gpu_opt_combos = sorted_combos;
                                    self.log.push_back(LogEntry::info(format!(
                                        "GPU Optimizer: {} combos tested in {:.1}ms ({:.0} combos/sec)",
                                        combo_count,
                                        elapsed.as_secs_f64() * 1000.0,
                                        combo_count as f64 / elapsed.as_secs_f64()
                                    )));
                                }
                            }
                        }
                    }
                    if gpu_available
                        && ui
                            .button(
                                egui::RichText::new("Run NNFX Optimizer")
                                    .color(egui::Color32::from_rgb(155, 89, 182))
                                    .strong(),
                            )
                            .clicked()
                        && n_bars > 50
                    {
                        if let Some(chart) = self.charts.get(self.active_tab) {
                            let closes: Vec<f32> =
                                chart.bars.iter().map(|b| b.close as f32).collect();
                            let highs: Vec<f32> =
                                chart.bars.iter().map(|b| b.high as f32).collect();
                            let lows: Vec<f32> =
                                chart.bars.iter().map(|b| b.low as f32).collect();

                            let mut nnfx_combos = Vec::new();
                            for kama_p in (5..=20).step_by(3) {
                                for fisher_p in (10..=40).step_by(5) {
                                    for adx_thresh in [20.0_f32, 25.0, 30.0] {
                                        for sl_mult in [1.0_f32, 1.5, 2.0, 2.5] {
                                            for tp_mult in [1.5_f32, 2.0, 3.0, 4.0] {
                                                nnfx_combos.push(gpu_compute::NnfxParamCombo {
                                                    kama_period: kama_p,
                                                    fisher_period: fisher_p,
                                                    atr_period: 14,
                                                    adx_period: 14,
                                                    adx_threshold: adx_thresh,
                                                    atr_sl_mult: sl_mult,
                                                    atr_tp_mult: tp_mult,
                                                });
                                            }
                                        }
                                    }
                                }
                            }

                            let combo_count = nnfx_combos.len();
                            if let Some(ref mut bt) = self.gpu_backtester {
                                let t = std::time::Instant::now();
                                if let Some(results) =
                                    bt.evaluate_nnfx(&closes, &highs, &lows, &nnfx_combos)
                                {
                                    let elapsed = t.elapsed();
                                    let mut indexed: Vec<(usize, f32)> = results
                                        .iter()
                                        .enumerate()
                                        .map(|(i, r)| (i, r.sharpe))
                                        .collect();
                                    indexed.sort_by(|a, b| {
                                        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
                                    });

                                    self.gpu_opt_results = indexed
                                        .iter()
                                        .map(|(i, _)| results[*i].clone())
                                        .collect();
                                    self.gpu_opt_combos = indexed
                                        .iter()
                                        .map(|(i, _)| {
                                            let nc = &nnfx_combos[*i];
                                            gpu_compute::ParamCombo {
                                                sma_fast: nc.kama_period,
                                                sma_slow: nc.fisher_period,
                                                rsi_period: nc.adx_period,
                                                rsi_overbought: nc.adx_threshold,
                                                rsi_oversold: 0.0,
                                                atr_period: nc.atr_period,
                                                atr_sl_mult: nc.atr_sl_mult,
                                                atr_tp_mult: nc.atr_tp_mult,
                                            }
                                        })
                                        .collect();

                                    self.log.push_back(LogEntry::info(format!(
                                        "NNFX Optimizer: {} combos tested in {:.1}ms ({:.0}/sec) — Fisher+KAMA+ATR+ADX",
                                        combo_count,
                                        elapsed.as_secs_f64() * 1000.0,
                                        combo_count as f64 / elapsed.as_secs_f64()
                                    )));
                                }
                            }
                        }
                    }
                    if ui.button("Run CPU Optimization").clicked() && n_bars > 50 {
                        if let Some(chart) = self.charts.get(self.active_tab) {
                            let engine_bars: Vec<EngineBar> = chart
                                .bars
                                .iter()
                                .map(|b| EngineBar {
                                    timestamp: format_ts(b.ts_ms, chart.timeframe),
                                    open: b.open,
                                    high: b.high,
                                    low: b.low,
                                    close: b.close,
                                    volume: b.volume,
                                })
                                .collect();
                            let fast: (usize, usize) = parse_range(&self.opt_fast_range, 5, 50);
                            let slow: (usize, usize) = parse_range(&self.opt_slow_range, 20, 200);
                            let report =
                                backtest::optimize_sma_cross(&engine_bars, fast, slow, 10000.0, 20);
                            self.opt_results = report.results;
                            self.log.push_back(LogEntry::info(format!(
                                "CPU Optimizer: {} combinations tested",
                                report.total_combinations
                            )));
                        }
                    }
                });

                if !self.gpu_opt_results.is_empty() {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "GPU Results — Top {} of {}",
                                self.gpu_opt_results.len().min(50),
                                self.gpu_opt_results.len()
                            ))
                            .strong()
                            .color(opt_green),
                        );
                    });

                    let bars: Vec<PlotBar> = self
                        .gpu_opt_results
                        .iter()
                        .take(50)
                        .enumerate()
                        .map(|(i, r)| {
                            let c = if r.net_pnl >= 0.0 {
                                opt_green
                            } else {
                                opt_red
                            };
                            PlotBar::new(i as f64, r.net_pnl as f64).width(0.8).fill(c)
                        })
                        .collect();
                    if !bars.is_empty() {
                        let chart = BarChart::new("P&L by Combo", bars);
                        Plot::new("gpu_opt_pnl")
                            .height(100.0)
                            .allow_drag(false)
                            .allow_zoom(false)
                            .allow_scroll(false)
                            .show_axes([false, true])
                            .show(ui, |plot_ui| {
                                plot_ui.bar_chart(chart);
                            });
                    }

                    if self.gpu_opt_results.len() > 4
                        && self.gpu_opt_combos.len() == self.gpu_opt_results.len()
                    {
                        ui.label(
                            egui::RichText::new("Parameter Heatmap (Fast × Slow → Sharpe)")
                                .small()
                                .strong(),
                        );
                        let mut fast_set: Vec<u32> =
                            self.gpu_opt_combos.iter().map(|c| c.sma_fast).collect();
                        fast_set.sort();
                        fast_set.dedup();
                        let mut slow_set: Vec<u32> =
                            self.gpu_opt_combos.iter().map(|c| c.sma_slow).collect();
                        slow_set.sort();
                        slow_set.dedup();

                        if fast_set.len() > 1 && slow_set.len() > 1 {
                            let cols = fast_set.len();
                            let rows = slow_set.len();
                            let avail_w = ui.available_width().min(500.0);
                            let h = (rows as f32 * 14.0).min(200.0);
                            let cell_w = avail_w / cols as f32;
                            let cell_h = h / rows as f32;
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(avail_w, h),
                                egui::Sense::hover(),
                            );
                            let painter = ui.painter_at(rect);
                            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(10, 10, 20));

                            let max_sharpe = self
                                .gpu_opt_results
                                .iter()
                                .map(|r| r.sharpe)
                                .fold(0.0_f32, f32::max)
                                .max(0.01);
                            let min_sharpe = self
                                .gpu_opt_results
                                .iter()
                                .map(|r| r.sharpe)
                                .fold(f32::MAX, f32::min);

                            let fast_idx: std::collections::HashMap<u32, usize> = fast_set
                                .iter()
                                .enumerate()
                                .map(|(i, &f)| (f, i))
                                .collect();
                            let slow_idx: std::collections::HashMap<u32, usize> = slow_set
                                .iter()
                                .enumerate()
                                .map(|(i, &s)| (s, i))
                                .collect();
                            for (combo, result) in
                                self.gpu_opt_combos.iter().zip(self.gpu_opt_results.iter())
                            {
                                let col = fast_idx.get(&combo.sma_fast).copied().unwrap_or(0);
                                let row = slow_idx.get(&combo.sma_slow).copied().unwrap_or(0);
                                let x = rect.left() + col as f32 * cell_w;
                                let y = rect.top() + row as f32 * cell_h;

                                let norm = if max_sharpe > min_sharpe {
                                    (result.sharpe - min_sharpe) / (max_sharpe - min_sharpe)
                                } else {
                                    0.5
                                };
                                let color = if result.sharpe > 0.0 {
                                    egui::Color32::from_rgb(
                                        0,
                                        (norm * 200.0) as u8,
                                        (norm * 60.0) as u8,
                                    )
                                } else {
                                    egui::Color32::from_rgb(
                                        ((1.0 - norm) * 200.0) as u8,
                                        0,
                                        0,
                                    )
                                };
                                painter.rect_filled(
                                    egui::Rect::from_min_size(
                                        egui::pos2(x, y),
                                        egui::vec2(cell_w - 1.0, cell_h - 1.0),
                                    ),
                                    0.0,
                                    color,
                                );
                            }

                            for (i, &f) in fast_set.iter().enumerate() {
                                let x = rect.left() + i as f32 * cell_w + cell_w / 2.0;
                                painter.text(
                                    egui::pos2(x, rect.bottom() + 2.0),
                                    egui::Align2::CENTER_TOP,
                                    format!("{}", f),
                                    egui::FontId::monospace(8.0),
                                    opt_dim,
                                );
                            }
                            for (i, &s) in slow_set.iter().enumerate() {
                                let y = rect.top() + i as f32 * cell_h + cell_h / 2.0;
                                painter.text(
                                    egui::pos2(rect.left() - 2.0, y),
                                    egui::Align2::RIGHT_CENTER,
                                    format!("{}", s),
                                    egui::FontId::monospace(8.0),
                                    opt_dim,
                                );
                            }
                            ui.add_space(14.0);
                        }
                    }

                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(350.0)
                        .show(ui, |ui| {
                            egui::Grid::new("gpu_opt_grid")
                                .striped(true)
                                .num_columns(11)
                                .min_col_width(45.0)
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("Fast").color(opt_dim).small());
                                    ui.label(egui::RichText::new("Slow").color(opt_dim).small());
                                    ui.label(egui::RichText::new("RSI").color(opt_dim).small());
                                    ui.label(egui::RichText::new("SL×").color(opt_dim).small());
                                    ui.label(egui::RichText::new("TP×").color(opt_dim).small());
                                    ui.label(egui::RichText::new("P&L").color(opt_dim).small());
                                    ui.label(egui::RichText::new("DD%").color(opt_dim).small());
                                    ui.label(
                                        egui::RichText::new("Sharpe").color(opt_dim).small(),
                                    );
                                    ui.label(egui::RichText::new("Win%").color(opt_dim).small());
                                    ui.label(
                                        egui::RichText::new("Trades").color(opt_dim).small(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Robust").color(opt_dim).small(),
                                    );
                                    ui.end_row();

                                    for (i, r) in self.gpu_opt_results.iter().take(50).enumerate()
                                    {
                                        let combo = &self.gpu_opt_combos[i.min(
                                            self.gpu_opt_combos.len().saturating_sub(1),
                                        )];
                                        ui.label(format!("{}", combo.sma_fast));
                                        ui.label(format!("{}", combo.sma_slow));
                                        ui.label(format!("{}", combo.rsi_period));
                                        ui.label(format!("{:.1}", combo.atr_sl_mult));
                                        ui.label(format!("{:.1}", combo.atr_tp_mult));
                                        let pc = if r.net_pnl >= 0.0 {
                                            opt_green
                                        } else {
                                            opt_red
                                        };
                                        ui.label(
                                            egui::RichText::new(format!("${:.0}", r.net_pnl))
                                                .color(pc),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:.1}%",
                                                r.max_drawdown * 100.0
                                            ))
                                            .color(opt_red),
                                        );
                                        let sc = if r.sharpe > 1.0 {
                                            opt_green
                                        } else if r.sharpe > 0.0 {
                                            opt_gold
                                        } else {
                                            opt_red
                                        };
                                        ui.label(
                                            egui::RichText::new(format!("{:.2}", r.sharpe))
                                                .color(sc),
                                        );
                                        let wc = if r.win_rate > 50.0 {
                                            opt_green
                                        } else {
                                            opt_red
                                        };
                                        ui.label(
                                            egui::RichText::new(format!("{:.0}%", r.win_rate))
                                                .color(wc),
                                        );
                                        ui.label(format!("{}", r.trade_count));
                                        let rc = if r.robustness_score > 0.7 {
                                            opt_green
                                        } else if r.robustness_score > 0.3 {
                                            opt_gold
                                        } else {
                                            opt_red
                                        };
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:.2}",
                                                r.robustness_score
                                            ))
                                            .color(rc),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                }

                if !self.opt_results.is_empty() && self.gpu_opt_results.is_empty() {
                    ui.add_space(10.0);
                    ui.heading(format!("CPU Results — Top {}", self.opt_results.len()));
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(300.0)
                        .show(ui, |ui| {
                            egui::Grid::new("opt_grid")
                                .striped(true)
                                .num_columns(6)
                                .show(ui, |ui| {
                                    ui.strong("Fast");
                                    ui.strong("Slow");
                                    ui.strong("Trades");
                                    ui.strong("PF");
                                    ui.strong("Sharpe");
                                    ui.strong("P&L");
                                    ui.end_row();
                                    for r in &self.opt_results {
                                        ui.label(format!("{}", r.fast_period));
                                        ui.label(format!("{}", r.slow_period));
                                        ui.label(format!("{}", r.total_trades));
                                        ui.label(format!("{:.2}", r.profit_factor));
                                        ui.label(format!("{:.3}", r.sharpe_ratio));
                                        let c = if r.total_pnl >= 0.0 { UP } else { DOWN };
                                        ui.label(
                                            egui::RichText::new(format!("${:.0}", r.total_pnl))
                                                .color(c),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                }

                ui.add_space(8.0);
                ui.heading("Walk-Forward Analysis");
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Windows:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.wf_windows_count).desired_width(40.0),
                    );
                    if ui.button("Run Walk-Forward").clicked() && n_bars > 200 {
                        if let Some(chart) = self.charts.get(self.active_tab) {
                            let engine_bars: Vec<EngineBar> = chart
                                .bars
                                .iter()
                                .map(|b| EngineBar {
                                    timestamp: format_ts(b.ts_ms, chart.timeframe),
                                    open: b.open,
                                    high: b.high,
                                    low: b.low,
                                    close: b.close,
                                    volume: b.volume,
                                })
                                .collect();
                            let fast_r: (usize, usize) = parse_range(&self.opt_fast_range, 5, 50);
                            let slow_r: (usize, usize) =
                                parse_range(&self.opt_slow_range, 20, 200);
                            let equity: f64 = self
                                .bt_equity
                                .replace(['$', ','], "")
                                .parse()
                                .unwrap_or(10000.0);
                            let windows: usize = self.wf_windows_count.parse().unwrap_or(5);
                            self.wf_result = Some(backtest::walk_forward(
                                &engine_bars,
                                fast_r.0..fast_r.1,
                                slow_r.0..slow_r.1,
                                windows,
                                equity,
                            ));
                            self.log.push_back(LogEntry::info(format!(
                                "Walk-forward complete: {} windows",
                                windows
                            )));
                        }
                    }
                });

                if let Some(ref wf) = self.wf_result {
                    ui.add_space(4.0);
                    let rob_c = if wf.robustness_score > 0.5 {
                        UP
                    } else if wf.robustness_score > 0.25 {
                        egui::Color32::from_rgb(241, 196, 15)
                    } else {
                        DOWN
                    };
                    egui::Grid::new("wf_summary")
                        .striped(true)
                        .num_columns(4)
                        .show(ui, |ui| {
                            ui.label("OOS Sharpe:");
                            ui.label(format!("{:.3}", wf.oos_sharpe));
                            ui.label("OOS PF:");
                            ui.label(format!("{:.2}", wf.oos_profit_factor));
                            ui.end_row();
                            ui.label("OOS Win%:");
                            ui.label(format!("{:.1}%", wf.oos_win_rate * 100.0));
                            ui.label("Robustness:");
                            ui.label(
                                egui::RichText::new(format!("{:.2}", wf.robustness_score))
                                    .color(rob_c),
                            );
                            ui.end_row();
                            ui.label("Best Params:");
                            ui.label(format!("Fast={} Slow={}", wf.best_params.0, wf.best_params.1));
                            ui.label("");
                            ui.label("");
                            ui.end_row();
                        });
                    if !wf.windows.is_empty() {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Per-Window Results").small().strong());
                        egui::Grid::new("wf_windows")
                            .striped(true)
                            .num_columns(6)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("#").color(AXIS_TEXT).small());
                                ui.label(
                                    egui::RichText::new("Fast/Slow").color(AXIS_TEXT).small(),
                                );
                                ui.label(
                                    egui::RichText::new("IS Sharpe").color(AXIS_TEXT).small(),
                                );
                                ui.label(
                                    egui::RichText::new("OOS Sharpe").color(AXIS_TEXT).small(),
                                );
                                ui.label(
                                    egui::RichText::new("OOS P&L").color(AXIS_TEXT).small(),
                                );
                                ui.label(egui::RichText::new("Trades").color(AXIS_TEXT).small());
                                ui.end_row();
                                for w in &wf.windows {
                                    ui.label(format!("{}", w.window_idx + 1));
                                    ui.label(format!("{}/{}", w.best_fast, w.best_slow));
                                    ui.label(format!("{:.3}", w.is_sharpe));
                                    let oos_c = if w.oos_sharpe > 0.0 { UP } else { DOWN };
                                    ui.label(
                                        egui::RichText::new(format!("{:.3}", w.oos_sharpe))
                                            .color(oos_c),
                                    );
                                    let pnl_c = if w.oos_pnl >= 0.0 { UP } else { DOWN };
                                    ui.label(
                                        egui::RichText::new(format!("${:.0}", w.oos_pnl))
                                            .color(pnl_c),
                                    );
                                    ui.label(format!("{}", w.oos_trades));
                                    ui.end_row();
                                }
                            });
                    }
                }
            });
        self.show_optimizer = show_optimizer;
    }
}
