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
