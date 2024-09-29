# K8s ELK

Run the ELK([Elaticsearch](https://www.elastic.co/elasticsearch) +
[Logstash](https://www.elastic.co/logstash) +
[Kibana](https://www.elastic.co/kibana)) stack in the Kubernetes

## Usage

* Install the service

```bash
# Add elasticsearch Custom Resource Definition (CRDs)
kubectl create -f https://download.elastic.co/downloads/eck/2.14.0/crds.yaml
# Add the operator
kubectl apply -f https://download.elastic.co/downloads/eck/2.14.0/operator.yaml

export K8S_NAMESPACE="elk"
# kubectl create ns "${K8S_NAMESPACE}"
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

* Uninstall the service

```shell
export K8S_NAMESPACE="elk"
bin/uninstall.sh
```
