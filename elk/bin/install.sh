#!/usr/bin/env bash

# Install the repo
# shellcheck disable=SC2143
if [[ ! $(helm repo list | grep -q elastic) ]]; then
  helm repo add elastic https://helm.elastic.co
fi

# Refer to https://www.elastic.co/guide/en/cloud-on-k8s/current/k8s-stack-helm-chart.html
helm upgrade -i eck elastic/eck-stack \
  --namespace "${K8S_NAMESPACE:-default}" \
  --values helm/basic-eck.yaml
