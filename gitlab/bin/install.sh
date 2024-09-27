#!/usr/bin/env bash

# Install the repo
# shellcheck disable=SC2143
if [[ ! $(helm repo list | grep gitlab) ]]; then
  helm repo add gitlab https://charts.gitlab.io/
fi

# For more configure: https://docs.gitlab.com/charts/installation/command-line-options.html
helm upgrade -i gitlab gitlab/gitlab \
  --namespace "${K8S_NAMESPACE:-default}" \
  --timeout 600s \
  --set global.hosts.domain="${GITLAB_HOST:-example.com}" \
  --set global.hosts.externalIP=10.10.10.10 \
  --set global.shell.port=32022 \
  --set global.image.pullPolicy=IfNotPresent \
  --set certmanager-issuer.email="${GITLAB_CA_ISSUER_EMAIL:-me@example.com}" \
  --set gitlab-runner.install=false \
  --set nginx-ingress.controller.replicaCount=1 \
  --set nginx-ingress.controller.service.type=NodePort
