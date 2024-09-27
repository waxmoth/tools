# K8s GitLab

Run the [Gitlab CE](https://gitlab.com/free-releases/gitlab-ce) in the Kubernetes

## Usage

* Install the service

```bash
export K8S_NAMESPACE="gitlab"
export GITLAB_HOST="example.com"
export GITLAB_CA_ISSUER_EMAIL="me@example.com"

# If not set the namespace
# kubectl create ns "${K8S_NAMESPACE}"

bin/install.sh
```

* Get initial password, then use this [link](http://gitlab.example.com) for login

```bash
export K8S_NAMESPACE="gitlab"
kubectl get secret --namespace "${K8S_NAMESPACE}" gitlab-gitlab-initial-root-password \
  -ojsonpath='{.data.password}' | base64 --decode ; echo
```

* Optional: Export the SSH port from K8s

```bash
helm upgrade -i gitlab gitlab/gitlab \
  --namespace "${K8S_NAMESPACE:-default}" \
  --set global.shell.port=32022 \
  --set nginx-ingress.controller.service.type=NodePort

# After set the SSH key into gitlab and configure the `~/.ssh/configure`, you can use SSH to clone your code
# Host example.com
#   PreferredAuthentications publickey
#   IdentityFile ~/.ssh/your_private_key
#   Port 32022

git clone ssh://git@gitlab.example.com:32022/[Project]/[REPO].git
```

* Register Gitlab runner into the cluster

```bash
helm upgrade -i gitlab-runner gitlab/gitlab-runner \
  --namespace gitlab \
  --set gitlabUrl=http://gitlab.${GITLAB_HOST},runnerToken=${RUNNER_TOKEN},runUntagged=true \
  --set rbac.create=true,rbac.serviceAccount=gitlab-runner,rbac.serviceAccountName=gitlab-runner

# If the runner cannot be registered by the TSL issue, you can use the following command to set the internal url
helm upgrade -i gitlab-runner gitlab/gitlab-runner \
  --namespace gitlab \
  --set gitlabUrl=http://gitlab-webservice-default.gitlab:8080,runnerToken=${RUNNER_TOKEN},runUntagged=true \
  --set rbac.create=true,rbac.serviceAccount=gitlab-runner,rbac.serviceAccountName=gitlab-runner
```

* Troubleshoots for GitLab Runner:

1. Got error from the job which related to `SSL certificate problem: self-signed certificate`

> Set the variables in the project's CI/CD settings, `GIT_SSL_NO_VERIFY=1`
Or try to add the self-signed CA in the cluster,
[doc](https://docs.gitlab.com/runner/configuration/tls-self-signed.html).

* Upgrade the service

```shell
# Export current HELM values
helm get values gitlab -n gitlab > gitlab_values.yaml

# For more versions: https://docs.gitlab.com/charts/installation/version_mappings.html
helm upgrade gitlab gitlab/gitlab -n gitlab \
  --version <CHART_NEW_VERSION> \
  -f gitlab_values.yaml \
  --set gitlab.migrations.enabled=true \
  --set global.gitlabVersion= <GITLAB_VERSION>
```

> You can check the new version form the website, path: Help -> Help

* Uninstall the service

```shell
export K8S_NAMESPACE="gitlab"
bin/uninstall.sh
```
