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

* Check the cron job
```bash
kubectl get cronjob app-cronjob-mysqlkiller
```

> The result should similar as:
```text
NAME                      SCHEDULE      SUSPEND   ACTIVE   LAST SCHEDULE   AGE
app-cronjob-mysqlkiller   */5 * * * *   False     0        <none>          57s
```

* Verifier it can kill the some long run query
> Create a sleep query
```sql
SELECT SLEEP(3600);
```

> Check the jobs
```bash
kubectl get jobs
```

> The output should be similar as:
```text
NAME                               COMPLETIONS   DURATION   AGE
app-cronjob-mysqlkiller-27756840   1/1           4s         14m
app-cronjob-mysqlkiller-27756845   1/1           4s         9m57s
app-cronjob-mysqlkiller-27756850   1/1           4s         4m57s
```

> Get logs for one job
```bash
kubectl logs job/app-cronjob-mysqlkiller-27756845
```

> Get the latest logs
```bash
kubectl get jobs|tail -n -1|awk '{print "job/"$1}'|xargs kubectl logs
```

> The output should be similar as:
```text
2022-10-10 14:32:12|Kill sleeping and long run DB connections for user: "root","reader"
2022-10-10 14:32:12|Killed thread: 69|Query: SELECT SLEEP(3600)
2022-10-10 14:32:12|Killed thread: 72|Query: NULL
2022-10-10 14:32:12|Done!
```
