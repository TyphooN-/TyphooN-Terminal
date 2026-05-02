# TyphooN Kubernetes LAN Server Terraform

Deploys the headless CLI LAN sync server into Kubernetes with a hostPath cache volume.

```bash
terraform init
terraform apply \
  -var='cache_host_path=/mnt/nas/typhoon-cache' \
  -var='image=typhoon-terminal:lan-server'
```

`cache_host_path` can be a local disk path or a NAS mount. If the cluster has multiple nodes, set `node_selector` so the pod lands on the node where that path is mounted.

Existing caches need no passphrase variable. For a fresh cache, add `-var='lan_bootstrap_passphrase=change-this'`; the CLI server persists it into the same keyring/KV locations used by GUI mode.
