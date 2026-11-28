## 1

A chart refresh holds a warehouse slot for as long as its scan takes, and the warehouse's instances share their scan rate among the scans running at once. How much of the table does one refresh read, and in how many files? Then picture five times as many refreshes at once.

## 2

Partition the table by hour, so a refresh reads about 85 MB instead of a whole day (2 GB) or the whole year. Then look at the files: 85 MB in 50 KB files is about 1,707 files to open, and opening them takes longer than the 4 s limit. Compaction rewrites them into a few large files. On the ingest side, a launch day sends five times the batches, so give the Stream Processor slots to spare and the Stream enough partitions to feed them; that same headroom drains the backlog after the processor comes back.

## 3

Put an AWS ALB in front of 6 Go App Servers that publish to Apache Kafka with 96 partitions. An Apache Flink Stream Processor with 20 instances (the default concurrency of 4) writes to an Apache Iceberg Lakehouse Table partitioned by hour, with hourly compaction and a target file size of 256 MB. A Snowflake Data Warehouse with 5 instances, a concurrency of 32 and a scan rate of 2 Gbps reads that table over a Query edge.
