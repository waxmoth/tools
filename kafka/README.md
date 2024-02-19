# K8S Kafka Service

The Kafka service run in the Kubernetes

## Dependencies

* [Minikube](https://minikube.sigs.k8s.io/docs/) for local development

* Create k8s pods and services

```shell
kubectl apply -f k8s/
```

* Get pods list

```shell
kubectl get pods -n kafka
```

## Expose the port through the ingress

```shell
# Enable the ingress addon if it not be enabled
minikube addons enable ingress

# Patch the tcp services configmap to expose the Kafka TCP port
kubectl patch configmap tcp-services -n ingress-nginx \
  --patch-file k8s/patch/ingress-nginx-tcp-configmap-patch.yaml

kubectl patch deployment ingress-nginx-controller -n ingress-nginx \
  --patch-file k8s/patch/ingress-nginx-controller-patch.yaml

# Test the tcp port
telnet $(minikube ip) 9092

```

## How to test the Kafka

```shell
kubectl exec -n kafka svc/kafka-service -it -- /bin/bash

# Get the topics
kafka-topics.sh --bootstrap-server localhost:9092 --list

# Create one topic
kafka-topics.sh --bootstrap-server localhost:9092 --create \
  --topic TOPIC_NAME --partitions 1 --replication-factor 1

# Get Topic offsets
kafka-run-class.sh kafka.tools.GetOffsetShell \
  --broker-list ${KAFKA_ADVERTISED_LISTENERS} \
  --topic TOPIC_NAME

# Testing producer
kafka-verifiable-producer.sh --broker-list ${KAFKA_ADVERTISED_LISTENERS} \
  --topic test --max-messages 2

# Testing consumer
kafka-verifiable-consumer.sh --broker-list ${KAFKA_ADVERTISED_LISTENERS} \
  --topic test \
  --group-id default_group --max-messages 2

# Check the topic details
kafka-topics.sh --bootstrap-server localhost:9092 --describe --topic test

# Manually consume a topic
kafka-console-consumer.sh --bootstrap-server localhost:9092 --topic test --from-beginning

# Change the topic retention time. e.g. delete messages after 3 days
kafka-configs.sh --bootstrap-server localhost:9092 --alter \
  --entity-type topics --entity-name test \
  --add-config retention.ms=259200000,cleanup.policy=delete

kafka-configs.sh --bootstrap-server localhost:9092 --describe \
  --entity-type topics --entity-name test
```

## Testing the kafka service by [KafkaCat](https://docs.confluent.io/platform/current/tools/kafkacat-usage.html)

```shell
# Get minikube ip
$(minikube ip)

# Update the /etc/hosts file
# E.g. 192.168.49.2 local-kafka.test

# Testing produce a message
echo "hello world!" | kafkacat -P -b $(minikube ip):9092 -t test

# Testing consume the topic
kafkacat -C -b $(minikube ip):9092 -t test
```

## Clean the services and pods

```shell script
kubectl delete -f k8s/
```
