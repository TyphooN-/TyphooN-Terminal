# ADR-209: LAN Observability and Kafka Deployment

**Status:** Implemented
**Date:** 2026-05-03

## Context

Headless LAN servers need operational visibility without opening the GUI. Operators also need a local Kafka broker option for downstream event streaming and future integrations. The LAN sync protocol should remain unchanged: observability and Kafka are deployment/runtime adjuncts, not a second sync path.

## Decision

Add a lightweight Prometheus endpoint to CLI/headless LAN mode:

- `--metrics-port` / `TYPHOON_METRICS_PORT` defaults to `9090`.
- `--metrics-port 0` disables the HTTP endpoint.
- `GET /metrics` emits Prometheus text exposition.
- `GET /healthz` returns a minimal health response.
- Metrics are read from the same `SqliteCache` used by LAN sync.

Initial CLI metrics:

- `typhoon_lan_server_up{mode}`.
- `typhoon_lan_server_uptime_seconds{mode}`.
- `typhoon_cache_bar_entries_total{db}`.
- `typhoon_cache_kv_entries_total{db}`.
- `typhoon_cache_size_bytes{db}` including WAL/SHM sidecars.
- `typhoon_cache_entry_bars{key,symbol,timeframe}`.
- `typhoon_cache_entry_updated_timestamp_ms{key,symbol,timeframe}`.

Add deployment support:

- Docker Compose `observability` profile with Prometheus and Grafana.
- Docker Compose `kafka` profile with a single-node Apache Kafka KRaft broker.
- Provisioned Grafana datasource and TyphooN LAN Server dashboard.
- Kubernetes `observability-kafka.yaml.tpl` for Prometheus, Grafana, and Kafka.
- Terraform variables/resources for optional Prometheus, Grafana, and Kafka.
- Ansible role variables/templates for the same optional services.

The Kafka broker is intentionally optional and standalone. TyphooN does not publish application events to Kafka in the current implementation; this ADR only standardizes the deployable broker and service discovery surface. Producer/consumer wiring is out of scope until a concrete event contract is accepted.

References:

- Prometheus configuration model: https://prometheus.io/docs/prometheus/latest/configuration/configuration/
- Grafana provisioning model: https://grafana.com/docs/grafana/latest/administration/provisioning/
- Apache Kafka 4.2 Docker image and KRaft examples: https://kafka.apache.org/42/getting-started/docker/

## Consequences

- **Pro:** LAN servers are scrapeable by Prometheus without GUI/native dependencies.
- **Pro:** Grafana starts with a ready dashboard for liveness, uptime, cache size, cache row counts, and largest bar series.
- **Pro:** Kafka can be deployed consistently across Docker Compose, Kubernetes, Terraform, and Ansible.
- **Pro:** Existing LAN sync remains protocol-compatible across GUI and CLI.
- **Con:** Per-cache-key metrics can be high-cardinality on very large caches; this is useful for LAN operations but should be watched before remote or multi-tenant Prometheus use.
- **Con:** The bundled Kafka topology is single-node KRaft. Production HA Kafka still needs an external cluster or a multi-broker deployment.
