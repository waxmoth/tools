# LocalS3
The Local AWS S3 service run in the Kubernetes

## Dependencies
* [Minikube](https://minikube.sigs.k8s.io/docs/) for local development

## Usage
* Create k8s pods and services
```bash
kubectl apply -f k8s/
```

* Check services
```bash
kubectl get svc
```

* Get pods list
```bash
kubectl get pods -o wide
```

* Get endpoint
```shell
export EDGE_PORT=$(kubectl get --namespace default -o jsonpath="{.spec.ports[0].nodePort}" services local-s3)
export LOCALSTACK_HOST=$(kubectl get nodes --namespace default -o jsonpath="{.items[0].status.addresses[0].address}")
echo http://$LOCALSTACK_HOST:$EDGE_PORT

# Or get it from the Ingress settings, k8s/ingress.yaml
export EDGE_PORT=80
export LOCALSTACK_HOST=local-s3.test
```

* Test create bucket
```shell
awslocal s3 mb s3://test

echo 'test' > /tmp/test.txt && awslocal s3 cp /tmp/test.txt s3://test/
awslocal s3 ls s3://test/
```

