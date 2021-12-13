# Docker MySQLTuner
Use docker to run the [MySQLTuner](https://github.com/major/MySQLTuner-perl)

## Usage
Build image
```bash
docker build --build-arg version=1.8.3 . -t mysqltuner
```

Run to check the MySQL
```bash
docker run --rm -it --net="host" mysqltuner --user=${DB_USER} --pass=${DB_PASSWORD} --host=${HOST} --port=${PORT}
# Add --forcemem parameter if checking the remote DB, the parameter unit is Mb. E.g. --forcemem=2048
```
