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

Do not pass the LAN server password through Docker, Kubernetes, Terraform, or Ansible. Configure it once in the GUI LAN Sync panel. The GUI stores it in the OS keyring under `lan_sync_passphrase` and mirrors it into the cache KV key `cred:lan_sync_passphrase`; CLI/headless mode reads the same locations.

## Docker Compose

```bash
export TYPHOON_CACHE_HOST_PATH=/mnt/nas/typhoon-cache
docker compose up -d --build lan-server
```

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
export TYPHOON_LAN_PORT=9847
export TYPHOON_NODE_PORT=30947
export TYPHOON_HOST_NETWORK=true
export TYPHOON_RUST_LOG=typhoon_engine=info,typhoon_cli=info

envsubst < deploy/kubernetes/lan-server.yaml.tpl | kubectl apply -f -
```

When `TYPHOON_HOST_NETWORK=true`, LAN clients can use `<node-ip>:9847`. Otherwise use the `NodePort` value.

## Terraform

```bash
cd deploy/terraform/kubernetes-lan-server
terraform init
terraform apply \
  -var='cache_host_path=/mnt/nas/typhoon-cache' \
  -var='image=typhoon-terminal:lan-server'
```

For a NAS path, mount the NAS on the selected Kubernetes node before applying. If multiple nodes can schedule the pod, set `node_selector` so the pod lands on the node that has the cache path mounted.

## Ansible

```bash
ansible-playbook -i inventory deploy/ansible/playbooks/lan-server.yml \
  -e typhoon_lan_server_cache_host_path=/mnt/nas/typhoon-cache \
  -e typhoon_lan_server_image=typhoon-terminal:lan-server
```

Set `typhoon_lan_server_build_context` if the target host should build from a local repository checkout instead of using a prebuilt image.
