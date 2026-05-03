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

The CLI exposes Prometheus text metrics at `/metrics` on `metrics_port` (default `9090`). To deploy Prometheus and Grafana with a provisioned LAN server dashboard:

```bash
terraform apply \
  -var='cache_host_path=/mnt/nas/typhoon-cache' \
  -var='enable_prometheus=true' \
  -var='enable_grafana=true' \
  -var='grafana_admin_password=change-this'
```

To deploy the optional single-node Apache Kafka KRaft broker:

```bash
terraform apply \
  -var='cache_host_path=/mnt/nas/typhoon-cache' \
  -var='enable_kafka=true' \
  -var='kafka_host_path=/mnt/nas/typhoon-kafka' \
  -var='kafka_advertised_host=<node-ip>'
```

Kafka clients on the LAN use `<node-ip>:30092` by default. In-cluster clients use `typhoon-lan-server-kafka.typhoon.svc.cluster.local:19092`.
