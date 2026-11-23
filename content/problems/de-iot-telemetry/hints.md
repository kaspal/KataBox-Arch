## 1

Compare what one Time-Series Database node writes per second with what arrives, on a normal day and when a whole area uploads what it kept offline. Then count the series each node holds, and multiply by 10.

## 2

A database that is full refuses the writes; a stream in front of it keeps them and hands them over at a pace the database can take. Workers reading the stream set that pace, so size them for the database, not for the burst. For the series, more instances help twice: each one adds its points per second, and each holds a smaller share of the series. Plan the instances so that ten times the series still stays under each node's limit.

## 3

Send the readings from the Go App Server (2 instances) to an Apache Kafka Stream with 192 partitions. Six Celery Workers consume it (concurrency 32 per instance, service time 2 ms) and write to an InfluxDB Time-Series Database with 3 instances at 4,000 points per second each. Keep the meters connected to an EMQX IoT Broker with the default 2 instances.
