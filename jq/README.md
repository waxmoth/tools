# Docker JQ
Use docker to run the [JQ](https://jqlang.github.io/jq/)

## Usage
Build image
```bash
docker build --build-arg version=1.6-r3 . -t jq
```

Get helps
```shell
docker run --rm jq jq --help
```

Test decode a JSON from a string
```shell
docker run --rm jq echo '{"foo": 0}' | jq .
```

Test load JSON from host file
```shell
PATH_OF_JSON_FILE=$(pwd)/example.json
docker run --rm -v $PATH_OF_JSON_FILE:/app/test.json jq sh -c "jq -rC '.' < /app/test.json"
```

## For more documents
- [Manual of JQ](https://jqlang.github.io/jq/manual/)
