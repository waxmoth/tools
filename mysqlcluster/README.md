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
kubectl create -f k8s/mysql-configmap.yaml

kubectl create -f k8s/mysql-master-rc.yaml
kubectl create -f k8s/mysql-master-sv.yaml

kubectl create -f k8s/mysql-slaver-rc.yaml
kubectl create -f k8s/mysql-slaver-sv.yaml
# Update the service?
# kubectl replace -f k8s/mysql-master-rc.yaml
```

* Check services
```bash
kubectl get svc
```

* Get pods list
```bash
kubectl get pods -o wide
```

* Delete pod
```bash
kubectl delete pods mysql-master-*
```

* Check the DB in the container
```bash
kubectl exec $(kubectl get pods|grep mysql-master|tail -n 1|awk '{print $1}') -it -- /bin/bash
> export MYSQL_PWD=${MYSQL_ROOT_PASSWORD}
> mysql -uroot
```
