# Docker Registry and UI in K8s

Run the [Docker Registry UI](https://github.com/Joxit/docker-registry-ui) in the Kubernetes

## Usage

* Install the service

```bash
export K8S_NAMESPACE="docker-registry"
export REGISTRY_HOST="example.com"
export REGISTRY_SECRET_NAME="docker-registry-secret"
export DEFAULT_DOCKER_USER="waxmoth"
export DEFAULT_DOCKER_EMAIL="me@example.com"

# If not set the namespace
# kubectl create ns "${K8S_NAMESPACE}"

# Create the htpasswd secret
htpasswd -cB /tmp/htpasswd $DEFAULT_DOCKER_USER
kubectl create secret generic $REGISTRY_SECRET_NAME --dry-run=client \
  -n $K8S_NAMESPACE \
  --from-file=/tmp/htpasswd -o yaml | kubectl apply -f -

bin/install.sh
```

* Uninstall the service

```shell
bin/uninstall.sh
```
