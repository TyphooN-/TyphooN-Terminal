apiVersion: v1
kind: Namespace
metadata:
  name: ${TYPHOON_NAMESPACE}
---
apiVersion: v1
kind: PersistentVolume
metadata:
  name: typhoon-cache
spec:
  capacity:
    storage: ${TYPHOON_CACHE_SIZE}
  accessModes:
    - ReadWriteOnce
  persistentVolumeReclaimPolicy: Retain
  storageClassName: typhoon-hostpath
  hostPath:
    path: ${TYPHOON_CACHE_HOST_PATH}
    type: DirectoryOrCreate
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: typhoon-cache
  namespace: ${TYPHOON_NAMESPACE}
spec:
  accessModes:
    - ReadWriteOnce
  storageClassName: typhoon-hostpath
  volumeName: typhoon-cache
  resources:
    requests:
      storage: ${TYPHOON_CACHE_SIZE}
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: typhoon-lan-server
  namespace: ${TYPHOON_NAMESPACE}
  labels:
    app.kubernetes.io/name: typhoon-lan-server
spec:
  replicas: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: typhoon-lan-server
  template:
    metadata:
      labels:
        app.kubernetes.io/name: typhoon-lan-server
    spec:
      hostNetwork: ${TYPHOON_HOST_NETWORK}
      securityContext:
        runAsUser: 1000
        runAsGroup: 1000
        fsGroup: 1000
      containers:
        - name: typhoon-lan-server
          image: ${TYPHOON_IMAGE}
          imagePullPolicy: ${TYPHOON_IMAGE_PULL_POLICY}
          args:
            - --lan-server
            - --cache-dir
            - /cache
            - --lan-port
            - "${TYPHOON_LAN_PORT}"
          env:
            - name: TYPHOON_CACHE_DIR
              value: /cache
            - name: RUST_LOG
              value: ${TYPHOON_RUST_LOG}
          ports:
            - name: lan-sync
              containerPort: ${TYPHOON_LAN_PORT}
              protocol: TCP
            - name: metrics
              containerPort: 9090
              protocol: TCP
          volumeMounts:
            - name: cache
              mountPath: /cache
      volumes:
        - name: cache
          persistentVolumeClaim:
            claimName: typhoon-cache
---
apiVersion: v1
kind: Service
metadata:
  name: typhoon-lan-server
  namespace: ${TYPHOON_NAMESPACE}
spec:
  type: NodePort
  selector:
    app.kubernetes.io/name: typhoon-lan-server
  ports:
    - name: lan-sync
      port: ${TYPHOON_LAN_PORT}
      targetPort: lan-sync
      nodePort: ${TYPHOON_NODE_PORT}
      protocol: TCP
    - name: metrics
      port: 9090
      targetPort: metrics
      protocol: TCP
