output "namespace" {
  value = kubernetes_namespace_v1.typhoon.metadata[0].name
}

output "service_name" {
  value = kubernetes_service_v1.lan_server.metadata[0].name
}

output "lan_port" {
  value = var.lan_port
}

output "node_port" {
  value = var.service_type == "NodePort" ? var.node_port : null
}

output "metrics_port" {
  value = var.metrics_port
}

output "prometheus_node_port" {
  value = var.enable_prometheus && var.monitoring_service_type == "NodePort" ? var.prometheus_node_port : null
}

output "grafana_node_port" {
  value = local.grafana_enabled && var.monitoring_service_type == "NodePort" ? var.grafana_node_port : null
}

output "kafka_bootstrap" {
  value = var.enable_kafka ? "${var.kafka_advertised_host}:${var.kafka_node_port}" : null
}

output "cache_host_path" {
  value = var.cache_host_path
}

output "client_hint" {
  value = var.host_network ? "Use <node-ip>:${var.lan_port} from GUI or CLI LAN clients." : "Use the Service address, LoadBalancer address, or <node-ip>:${var.node_port} when service_type=NodePort."
}
