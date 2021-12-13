# Docker MySQLKiller
A tool to kill sleep or long MySQL connections

## Usage
Build image
```bash
docker build . -t mysqlkiller
```

Run to check the MySQL
```bash
docker run --rm -v $(pwd):/app -w /app --net="host" --env-file .env mysqlkiller ./bin/mysql_killer.sh
```

## Build and run cron job in the K8s
* Build image in K8s Cluster
```bash
eval $(minikube docker-env)
docker build . -t mysqlkiller
```

* Deploy the cron job
```bash
kubectl apply -f k8s/
```
