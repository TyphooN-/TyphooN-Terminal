variable "name" {
  description = "Base name for Kubernetes resources."
  type        = string
  default     = "typhoon-lan-server"
}

variable "namespace" {
  description = "Kubernetes namespace to create/use."
  type        = string
  default     = "typhoon"
}

variable "image" {
  description = "TyphooN CLI/LAN server image."
  type        = string
  default     = "typhoon-terminal:lan-server"
}

variable "image_pull_policy" {
  description = "Kubernetes imagePullPolicy for the LAN server container."
  type        = string
  default     = "IfNotPresent"
}

variable "cache_host_path" {
  description = "Local or NAS-mounted path on the Kubernetes node that stores typhoon_cache.db."
  type        = string
}

variable "cache_size" {
  description = "PersistentVolume/PersistentVolumeClaim size request."
  type        = string
  default     = "100Gi"
}

variable "lan_port" {
  description = "LAN sync port exposed by the TyphooN CLI server."
  type        = number
  default     = 9847
}

variable "metrics_port" {
  description = "Prometheus metrics port exposed by the TyphooN CLI server."
  type        = number
  default     = 9090
}

variable "lan_bootstrap_passphrase" {
  description = "Optional first-run LAN sync passphrase. Used only when the mounted cache/keyring does not already contain a LAN passphrase."
  type        = string
  default     = ""
  sensitive   = true
}

variable "service_type" {
  description = "Kubernetes service type. Use NodePort for bare-metal LAN clusters, LoadBalancer where available."
  type        = string
  default     = "NodePort"
}

variable "node_port" {
  description = "NodePort for LAN sync when service_type is NodePort."
  type        = number
  default     = 30947
}

variable "host_network" {
  description = "Bind directly on the node network. When true, LAN clients can use node_ip:lan_port."
  type        = bool
  default     = true
}

variable "node_selector" {
  description = "Optional node selector. Use this to pin the pod to the node that has cache_host_path mounted."
  type        = map(string)
  default     = {}
}

variable "rust_log" {
  description = "RUST_LOG value for the LAN server container."
  type        = string
  default     = "typhoon_engine=info,typhoon_cli=info"
}

variable "enable_prometheus" {
  description = "Deploy a Prometheus instance that scrapes the LAN server metrics endpoint."
  type        = bool
  default     = false
}

variable "prometheus_image" {
  description = "Prometheus container image."
  type        = string
  default     = "prom/prometheus:latest"
}

variable "prometheus_port" {
  description = "Prometheus service port."
  type        = number
  default     = 9090
}

variable "prometheus_node_port" {
  description = "NodePort for Prometheus when monitoring_service_type is NodePort."
  type        = number
  default     = 30090
}

variable "enable_grafana" {
  description = "Deploy Grafana with a provisioned Prometheus datasource and TyphooN LAN dashboard."
  type        = bool
  default     = false
}

variable "grafana_image" {
  description = "Grafana container image."
  type        = string
  default     = "grafana/grafana-oss:latest"
}

variable "grafana_port" {
  description = "Grafana service port."
  type        = number
  default     = 3000
}

variable "grafana_node_port" {
  description = "NodePort for Grafana when monitoring_service_type is NodePort."
  type        = number
  default     = 30300
}

variable "grafana_admin_user" {
  description = "Grafana admin username."
  type        = string
  default     = "admin"
}

variable "grafana_admin_password" {
  description = "Grafana admin password."
  type        = string
  default     = "admin"
  sensitive   = true
}

variable "monitoring_service_type" {
  description = "Kubernetes service type for Prometheus and Grafana."
  type        = string
  default     = "NodePort"
}

variable "enable_kafka" {
  description = "Deploy a single-node Apache Kafka KRaft broker for LAN-side event streaming integrations."
  type        = bool
  default     = false
}

variable "kafka_image" {
  description = "Apache Kafka container image."
  type        = string
  default     = "apache/kafka:4.2.0"
}

variable "kafka_host_path" {
  description = "Local or NAS-mounted path on the Kubernetes node for Kafka data."
  type        = string
  default     = "/srv/typhoon/kafka"
}

variable "kafka_size" {
  description = "PersistentVolume/PersistentVolumeClaim size request for Kafka data."
  type        = string
  default     = "20Gi"
}

variable "kafka_advertised_host" {
  description = "Host/IP advertised to LAN Kafka clients through the external listener."
  type        = string
  default     = "localhost"
}

variable "kafka_node_port" {
  description = "NodePort for Kafka's external plaintext listener."
  type        = number
  default     = 30092
}

variable "kafka_cluster_id" {
  description = "Kafka KRaft cluster id. Keep stable for an existing Kafka data directory."
  type        = string
  default     = "4L6g3nShT-eMCtK--X86sw"
}

variable "kubeconfig_path" {
  description = "Optional kubeconfig path. Null uses the provider default."
  type        = string
  default     = null
}

variable "kubeconfig_context" {
  description = "Optional kubeconfig context. Null uses the provider default."
  type        = string
  default     = null
}
