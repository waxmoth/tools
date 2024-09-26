#!/usr/bin/env bash

helm delete gitlab --namespace "${K8S_NAMESPACE:-default}"
