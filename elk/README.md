# K8s ELK

Run the ELK([Elaticsearch](https://www.elastic.co/elasticsearch) +
[Logstash](https://www.elastic.co/logstash) +
[Kibana](https://www.elastic.co/kibana)) stack in the Kubernetes

## Usage

* Install the service

```bash
export K8S_NAMESPACE="elk"
# kubectl create ns "${K8S_NAMESPACE}"

# Install the operator and CRDs
helm upgrade -i elastic-operator elastic/eck-operator -n elastic-system \
  --set=installCRDs=true \
  --set=managedNamespaces="{${K8S_NAMESPACE:-default}}" \
  --set=createClusterScopedResources=false \
  --set=webhook.enabled=false \
  --set=config.validateStorageClass=false

bin/install.sh
# It would take a few minutes for the service to be ready
```

* Expose the service through the ingress

```shell
kubectl patch configmap tcp-services -n ingress-nginx \
  --patch-file k8s/patch/ingress-nginx-tcp-configmap-patch.yaml

kubectl patch deployment ingress-nginx-controller -n ingress-nginx \
  --patch-file k8s/patch/ingress-nginx-controller-patch.yaml
```

* Login to the Kibana `https://[HOST]:5601`

```shell
# Get Default Password for user 'elastic'
kubectl get secret elasticsearch-es-elastic-user -n ${K8S_NAMESPACE} -o=jsonpath='{.data.elastic}' | \
  base64 --decode; echo
```

* Check the Kibana status `https://[HOST]:5601/status`

* Uninstall the service

```shell
export K8S_NAMESPACE="elk"
bin/uninstall.sh
```
