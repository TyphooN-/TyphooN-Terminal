apiVersion: v1
kind: ConfigMap
metadata:
  name: typhoon-prometheus
  namespace: ${TYPHOON_NAMESPACE}
data:
  prometheus.yml: |
    global:
      scrape_interval: 15s
      evaluation_interval: 15s

    scrape_configs:
      - job_name: typhoon-lan-server
        metrics_path: /metrics
        static_configs:
          - targets:
              - typhoon-lan-server.${TYPHOON_NAMESPACE}.svc.cluster.local:${TYPHOON_METRICS_PORT}
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: typhoon-prometheus
  namespace: ${TYPHOON_NAMESPACE}
  labels:
    app.kubernetes.io/name: typhoon-prometheus
spec:
  replicas: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: typhoon-prometheus
  template:
    metadata:
      labels:
        app.kubernetes.io/name: typhoon-prometheus
    spec:
      containers:
        - name: prometheus
          image: ${TYPHOON_PROMETHEUS_IMAGE}
          args:
            - --config.file=/etc/prometheus/prometheus.yml
            - --storage.tsdb.path=/prometheus
            - --web.enable-lifecycle
          ports:
            - name: http
              containerPort: 9090
          volumeMounts:
            - name: config
              mountPath: /etc/prometheus/prometheus.yml
              subPath: prometheus.yml
            - name: data
              mountPath: /prometheus
      volumes:
        - name: config
          configMap:
            name: typhoon-prometheus
        - name: data
          emptyDir: {}
---
apiVersion: v1
kind: Service
metadata:
  name: typhoon-prometheus
  namespace: ${TYPHOON_NAMESPACE}
spec:
  type: NodePort
  selector:
    app.kubernetes.io/name: typhoon-prometheus
  ports:
    - name: http
      port: ${TYPHOON_PROMETHEUS_PORT}
      targetPort: http
      nodePort: ${TYPHOON_PROMETHEUS_NODE_PORT}
---
apiVersion: v1
kind: Secret
metadata:
  name: typhoon-grafana-admin
  namespace: ${TYPHOON_NAMESPACE}
type: Opaque
stringData:
  user: ${TYPHOON_GRAFANA_ADMIN_USER}
  password: ${TYPHOON_GRAFANA_ADMIN_PASSWORD}
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: typhoon-grafana-provisioning
  namespace: ${TYPHOON_NAMESPACE}
data:
  prometheus.yml: |
    apiVersion: 1

    datasources:
      - name: Prometheus
        uid: typhoon-prometheus
        type: prometheus
        access: proxy
        url: http://typhoon-prometheus.${TYPHOON_NAMESPACE}.svc.cluster.local:9090
        isDefault: true
        editable: false
  dashboards.yml: |
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
  typhoon-lan-server.json: |
    {
      "editable": true,
      "panels": [
        {
          "datasource": {
            "type": "prometheus",
            "uid": "typhoon-prometheus"
          },
          "gridPos": {
            "h": 4,
            "w": 8,
            "x": 0,
            "y": 0
          },
          "id": 1,
          "options": {
            "reduceOptions": {
              "calcs": [
                "lastNotNull"
              ],
              "fields": "",
              "values": false
            }
          },
          "targets": [
            {
              "expr": "typhoon_lan_server_up",
              "legendFormat": "{{mode}}",
              "refId": "A"
            }
          ],
          "title": "LAN Server Up",
          "type": "stat"
        },
        {
          "datasource": {
            "type": "prometheus",
            "uid": "typhoon-prometheus"
          },
          "fieldConfig": {
            "defaults": {
              "unit": "bytes"
            },
            "overrides": []
          },
          "gridPos": {
            "h": 4,
            "w": 8,
            "x": 8,
            "y": 0
          },
          "id": 2,
          "targets": [
            {
              "expr": "typhoon_cache_size_bytes",
              "legendFormat": "cache",
              "refId": "A"
            }
          ],
          "title": "Cache Size",
          "type": "stat"
        },
        {
          "datasource": {
            "type": "prometheus",
            "uid": "typhoon-prometheus"
          },
          "gridPos": {
            "h": 8,
            "w": 24,
            "x": 0,
            "y": 4
          },
          "id": 3,
          "targets": [
            {
              "expr": "topk(20, typhoon_cache_entry_bars)",
              "legendFormat": "{{symbol}} {{timeframe}}",
              "refId": "A"
            }
          ],
          "title": "Largest Bar Series",
          "type": "timeseries"
        }
      ],
      "refresh": "30s",
      "schemaVersion": 39,
      "tags": [
        "typhoon",
        "lan"
      ],
      "time": {
        "from": "now-6h",
        "to": "now"
      },
      "title": "TyphooN LAN Server",
      "uid": "typhoon-lan-server",
      "version": 1
    }
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: typhoon-grafana
  namespace: ${TYPHOON_NAMESPACE}
  labels:
    app.kubernetes.io/name: typhoon-grafana
