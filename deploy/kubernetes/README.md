# TyphooN LAN Server Kubernetes Template

Render `lan-server.yaml.tpl` with the cache path before applying:

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

Use a local directory or a NAS mount for `TYPHOON_CACHE_HOST_PATH`.
Configure the LAN passphrase once in the GUI LAN Sync panel before deploying; the CLI server reads the same keyring/KV value from the mounted cache.
