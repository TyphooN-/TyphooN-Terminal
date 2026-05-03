# LAN Server Deployment

TyphooN LAN sync can run from the native GUI or from the CLI container. Both modes use the same `typhoon_engine::core::lan_sync` protocol, the same `typhoon_cache.db` SQLite schema, and the same TLS + PBKDF2-HMAC passphrase authentication.

## Cache Directory Contract

The LAN server reads and writes `typhoon_cache.db` in the configured cache directory.

CLI precedence:

1. `--cache-dir PATH`
2. `TYPHOON_CACHE_DIR`
3. GUI cache override at `~/.config/typhoon-terminal/cache_location.txt`
4. Default `~/.config/typhoon-terminal/cache`

For Docker and Kubernetes, mount any local directory or NAS-mounted directory to `/cache` and run the CLI with `--cache-dir /cache`.

## LAN Passphrase Contract

Existing cache servers do not need a password variable. Configure the LAN password once in the GUI LAN Sync panel; the GUI stores it in the OS keyring under `lan_sync_passphrase` and mirrors it into the cache KV key `cred:lan_sync_passphrase`; CLI/headless mode reads the same locations.

Fresh cache servers can be bootstrapped by passing the password once. If CLI/headless mode receives `--lan-passphrase` or `TYPHOON_LAN_PASSPHRASE` and no saved LAN password already exists, it writes that value into the same keyring/KV locations. Saved values win over bootstrap values on later starts.

## Docker Compose

```bash
export TYPHOON_CACHE_HOST_PATH=/mnt/nas/typhoon-cache
docker compose up -d --build lan-server
```

For a fresh cache:

```bash
export TYPHOON_CACHE_HOST_PATH=/mnt/nas/typhoon-cache
export TYPHOON_LAN_PASSPHRASE='change-this'
docker compose up -d --build lan-server
```

The CLI server exposes Prometheus metrics at `http://<docker-host-ip>:9090/metrics` by default. To start the bundled Prometheus + Grafana profile:

```bash
export TYPHOON_CACHE_HOST_PATH=/mnt/nas/typhoon-cache
export TYPHOON_GRAFANA_ADMIN_PASSWORD='change-this'
docker compose --profile observability up -d --build
```

Prometheus listens on `TYPHOON_PROMETHEUS_PORT` (default host port `9091`) and Grafana listens on `TYPHOON_GRAFANA_PORT` (default `3000`) with a provisioned TyphooN LAN Server dashboard.

To start the bundled single-node Apache Kafka broker:

```bash
export TYPHOON_KAFKA_ADVERTISED_HOST=<docker-host-ip>
docker compose --profile kafka up -d kafka
```

Kafka clients on the LAN use `<docker-host-ip>:9092`. Compose-internal clients use `kafka:19092`.

Clients connect to:

```text
wss://<docker-host-ip>:9847
```

Use the same saved passphrase in GUI LAN Client Mode or:

```bash
typhoon-cli --lan-client <server-ip>
```

## Kubernetes Manifest Template

Render the template with the cache path before applying it:

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

For a fresh cache, set `TYPHOON_LAN_BOOTSTRAP_PASSPHRASE` before rendering.

When `TYPHOON_HOST_NETWORK=true`, LAN clients can use `<node-ip>:9847`. Otherwise use the `NodePort` value.

Prometheus, Grafana, and Kafka can be deployed with `deploy/kubernetes/observability-kafka.yaml.tpl` after the LAN server namespace and service exist. Set the image, node port, Grafana admin, and Kafka host path variables shown in `deploy/kubernetes/README.md`, then render with `envsubst`.

## Terraform

```bash
cd deploy/terraform/kubernetes-lan-server
terraform init
terraform apply \
  -var='cache_host_path=/mnt/nas/typhoon-cache' \
  -var='image=typhoon-terminal:lan-server'
```

For a fresh cache, add `-var='lan_bootstrap_passphrase=change-this'`.

For a NAS path, mount the NAS on the selected Kubernetes node before applying. If multiple nodes can schedule the pod, set `node_selector` so the pod lands on the node that has the cache path mounted.

Add `-var='enable_prometheus=true' -var='enable_grafana=true'` to deploy Prometheus and Grafana. Add `-var='enable_kafka=true' -var='kafka_host_path=/mnt/nas/typhoon-kafka' -var='kafka_advertised_host=<node-ip>'` to deploy the optional Kafka broker.

## Ansible

```bash
ansible-playbook -i inventory deploy/ansible/playbooks/lan-server.yml \
  -e typhoon_lan_server_cache_host_path=/mnt/nas/typhoon-cache \
  -e typhoon_lan_server_image=typhoon-terminal:lan-server
```

Set `typhoon_lan_server_build_context` if the target host should build from a local repository checkout instead of using a prebuilt image.
For a fresh cache, add `-e typhoon_lan_server_bootstrap_passphrase=change-this` or provide that variable from Ansible Vault.

Set `typhoon_lan_server_prometheus_enabled=true` and `typhoon_lan_server_grafana_enabled=true` for the local observability stack. Set `typhoon_lan_server_kafka_enabled=true` plus `typhoon_lan_server_kafka_advertised_host=<host-ip>` for Kafka.
