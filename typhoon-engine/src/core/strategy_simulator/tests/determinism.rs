// ── Determinism (ADR-135 §6.10, §13 M1 gate clause 2) ──────────────

/// The canonical serialization of one run, as bytes. Two runs are "the same
/// run" only if these are byte-identical.
fn ledger_bytes(report: &SimulationReport) -> Vec<u8> {
    serde_json::to_vec(report).expect("the report serializes")
}

fn mixed_scenario() -> (
    ExecutionSettings,
    Vec<SymbolStream>,
    Vec<(usize, OrderRequest)>,
) {
    let settings = ExecutionSettings {
        latency: LatencyModel::SeededUniform {
            decision_to_submit_min_ns: 0,
            decision_to_submit_max_ns: 5 * SECOND_NS,
            submit_to_exchange_min_ns: 0,
            submit_to_exchange_max_ns: SECOND_NS,
        },
        ..costed_settings()
    };
    let streams = vec![ramp("aaa", 24), offset_ramp("bbb", 24)];
    let script = vec![
        (0, OrderRequest::market(SymbolId(0), OrderSide::Buy, 3.0)),
        (1, OrderRequest::market(SymbolId(1), OrderSide::Sell, 2.0)),
        (
            3,
            OrderRequest::limit(SymbolId(0), OrderSide::Sell, 3.0, 104.0),
        ),
        (
            5,
            OrderRequest::stop(SymbolId(1), OrderSide::Buy, 2.0, 207.0),
        ),
        (
            9,
            OrderRequest::market_on_close(SymbolId(0), OrderSide::Buy, 1.0),
        ),
    ];
    (settings, streams, script)
}

#[test]
fn an_identical_run_is_bit_identical_when_repeated() {
    let (settings, streams, script) = mixed_scenario();
    let setup = SimulationSetup {
        seed: 0x5eed_0000_dead_beef,
        ..SimulationSetup::default()
    };

    let first = run_with(
        settings.clone(),
        setup.clone(),
        &streams,
        &mut OrderScript::new(script.clone()),
    )
    .expect("runs");
    let second = run_with(settings, setup, &streams, &mut OrderScript::new(script)).expect("runs");

    assert_eq!(ledger_bytes(&first), ledger_bytes(&second));
}

#[test]
fn concurrent_runs_of_the_same_input_agree_bit_for_bit() {
    let (settings, streams, script) = mixed_scenario();
    let setup = SimulationSetup {
        seed: 0x5eed_0000_dead_beef,
        ..SimulationSetup::default()
    };
    let expected = ledger_bytes(
        &run_with(
            settings.clone(),
            setup.clone(),
            &streams,
            &mut OrderScript::new(script.clone()),
        )
        .expect("runs"),
    );

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let settings = settings.clone();
            let setup = setup.clone();
            let streams = streams.clone();
            let script = script.clone();
            std::thread::spawn(move || {
                // Each thread runs the whole simulation independently; nothing
                // is shared, so a difference could only come from the engine
                // consulting something outside its inputs.
                ledger_bytes(
                    &run_with(settings, setup, &streams, &mut OrderScript::new(script))
                        .expect("runs"),
                )
            })
        })
        .collect();

    for handle in handles {
        let actual = handle.join().expect("thread completes");
        assert_eq!(actual, expected, "a concurrent run diverged");
    }
}

#[test]
fn interleaving_other_simulations_does_not_change_a_run() {
    let (settings, streams, script) = mixed_scenario();
    let setup = SimulationSetup {
        seed: 7,
        ..SimulationSetup::default()
    };
    let expected = ledger_bytes(
        &run_with(
            settings.clone(),
            setup.clone(),
            &streams,
            &mut OrderScript::new(script.clone()),
        )
        .expect("runs"),
    );

    // Twelve threads, half of them running a *different* seed, so any shared
    // or global RNG state would show up as a mismatch.
    let handles: Vec<_> = (0..12)
        .map(|index| {
            let settings = settings.clone();
            let streams = streams.clone();
            let script = script.clone();
            let seed = if index % 2 == 0 { 7 } else { 1_000 + index };
            std::thread::spawn(move || {
                let setup = SimulationSetup {
                    seed,
                    ..SimulationSetup::default()
                };
                (
                    seed,
                    ledger_bytes(
                        &run_with(settings, setup, &streams, &mut OrderScript::new(script))
                            .expect("runs"),
                    ),
                )
            })
        })
        .collect();

    for handle in handles {
        let (seed, actual) = handle.join().expect("thread completes");
        if seed == 7 {
            assert_eq!(actual, expected, "seed 7 diverged under interleaving");
        }
    }
}

#[test]
fn the_seed_is_the_only_source_of_latency_randomness() {
    let (settings, streams, script) = mixed_scenario();
    let at_seed = |seed: u64| {
        ledger_bytes(
            &run_with(
                settings.clone(),
                SimulationSetup {
                    seed,
                    ..SimulationSetup::default()
                },
                &streams,
                &mut OrderScript::new(script.clone()),
            )
            .expect("runs"),
        )
    };

    assert_eq!(at_seed(1), at_seed(1), "one seed, one ledger");
    assert_ne!(
        at_seed(1),
        at_seed(2),
        "a different seed must draw different delays"
    );
}

#[test]
fn a_fixed_latency_run_ignores_the_seed_entirely() {
    let settings = ExecutionSettings {
        latency: LatencyModel::Fixed {
            decision_to_submit_ns: SECOND_NS,
            submit_to_exchange_ns: 0,
        },
        ..costed_settings()
    };
    let streams = [ramp("aaa", 12)];
    let script = vec![(0, OrderSide::Buy, 2.0), (4, OrderSide::Sell, 2.0)];

    let at_seed = |seed: u64| {
        ledger_bytes(
            &run_with(
                settings.clone(),
                SimulationSetup {
                    seed,
                    ..SimulationSetup::default()
                },
                &streams,
                &mut ScriptedStrategy::new(script.clone()),
            )
            .expect("runs"),
        )
    };
    assert_eq!(
        at_seed(1),
        at_seed(9_999),
        "a deterministic latency model must not consult the RNG"
    );
}

#[test]
fn seeded_latency_stays_inside_its_declared_range() {
    let settings = ExecutionSettings {
        latency: LatencyModel::SeededUniform {
            decision_to_submit_min_ns: 2 * SECOND_NS,
            decision_to_submit_max_ns: 3 * SECOND_NS,
            submit_to_exchange_min_ns: 0,
            submit_to_exchange_max_ns: 0,
        },
        ..free_settings()
    };
    let script: Vec<(usize, OrderSide, f64)> =
        (0..20).map(|index| (index, OrderSide::Buy, 1.0)).collect();

    for seed in 0..8u64 {
        let report = run_with(
            settings.clone(),
            SimulationSetup {
                seed,
                ..SimulationSetup::default()
            },
            &[ramp("aaa", 24)],
            &mut ScriptedStrategy::new(script.clone()),
        )
        .expect("runs");

        let decisions: Vec<i64> = (0..20).map(|index| (index + 1) * MINUTE_NS - 1).collect();
        let submits: Vec<i64> = report
            .events
            .iter()
            .filter(|event| event.kind == SimEventKind::OrderSubmit)
            .map(|event| event.time_ns)
            .collect();
        assert_eq!(submits.len(), decisions.len());
        for (submit, decision) in submits.iter().zip(&decisions) {
            let delay = submit - decision;
            assert!(
                (2 * SECOND_NS..=3 * SECOND_NS).contains(&delay),
                "seed {seed} drew {delay} ns, outside the declared range"
            );
        }
    }
}
