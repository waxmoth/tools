# K8s Redis Cluster

A [Redis cluster](https://github.com/bitnami/charts/tree/main/bitnami/redis)
run in the Kubernetes

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
# Create the redis cluster which has one node and without the password auth
helm install redis-local bitnami/redis \
  --set architecture=standalone \
  --set auth.enabled=false \
  --set volumePermissions.enabled=true \
  --set persistence.storageClass=nfs-client,redis.replicas.persistence.storageClass=nfs-client
```

* Connect your redis cluster in the K8s cluster

```bash
# Start one redis-cli
kubectl run redis-client --restart='Never' \
  --image docker.io/bitnami/redis:7.2.3-debian-11-r2 --command -- sleep infinity

kubectl exec -it redis-client -- bash

# Connect to the redis through the redis-cli
redis-cli -h redis-local-master
```

## Expose the Redis tcp port

```bash
kubectl patch configmap tcp-services -n ingress-nginx \
  --patch-file k8s/patch/ingress-nginx-tcp-configmap-patch.yaml

kubectl patch deployment ingress-nginx-controller -n ingress-nginx \
  --patch-file k8s/patch/ingress-nginx-controller-patch.yaml

# Test the tcp port
telnet $(minikube ip) 6379
```

## Remove the cluster

```bash
helm delete redis-local
```
