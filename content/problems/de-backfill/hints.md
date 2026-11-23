## 1

The warehouse can load only so many batches a second. When the backfill asks for more than the room it was paced for, somebody has to wait. Who should it be, and what decides the order in which the Stream hands out batches?

## 2

A Stream is first in, first out: a live batch waits behind every backfill batch already in it, so a fast backfill makes the dashboards stale. Turn on "Live before bulk (priority)" on the Stream, so a live batch goes ahead of every queued backfill batch; the backfill piles up in the Stream, where waiting costs nothing. Priority can't help when the live batches alone are more than the warehouse can load: size the warehouse for the live burst, and give the Stream enough partitions and the Stream Processor enough slots to keep every warehouse slot busy.

## 3

Put an AWS ALB in front of 2 Go App Servers that publish to an Apache Kafka Stream with 32 Partitions and "Live before bulk (priority)" on. An Apache Flink Stream Processor with 2 instances and a Concurrency per instance of 16 consumes it and loads each batch into a Snowflake Data Warehouse with 3 Instances.
