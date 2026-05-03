resource "kubernetes_config_map_v1" "prometheus" {
  count = var.enable_prometheus ? 1 : 0

  metadata {
    name      = "${var.name}-prometheus"
    namespace = kubernetes_namespace_v1.typhoon.metadata[0].name
    labels    = local.labels
  }

  data = {
    "prometheus.yml" = <<-YAML
      global:
        scrape_interval: 15s
        evaluation_interval: 15s

      scrape_configs:
        - job_name: typhoon-lan-server
          metrics_path: /metrics
          static_configs:
            - targets:
                - ${var.name}.${var.namespace}.svc.cluster.local:${var.metrics_port}
    YAML
  }
}

resource "kubernetes_deployment_v1" "prometheus" {
  count = var.enable_prometheus ? 1 : 0

  metadata {
    name      = "${var.name}-prometheus"
    namespace = kubernetes_namespace_v1.typhoon.metadata[0].name
    labels = merge(local.labels, {
      "app.kubernetes.io/name"      = "${var.name}-prometheus"
      "app.kubernetes.io/component" = "prometheus"
    })
  }

  spec {
    replicas = 1

    selector {
      match_labels = {
        "app.kubernetes.io/name"      = "${var.name}-prometheus"
        "app.kubernetes.io/component" = "prometheus"
      }
    }

    template {
      metadata {
        labels = {
          "app.kubernetes.io/name"      = "${var.name}-prometheus"
          "app.kubernetes.io/component" = "prometheus"
        }
      }

      spec {
        node_selector = var.node_selector

        container {
          name  = "prometheus"
          image = var.prometheus_image

          args = [
            "--config.file=/etc/prometheus/prometheus.yml",
            "--storage.tsdb.path=/prometheus",
            "--web.enable-lifecycle",
          ]

          port {
            name           = "http"
            container_port = 9090
            protocol       = "TCP"
          }

          volume_mount {
            name       = "config"
            mount_path = "/etc/prometheus/prometheus.yml"
            sub_path   = "prometheus.yml"
          }

          volume_mount {
            name       = "data"
            mount_path = "/prometheus"
          }
        }

        volume {
          name = "config"
          config_map {
            name = kubernetes_config_map_v1.prometheus[0].metadata[0].name
          }
        }

        volume {
          name = "data"
          empty_dir {}
        }
      }
    }
  }
}

resource "kubernetes_service_v1" "prometheus" {
  count = var.enable_prometheus ? 1 : 0

  metadata {
    name      = "${var.name}-prometheus"
    namespace = kubernetes_namespace_v1.typhoon.metadata[0].name
    labels = merge(local.labels, {
      "app.kubernetes.io/name"      = "${var.name}-prometheus"
      "app.kubernetes.io/component" = "prometheus"
    })
  }

  spec {
    selector = {
      "app.kubernetes.io/name"      = "${var.name}-prometheus"
      "app.kubernetes.io/component" = "prometheus"
    }
    type = var.monitoring_service_type

    port {
      name        = "http"
      port        = var.prometheus_port
      target_port = "http"
      node_port   = var.monitoring_service_type == "NodePort" ? var.prometheus_node_port : null
      protocol    = "TCP"
    }
  }
}

resource "kubernetes_secret_v1" "grafana_admin" {
  count = local.grafana_enabled ? 1 : 0

  metadata {
    name      = "${var.name}-grafana-admin"
    namespace = kubernetes_namespace_v1.typhoon.metadata[0].name
    labels    = local.labels
  }

  data = {
    user     = var.grafana_admin_user
    password = var.grafana_admin_password
  }

  type = "Opaque"
}

resource "kubernetes_config_map_v1" "grafana" {
  count = local.grafana_enabled ? 1 : 0

  metadata {
    name      = "${var.name}-grafana-provisioning"
    namespace = kubernetes_namespace_v1.typhoon.metadata[0].name
    labels    = local.labels
  }

  data = {
    "prometheus.yml" = <<-YAML
      apiVersion: 1

      datasources:
        - name: Prometheus
          uid: typhoon-prometheus
          type: prometheus
          access: proxy
          url: http://${kubernetes_service_v1.prometheus[0].metadata[0].name}.${var.namespace}.svc.cluster.local:${var.prometheus_port}
          isDefault: true
          editable: false
    YAML

    "dashboards.yml" = <<-YAML
      apiVersion: 1

      providers:
        - name: TyphooN
          orgId: 1
          folder: TyphooN
          type: file
          disableDeletion: false
          editable: true
          options:
            path: /var/lib/grafana/dashboards
    YAML

    "typhoon-lan-server.json" = file("${path.module}/../../grafana/dashboards/typhoon-lan-server.json")
  }
}