spec:
  replicas: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: typhoon-grafana
  template:
    metadata:
      labels:
        app.kubernetes.io/name: typhoon-grafana
    spec:
      containers:
        - name: grafana
          image: ${TYPHOON_GRAFANA_IMAGE}
          env:
            - name: GF_SECURITY_ADMIN_USER
              valueFrom:
                secretKeyRef:
                  name: typhoon-grafana-admin
                  key: user
            - name: GF_SECURITY_ADMIN_PASSWORD
              valueFrom:
                secretKeyRef:
                  name: typhoon-grafana-admin
                  key: password
            - name: GF_USERS_ALLOW_SIGN_UP
              value: "false"
          ports:
            - name: http
              containerPort: 3000
          volumeMounts:
            - name: provisioning
              mountPath: /etc/grafana/provisioning/datasources/prometheus.yml
              subPath: prometheus.yml
            - name: provisioning
              mountPath: /etc/grafana/provisioning/dashboards/typhoon.yml
              subPath: dashboards.yml
            - name: provisioning
              mountPath: /var/lib/grafana/dashboards/typhoon-lan-server.json
              subPath: typhoon-lan-server.json
      volumes:
        - name: provisioning
          configMap:
            name: typhoon-grafana-provisioning
---
apiVersion: v1
kind: Service
metadata:
  name: typhoon-grafana
  namespace: ${TYPHOON_NAMESPACE}
spec:
  type: NodePort
  selector:
    app.kubernetes.io/name: typhoon-grafana
  ports:
    - name: http
      port: ${TYPHOON_GRAFANA_PORT}
      targetPort: http
      nodePort: ${TYPHOON_GRAFANA_NODE_PORT}
---
apiVersion: v1
kind: PersistentVolume
metadata:
  name: typhoon-kafka
spec:
  capacity:
    storage: ${TYPHOON_KAFKA_SIZE}
  accessModes:
    - ReadWriteOnce
  persistentVolumeReclaimPolicy: Retain
  storageClassName: typhoon-kafka-hostpath
  hostPath:
    path: ${TYPHOON_KAFKA_HOST_PATH}
    type: DirectoryOrCreate
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: typhoon-kafka
  namespace: ${TYPHOON_NAMESPACE}
spec:
  accessModes:
    - ReadWriteOnce
  storageClassName: typhoon-kafka-hostpath
  volumeName: typhoon-kafka
  resources:
    requests:
      storage: ${TYPHOON_KAFKA_SIZE}
---
apiVersion: v1
kind: Service
metadata:
  name: typhoon-kafka-headless
  namespace: ${TYPHOON_NAMESPACE}
spec:
  clusterIP: None
  selector:
    app.kubernetes.io/name: typhoon-kafka
  ports:
    - name: controller
      port: 29093
      targetPort: controller
---
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: typhoon-kafka
  namespace: ${TYPHOON_NAMESPACE}
  labels:
    app.kubernetes.io/name: typhoon-kafka
spec:
  serviceName: typhoon-kafka-headless
  replicas: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: typhoon-kafka
  template:
    metadata:
      labels:
        app.kubernetes.io/name: typhoon-kafka
    spec:
      containers:
        - name: kafka
          image: ${TYPHOON_KAFKA_IMAGE}
          env:
            - name: KAFKA_NODE_ID
              value: "1"
            - name: KAFKA_LISTENER_SECURITY_PROTOCOL_MAP
              value: CONTROLLER:PLAINTEXT,PLAINTEXT:PLAINTEXT,PLAINTEXT_HOST:PLAINTEXT
            - name: KAFKA_ADVERTISED_LISTENERS
              value: PLAINTEXT_HOST://${TYPHOON_KAFKA_ADVERTISED_HOST}:${TYPHOON_KAFKA_NODE_PORT},PLAINTEXT://typhoon-kafka.${TYPHOON_NAMESPACE}.svc.cluster.local:19092
            - name: KAFKA_PROCESS_ROLES
              value: broker,controller
            - name: KAFKA_CONTROLLER_QUORUM_VOTERS
              value: 1@typhoon-kafka-0.typhoon-kafka-headless.${TYPHOON_NAMESPACE}.svc.cluster.local:29093
            - name: KAFKA_LISTENERS
              value: CONTROLLER://:29093,PLAINTEXT_HOST://:9092,PLAINTEXT://:19092
            - name: KAFKA_INTER_BROKER_LISTENER_NAME
              value: PLAINTEXT
            - name: KAFKA_CONTROLLER_LISTENER_NAMES
              value: CONTROLLER
            - name: CLUSTER_ID
              value: ${TYPHOON_KAFKA_CLUSTER_ID}
            - name: KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR
              value: "1"
            - name: KAFKA_GROUP_INITIAL_REBALANCE_DELAY_MS
              value: "0"
            - name: KAFKA_TRANSACTION_STATE_LOG_MIN_ISR
              value: "1"
            - name: KAFKA_TRANSACTION_STATE_LOG_REPLICATION_FACTOR
              value: "1"
            - name: KAFKA_SHARE_COORDINATOR_STATE_TOPIC_REPLICATION_FACTOR
              value: "1"
            - name: KAFKA_SHARE_COORDINATOR_STATE_TOPIC_MIN_ISR
              value: "1"
            - name: KAFKA_LOG_DIRS
              value: /tmp/kraft-combined-logs
          ports:
            - name: kafka-host
              containerPort: 9092
            - name: kafka
              containerPort: 19092
            - name: controller
              containerPort: 29093
          volumeMounts:
            - name: data
              mountPath: /tmp/kraft-combined-logs
      volumes:
        - name: data
          persistentVolumeClaim:
            claimName: typhoon-kafka
---
apiVersion: v1
kind: Service
metadata:
  name: typhoon-kafka
  namespace: ${TYPHOON_NAMESPACE}
spec:
  type: NodePort
  selector:
    app.kubernetes.io/name: typhoon-kafka
  ports:
    - name: kafka-host
      port: 9092
      targetPort: kafka-host
      nodePort: ${TYPHOON_KAFKA_NODE_PORT}
    - name: kafka
      port: 19092
      targetPort: kafka
