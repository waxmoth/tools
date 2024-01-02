# K8s Redis Cluster
A [Redis cluster](https://github.com/bitnami/charts/tree/main/bitnami/redis-cluster/#installing-the-chart) run in the Kubernetes

## Dependencies
* [Minikube](https://minikube.sigs.k8s.io/docs/) for local development
* [Helm](https://helm.sh/) the package manager for K8s

## Usage
* Add helm repo
```bash
helm repo add bitnami https://charts.bitnami.com/bitnami
```

* Install the cluster
```bash
helm install redis-local bitnami/redis --set usePassword=false

# Upgrade it to enable the persistence
helm upgrade redis-local bitnami/redis --set volumePermissions.enabled=true \
     --set persistence.storageClass=nfs-client,redis.replicas.persistence.storageClass=nfs-client
```

* Connect your redis cluster in the K8s cluster
```bash
export REDIS_PASSWORD=$(kubectl get secret --namespace default redis-local -o jsonpath="{.data.redis-password}" | base64 -d)

# Start one redis-cli
kubectl run --namespace default redis-client --restart='Never'  --env REDIS_PASSWORD=$REDIS_PASSWORD \
  --image docker.io/bitnami/redis:7.2.3-debian-11-r2 --command -- sleep infinity

kubectl exec --tty -i redis-client \
   --namespace default -- bash

# Connect to the redis through the redis-cli
REDISCLI_AUTH="$REDIS_PASSWORD" redis-cli -h redis-local-master
REDISCLI_AUTH="$REDIS_PASSWORD" redis-cli -h redis-local-replicas
```

## Remove the cluster
```bash
helm delete redis-local
```