resource "kubernetes_deployment_v1" "grafana" {
  count = local.grafana_enabled ? 1 : 0

  metadata {
    name      = "${var.name}-grafana"
    namespace = kubernetes_namespace_v1.typhoon.metadata[0].name
    labels = merge(local.labels, {
      "app.kubernetes.io/name"      = "${var.name}-grafana"
      "app.kubernetes.io/component" = "grafana"
    })
  }

  spec {
    replicas = 1

    selector {
      match_labels = {
        "app.kubernetes.io/name"      = "${var.name}-grafana"
        "app.kubernetes.io/component" = "grafana"
      }
    }

    template {
      metadata {
        labels = {
          "app.kubernetes.io/name"      = "${var.name}-grafana"
          "app.kubernetes.io/component" = "grafana"
        }
      }

      spec {
        node_selector = var.node_selector

        container {
          name  = "grafana"
          image = var.grafana_image

          env {
            name = "GF_SECURITY_ADMIN_USER"
            value_from {
              secret_key_ref {
                name = kubernetes_secret_v1.grafana_admin[0].metadata[0].name
                key  = "user"
              }
            }
          }

          env {
            name = "GF_SECURITY_ADMIN_PASSWORD"
            value_from {
              secret_key_ref {
                name = kubernetes_secret_v1.grafana_admin[0].metadata[0].name
                key  = "password"
              }
            }
          }

          env {
            name  = "GF_USERS_ALLOW_SIGN_UP"
            value = "false"
          }

          port {
            name           = "http"
            container_port = 3000
            protocol       = "TCP"
          }

          volume_mount {
            name       = "provisioning"
            mount_path = "/etc/grafana/provisioning/datasources/prometheus.yml"
            sub_path   = "prometheus.yml"
          }

          volume_mount {
            name       = "provisioning"
            mount_path = "/etc/grafana/provisioning/dashboards/typhoon.yml"
            sub_path   = "dashboards.yml"
          }

          volume_mount {
            name       = "provisioning"
            mount_path = "/var/lib/grafana/dashboards/typhoon-lan-server.json"
            sub_path   = "typhoon-lan-server.json"
          }
        }

        volume {
          name = "provisioning"
          config_map {
            name = kubernetes_config_map_v1.grafana[0].metadata[0].name
          }
        }
      }
    }
  }
}

resource "kubernetes_service_v1" "grafana" {
  count = local.grafana_enabled ? 1 : 0

  metadata {
    name      = "${var.name}-grafana"
    namespace = kubernetes_namespace_v1.typhoon.metadata[0].name
    labels = merge(local.labels, {
      "app.kubernetes.io/name"      = "${var.name}-grafana"
      "app.kubernetes.io/component" = "grafana"
    })
  }

  spec {
    selector = {
      "app.kubernetes.io/name"      = "${var.name}-grafana"
      "app.kubernetes.io/component" = "grafana"
    }
    type = var.monitoring_service_type

    port {
      name        = "http"
      port        = var.grafana_port
      target_port = "http"
      node_port   = var.monitoring_service_type == "NodePort" ? var.grafana_node_port : null
      protocol    = "TCP"
    }
  }
}

resource "kubernetes_persistent_volume_v1" "kafka" {
  count = var.enable_kafka ? 1 : 0

  metadata {
    name   = "${var.name}-kafka"
    labels = local.labels
  }

  spec {
    capacity = {
      storage = var.kafka_size
    }
    access_modes                     = ["ReadWriteOnce"]
    persistent_volume_reclaim_policy = "Retain"
    storage_class_name               = "${var.name}-kafka-hostpath"

    persistent_volume_source {
      host_path {
        path = var.kafka_host_path
        type = "DirectoryOrCreate"
      }
    }
  }
}

resource "kubernetes_persistent_volume_claim_v1" "kafka" {
  count = var.enable_kafka ? 1 : 0

  metadata {
    name      = "${var.name}-kafka"
    namespace = kubernetes_namespace_v1.typhoon.metadata[0].name
    labels    = local.labels
  }

  wait_until_bound = false

  spec {
    access_modes       = ["ReadWriteOnce"]
    storage_class_name = kubernetes_persistent_volume_v1.kafka[0].spec[0].storage_class_name
    volume_name        = kubernetes_persistent_volume_v1.kafka[0].metadata[0].name

    resources {
      requests = {
        storage = var.kafka_size
      }
    }
  }
}

