# Docker Registry and UI in K8s

Run the [Docker Registry UI](https://github.com/Joxit/docker-registry-ui) in the Kubernetes

## Usage

* Install the service

```bash
export K8S_NAMESPACE="docker-registry"
export REGISTRY_HOST="example.com"
export REGISTRY_SECRET_NAME="docker-registry-secret"
export DEFAULT_DOCKER_USER="<USER>"

# If not set the namespace
# kubectl create ns "${K8S_NAMESPACE}"

# Create the htpasswd secret
docker run -it --entrypoint htpasswd \
  httpd:alpine -Bbn $DEFAULT_DOCKER_USER <PASSWORD> > /tmp/htpasswd
kubectl create secret generic $REGISTRY_SECRET_NAME --dry-run=client \
  -n $K8S_NAMESPACE \
  --from-file=/tmp/htpasswd -o yaml | kubectl apply -f -

bin/install.sh
```

* Login to the docker

```shell
# Add this file /etc/docker/daemon.json to add the insecure-registry
# e.g. insecure-registries = ["${REGISTRY_HOST}"]
docker login ${REGISTRY_HOST} -u ${DEFAULT_DOCKER_USER}
```

* Uninstall the service

```shell
bin/uninstall.sh
```
