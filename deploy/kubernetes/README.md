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
export TYPHOON_NODE_PORT=30947
export TYPHOON_HOST_NETWORK=true
export TYPHOON_RUST_LOG=typhoon_engine=info,typhoon_cli=info

envsubst < deploy/kubernetes/lan-server.yaml.tpl | kubectl apply -f -
```

Use a local directory or a NAS mount for `TYPHOON_CACHE_HOST_PATH`.
Existing caches can leave `TYPHOON_LAN_BOOTSTRAP_PASSPHRASE` empty. Fresh caches can set it once; the CLI server persists it into the same keyring/KV locations used by GUI mode.
