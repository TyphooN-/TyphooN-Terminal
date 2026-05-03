# TyphooN LAN Server Kubernetes Template

Render `lan-server.yaml.tpl` with the cache path before applying:

```bash
export TYPHOON_NAMESPACE=typhoon
export TYPHOON_IMAGE=typhoon-terminal:lan-server
export TYPHOON_IMAGE_PULL_POLICY=IfNotPresent
export TYPHOON_CACHE_HOST_PATH=/mnt/nas/typhoon-cache
export TYPHOON_CACHE_SIZE=100Gi
export TYPHOON_LAN_BOOTSTRAP_PASSPHRASE=
export TYPHOON_LAN_PORT=9847
export TYPHOON_METRICS_PORT=9090
export TYPHOON_NODE_PORT=30947
export TYPHOON_HOST_NETWORK=true
export TYPHOON_RUST_LOG=typhoon_engine=info,typhoon_cli=info

envsubst < deploy/kubernetes/lan-server.yaml.tpl | kubectl apply -f -
```

Use a local directory or a NAS mount for `TYPHOON_CACHE_HOST_PATH`.
Existing caches can leave `TYPHOON_LAN_BOOTSTRAP_PASSPHRASE` empty. Fresh caches can set it once; the CLI server persists it into the same keyring/KV locations used by GUI mode.

The LAN server exposes Prometheus metrics at `/metrics` on `TYPHOON_METRICS_PORT`.

To deploy optional Prometheus, Grafana, and single-node Kafka support:

```bash
export TYPHOON_PROMETHEUS_IMAGE=prom/prometheus:latest
export TYPHOON_PROMETHEUS_PORT=9090
export TYPHOON_PROMETHEUS_NODE_PORT=30090
export TYPHOON_GRAFANA_IMAGE=grafana/grafana-oss:latest
export TYPHOON_GRAFANA_PORT=3000
export TYPHOON_GRAFANA_NODE_PORT=30300
export TYPHOON_GRAFANA_ADMIN_USER=admin
export TYPHOON_GRAFANA_ADMIN_PASSWORD=admin
export TYPHOON_KAFKA_IMAGE=apache/kafka:4.2.0
export TYPHOON_KAFKA_HOST_PATH=/mnt/nas/typhoon-kafka
export TYPHOON_KAFKA_SIZE=20Gi
export TYPHOON_KAFKA_ADVERTISED_HOST=<node-ip>
export TYPHOON_KAFKA_NODE_PORT=30092
export TYPHOON_KAFKA_CLUSTER_ID=4L6g3nShT-eMCtK--X86sw

envsubst < deploy/kubernetes/observability-kafka.yaml.tpl | kubectl apply -f -
```
