#!/usr/bin/env bash

helm delete eck --namespace "${K8S_NAMESPACE:-default}"
