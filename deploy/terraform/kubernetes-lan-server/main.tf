locals {
  labels = {
    "app.kubernetes.io/name"      = var.name
    "app.kubernetes.io/component" = "lan-sync"
  }
}

resource "kubernetes_namespace_v1" "typhoon" {
  metadata {
    name = var.namespace
  }
}

resource "kubernetes_persistent_volume_v1" "cache" {
  metadata {
    name   = "${var.name}-cache"
    labels = local.labels
  }

  spec {
    capacity = {
      storage = var.cache_size
    }
    access_modes                     = ["ReadWriteOnce"]
    persistent_volume_reclaim_policy = "Retain"
    storage_class_name               = "${var.name}-hostpath"

    persistent_volume_source {
      host_path {
        path = var.cache_host_path
        type = "DirectoryOrCreate"
      }
    }
  }
}

resource "kubernetes_persistent_volume_claim_v1" "cache" {
  metadata {
    name      = "${var.name}-cache"
    namespace = kubernetes_namespace_v1.typhoon.metadata[0].name
    labels    = local.labels
  }

  wait_until_bound = false

  spec {
    access_modes       = ["ReadWriteOnce"]
    storage_class_name = kubernetes_persistent_volume_v1.cache.spec[0].storage_class_name
    volume_name        = kubernetes_persistent_volume_v1.cache.metadata[0].name

    resources {
      requests = {
        storage = var.cache_size
      }
    }
  }
}

resource "kubernetes_deployment_v1" "lan_server" {
  metadata {
    name      = var.name
    namespace = kubernetes_namespace_v1.typhoon.metadata[0].name
    labels    = local.labels
  }

  spec {
    replicas = 1

    selector {
      match_labels = local.labels
    }

    template {
      metadata {
        labels = local.labels
      }

      spec {
        host_network  = var.host_network
        node_selector = var.node_selector

        security_context {
          run_as_user  = 1000
          run_as_group = 1000
          fs_group     = 1000
        }

        container {
          name              = "typhoon-lan-server"
          image             = var.image
          image_pull_policy = var.image_pull_policy

          args = [
            "--lan-server",
            "--cache-dir",
            "/cache",
            "--lan-port",
            tostring(var.lan_port),
          ]

          env {
            name  = "TYPHOON_CACHE_DIR"
            value = "/cache"
          }

          env {
            name  = "RUST_LOG"
            value = var.rust_log
          }

          port {
            name           = "lan-sync"
            container_port = var.lan_port
            host_port      = var.host_network ? var.lan_port : null
            protocol       = "TCP"
          }

          port {
            name           = "metrics"
            container_port = 9090
            protocol       = "TCP"
          }

          volume_mount {
            name       = "cache"
            mount_path = "/cache"
          }
        }

        volume {
          name = "cache"
          persistent_volume_claim {
            claim_name = kubernetes_persistent_volume_claim_v1.cache.metadata[0].name
          }
        }
      }
    }
  }
}

resource "kubernetes_service_v1" "lan_server" {
  metadata {
    name      = var.name
    namespace = kubernetes_namespace_v1.typhoon.metadata[0].name
    labels    = local.labels
  }

  spec {
    selector = local.labels
    type     = var.service_type

    port {
      name        = "lan-sync"
      port        = var.lan_port
      target_port = "lan-sync"
      node_port   = var.service_type == "NodePort" ? var.node_port : null
      protocol    = "TCP"
    }

    port {
      name        = "metrics"
      port        = 9090
      target_port = "metrics"
      protocol    = "TCP"
    }
  }
}
