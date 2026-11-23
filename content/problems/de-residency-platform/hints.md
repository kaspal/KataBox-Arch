## 1

Nothing in the design can tell an EU event from a US one, so every place an event can land must be fine for both. List every card that keeps an event: the stream, the consent table, the lake. Then ask where each of them goes on a bad day, when a Region is lost or a primary fails.

## 2

A standby is part of the pipeline. If ingestion fails over to a US Region, or the consent table's replica sits in the US, the first failure moves EU data out of the EU. Run the whole pipeline in two Regions that are both tagged `eu`, and keep the consent table's replica in the other EU Region. Only the warehouse with the aggregates belongs in the US. Each EU Region must carry the whole peak alone.

## 3

A Client with 2 Retries, a Retry backoff of 200 ms and Full backoff jitter calls a Route 53 DNS card (TTL 30 s, Round robin) that points at two Regions tagged `eu`, eu-west and eu-central. Each has an AWS ALB, 2 Go App Servers, an Apache Kafka Stream with 256 Partitions, an Apache Flink Stream Processor with 2 Instances and a Concurrency per instance of 128, a DynamoDB NoSQL Store and an Apache Iceberg Lakehouse Table (Partitioning By day, Compaction Hourly). The DynamoDB in eu-west is wired with Replicate to the one in eu-central. A Region tagged `us` holds an AWS ALB, 2 Go App Servers and a Snowflake Data Warehouse for the dashboard; the Client calls that ALB directly.
