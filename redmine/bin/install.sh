#!/usr/bin/env bash

# Install the repo
# shellcheck disable=SC2143
if [[ ! $(helm repo list | grep bitnami) ]]; then
  helm repo add bitnami https://charts.bitnami.com/bitnami
fi

# For more configure: https://artifacthub.io/packages/helm/bitnami/redmine
helm upgrade -i redmine bitnami/redmine \
  --namespace "${K8S_NAMESPACE:-default}"
