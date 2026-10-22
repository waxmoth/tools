#!/usr/bin/env bash

helm delete redmine --namespace "${K8S_NAMESPACE:-default}"
