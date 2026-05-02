# TyphooN Kubernetes LAN Server Terraform

Deploys the headless CLI LAN sync server into Kubernetes with a hostPath cache volume.

```bash
terraform init
terraform apply \
  -var='cache_host_path=/mnt/nas/typhoon-cache' \
  -var='image=typhoon-terminal:lan-server'
```

`cache_host_path` can be a local disk path or a NAS mount. If the cluster has multiple nodes, set `node_selector` so the pod lands on the node where that path is mounted.

Configure the LAN passphrase once in the GUI LAN Sync panel before deployment. The headless CLI server reads the same keyring/KV value from the mounted cache, so Terraform does not manage the passphrase.
