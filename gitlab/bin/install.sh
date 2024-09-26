#!/usr/bin/env bash

# Install the repo
# shellcheck disable=SC2143
if [[ ! $(helm repo list | grep -q gitlab) ]]; then
  helm repo add bitnami https://charts.gitlab.io/
fi

# For more configure: https://docs.gitlab.com/charts/installation/command-line-options.html
helm upgrade -i gitlab gitlab/gitlab \
  --namespace "${K8S_NAMESPACE:-default}" \
  --timeout 600s \
  --set global.hosts.domain="${GITLAB_HOST:-example.com}" \
  --set global.hosts.externalIP=10.10.10.10 \
  --set certmanager-issuer.email="${GITLAB_CA_ISSUER_EMAIL:-me@example.com}" \
  --set global.image.pullPolicy=IfNotPresent \
  --set gitlab-runner.install=false \
  --set global.shell.port=32022 \
  --set nginx-ingress.controller.service.type=NodePort