resource "kubernetes_deployment_v1" "kafka" {
  count = var.enable_kafka ? 1 : 0

  metadata {
    name      = "${var.name}-kafka"
    namespace = kubernetes_namespace_v1.typhoon.metadata[0].name
    labels = merge(local.labels, {
      "app.kubernetes.io/name"      = "${var.name}-kafka"
      "app.kubernetes.io/component" = "kafka"
    })
  }

  spec {
    replicas = 1

    selector {
      match_labels = {
        "app.kubernetes.io/name"      = "${var.name}-kafka"
        "app.kubernetes.io/component" = "kafka"
      }
    }

    template {
      metadata {
        labels = {
          "app.kubernetes.io/name"      = "${var.name}-kafka"
          "app.kubernetes.io/component" = "kafka"
        }
      }

      spec {
        node_selector = var.node_selector

        container {
          name  = "kafka"
          image = var.kafka_image

          env {
            name  = "KAFKA_NODE_ID"
            value = "1"
          }
          env {
            name  = "KAFKA_LISTENER_SECURITY_PROTOCOL_MAP"
            value = "CONTROLLER:PLAINTEXT,PLAINTEXT:PLAINTEXT,PLAINTEXT_HOST:PLAINTEXT"
          }
          env {
            name  = "KAFKA_ADVERTISED_LISTENERS"
            value = "PLAINTEXT_HOST://${var.kafka_advertised_host}:${var.kafka_node_port},PLAINTEXT://${var.name}-kafka.${var.namespace}.svc.cluster.local:19092"
          }
          env {
            name  = "KAFKA_PROCESS_ROLES"
            value = "broker,controller"
          }
          env {
            name  = "KAFKA_CONTROLLER_QUORUM_VOTERS"
            value = "1@localhost:29093"
          }
          env {
            name  = "KAFKA_LISTENERS"
            value = "CONTROLLER://:29093,PLAINTEXT_HOST://:9092,PLAINTEXT://:19092"
          }
          env {
            name  = "KAFKA_INTER_BROKER_LISTENER_NAME"
            value = "PLAINTEXT"
          }
          env {
            name  = "KAFKA_CONTROLLER_LISTENER_NAMES"
            value = "CONTROLLER"
          }
          env {
            name  = "CLUSTER_ID"
            value = var.kafka_cluster_id
          }
          env {
            name  = "KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR"
            value = "1"
          }
          env {
            name  = "KAFKA_GROUP_INITIAL_REBALANCE_DELAY_MS"
            value = "0"
          }
          env {
            name  = "KAFKA_TRANSACTION_STATE_LOG_MIN_ISR"
            value = "1"
          }
          env {
            name  = "KAFKA_TRANSACTION_STATE_LOG_REPLICATION_FACTOR"
            value = "1"
          }
          env {
            name  = "KAFKA_SHARE_COORDINATOR_STATE_TOPIC_REPLICATION_FACTOR"
            value = "1"
          }
          env {
            name  = "KAFKA_SHARE_COORDINATOR_STATE_TOPIC_MIN_ISR"
            value = "1"
          }
          env {
            name  = "KAFKA_LOG_DIRS"
            value = "/tmp/kraft-combined-logs"
          }

          port {
            name           = "kafka-host"
            container_port = 9092
            protocol       = "TCP"
          }

          port {
            name           = "kafka"
            container_port = 19092
            protocol       = "TCP"
          }

          port {
            name           = "controller"
            container_port = 29093
            protocol       = "TCP"
          }

          volume_mount {
            name       = "data"
            mount_path = "/tmp/kraft-combined-logs"
          }
        }

        volume {
          name = "data"
          persistent_volume_claim {
            claim_name = kubernetes_persistent_volume_claim_v1.kafka[0].metadata[0].name
          }
        }
      }
    }
  }
}

resource "kubernetes_service_v1" "kafka" {
  count = var.enable_kafka ? 1 : 0

  metadata {
    name      = "${var.name}-kafka"
    namespace = kubernetes_namespace_v1.typhoon.metadata[0].name
    labels = merge(local.labels, {
      "app.kubernetes.io/name"      = "${var.name}-kafka"
      "app.kubernetes.io/component" = "kafka"
    })
  }

  spec {
    selector = {
      "app.kubernetes.io/name"      = "${var.name}-kafka"
      "app.kubernetes.io/component" = "kafka"
    }
    type = "NodePort"

    port {
      name        = "kafka-host"
      port        = 9092
      target_port = "kafka-host"
      node_port   = var.kafka_node_port
      protocol    = "TCP"
    }

    port {
      name        = "kafka"
      port        = 19092
      target_port = "kafka"
      protocol    = "TCP"
    }
  }
}
