#!/usr/bin/env bash

# Install the repo
# shellcheck disable=SC2143
if [[ ! $(helm repo list | grep joxit) ]]; then
  helm repo add joxit https://helm.joxit.dev
fi

# For more configure: https://github.com/Joxit/helm-charts/blob/main/charts/docker-registry-ui/values.yaml
helm upgrade -i docker-registry joxit/docker-registry-ui \
  --namespace "${K8S_NAMESPACE:-default}" \
  --timeout 600s \
  -f helm/docker-registry-values.yml \
  --set ui.ingress.host="${REGISTRY_HOST:-example.com}" \
  --set registry.auth.basic.secretName="${REGISTRY_SECRET_NAME}" \
  --set registry.ingress.host="${REGISTRY_HOST:-example.com}"