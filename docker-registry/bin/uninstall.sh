#!/usr/bin/env bash

helm delete docker-registry --namespace "${K8S_NAMESPACE:-default}"
