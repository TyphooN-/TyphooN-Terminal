# ADR-206: Headless LAN Server Deployment

**Status:** Implemented
**Date:** 2026-05-02

## Context

LAN sync was available from the native GUI, but deployment on a small LAN server, NAS-mounted machine, or Kubernetes node needed a headless runtime. The server also needed an operator-supplied cache directory before deployment so `typhoon_cache.db` can live on local storage or a NAS mount.

## Decision

Add CLI LAN server/client mode and deployment assets:

- `typhoon-cli --lan-server` starts the shared `LanSyncServer`.
- `typhoon-cli --lan-client <host>` starts the shared `LanSyncClient`.
- `--cache-dir PATH` and `TYPHOON_CACHE_DIR` override the cache directory.
- If no explicit cache dir is set, the CLI reads the GUI cache override file at `~/.config/typhoon-terminal/cache_location.txt`.
- CLI LAN mode resolves the passphrase from the same locations as GUI mode: OS keyring key `lan_sync_passphrase`, then cache KV key `cred:lan_sync_passphrase`.
- For fresh cache servers, `--lan-passphrase` / `TYPHOON_LAN_PASSPHRASE` bootstraps the passphrase only when no saved value exists, then persists it into keyring/KV. Existing saved values win.
- `--metrics-port` / `TYPHOON_METRICS_PORT` exposes Prometheus text metrics for CLI/headless LAN mode. Port `0` disables the endpoint.
- Docker mounts the operator-provided host path at `/cache`.
- Docker Compose can start optional Prometheus, Grafana, and Apache Kafka profiles around the LAN server.
- Kubernetes and Terraform deployments use a hostPath PV whose path is supplied before apply, with optional Prometheus, Grafana, and Kafka resources.
- Ansible deploys the same Docker Compose stack to a LAN host, with variables for observability and Kafka.

The CLI does not implement a second LAN protocol. It links the same engine implementation as the GUI, so GUI and CLI LAN servers/clients remain wire-compatible.

## Deployment Surfaces

- `Dockerfile`: builds the CLI plus shared engine LAN sync code without GPU dependencies.
- `docker-compose.yml`: runs `typhoon-cli --lan-server --cache-dir /cache --metrics-port 9090` and defines optional `observability` and `kafka` profiles.
- `deploy/prometheus/prometheus.yml`: local Prometheus scrape config for the CLI metrics endpoint.
- `deploy/grafana/provisioning` and `deploy/grafana/dashboards`: provisioned Grafana datasource and LAN dashboard.
- `deploy/kubernetes/lan-server.yaml.tpl`: envsubst-rendered Kubernetes template.
- `deploy/kubernetes/observability-kafka.yaml.tpl`: optional Prometheus, Grafana, and single-node Kafka template.
- `deploy/terraform/kubernetes-lan-server`: Terraform module for Kubernetes hostPath deployment.
- `deploy/ansible/roles/typhoon_lan_server`: Docker Compose role for LAN hosts.
- `docs/deployment/lan-server.md`: operator instructions.

## Consequences

- **Pro:** LAN server can run on a headless host, NAS-mounted Linux box, or Kubernetes node.
- **Pro:** Cache placement is explicit and works with either local disks or NAS mounts.
- **Pro:** GUI and CLI LAN clients use the same TLS + PBKDF2-HMAC authentication and SQLite sync schema.
- **Pro:** Existing deployments do not need to pass the LAN passphrase; headless mode reads the existing GUI keyring/KV secret.
- **Pro:** Fresh cache servers can be fully deployed by supplying a bootstrap passphrase once.
- **Pro:** Prometheus and Grafana can monitor cache size, cache row counts, per-series bar counts, uptime, and server liveness without a GUI process.
- **Pro:** Kafka can be deployed beside the LAN server for downstream event streaming integrations without changing the LAN sync protocol.
- **Con:** Kubernetes hostPath requires the cache path to exist or be mountable on the scheduled node.
- **Con:** Bundled Kafka is a single-node KRaft broker for LAN integration and development, not a high-availability Kafka cluster.
- **Con:** CLI LAN server serves the cache sync protocol; GUI-only live UI workflows still require the GUI process.
