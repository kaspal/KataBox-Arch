## 1

Each change has to wait for a warehouse write of about 300 ms. How many writes must be in flight at once to keep up with 12 changes a second? And with 36 a second, or when every write takes 1.8 s?

## 2

Three cards each cap how many changes are in flight: the Stream hands out one record per partition at a time, the Stream Processor holds a slot for the whole write, and the Data Warehouse runs a fixed number of writes per instance. What gets through a second is the smallest of the three divided by the time of one write. Size all three for the slow warehouse, not for the average day, and let the Stream hold whatever is left over until it can be written.

## 3

Wire PostgreSQL with a Publish edge (change data capture) to an Apache Kafka Stream with 24 partitions. Consume it with Apache Flink, 2 instances with a concurrency of 12 (24 slots), writing into Snowflake with 3 instances (8 writes each, 24 in all). Keep an AWS ALB and 2 Go App Servers in front of the database.
