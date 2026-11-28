## 1

Three kinds of work share one product, and they want very different things: 800 reads a second that must answer in milliseconds, about 200 updates a second that must land within moments, and 2 big training reads a second that can take a second. Which store is built for each? And how many reads a second can a store take when each query costs 300 ms and an instance runs 8 at a time?

## 2

A Cache in front of the warehouse only helps the users who come back within its TTL. Every miss is a 300 ms warehouse query, and more traffic means more misses at once. Put every user's features in an online store that answers each read in a few milliseconds, and let the Stream Processor write updates straight into it; keep the warehouse for the training reads. Then give the stream path headroom: if each update takes 100 ms longer, how many must be in flight to keep up with 200 a second?

## 3

Put an AWS ALB in front of 2 Go App Servers and a Generic Microservice for the training reads. The App Servers read from a ScyllaDB NoSQL Store with 1 partition and publish events to an Apache Kafka Stream with 32 partitions. Apache Flink, 4 instances with a concurrency of 8 (32 slots), consumes the Stream and writes to the same ScyllaDB. The Microservice queries a Snowflake Data Warehouse with 3 instances, enough scanning for three times the training reads.
