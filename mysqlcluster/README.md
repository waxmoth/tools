# K8S MySQLCluster
A MySQL master/slave cluster run in the Kubernetes

## Dependencies
* [Minikube](https://minikube.sigs.k8s.io/docs/) for local development

## Usage
* Build images
```bash
# Build local images into the k8s
eval $(minikube docker-env)
docker-compose build

# Check the images
minikube ssh
> docker images
```

* Create k8s pods and services
```bash
kubectl apply -f k8s/
# Update the service?
# kubectl replace -f k8s/mysql-master.yaml
```

* Check services
```bash
kubectl get svc
```

* Get pods list
```bash
kubectl get pods -o wide
```

* Check the DB in the container
```bash
kubectl exec $(kubectl get pods|grep mysql-master|tail -n 1|awk '{print $1}') -it -- /bin/bash
> export MYSQL_PWD=${MYSQL_ROOT_PASSWORD}
> mysql -uroot
```

* Delete pod
```bash
kubectl delete pods $(kubectl get pods|grep mysql-master|tail -n 1|awk '{print $1}')
kubectl delete pods $(kubectl get pods|grep mysql-slaver|tail -n 1|awk '{print $1}')
```

* Clean the services and pods
```shell script
kubectl delete rc mysql-slaver
kubectl delete svc mysql-slaver
kubectl delete pvc db-slaver-pvc
kubectl delete pv db-slaver-pv

kubectl delete rc mysql-master
kubectl delete svc mysql-master
kubectl delete pvc db-master-pvc
kubectl delete pv db-master-pv
```

## Expose the MySQL TCP ports through the ingress
```shell
# Enable the ingress addon if it not be enabled
minikube addons enable ingress

# Patch the tcp services configmap to expose the MySQL TCP port
kubectl patch configmap tcp-services -n ingress-nginx \
  --patch-file k8s/patch/ingress-nginx-tcp-configmap-patch.yaml

kubectl patch deployment ingress-nginx-controller -n ingress-nginx \
  --patch-file k8s/patch/ingress-nginx-controller-patch.yaml

# Test the tcp port
telnet $(minikube ip) 3306
```
