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
