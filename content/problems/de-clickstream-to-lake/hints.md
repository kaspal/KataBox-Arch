## 1

A chart refresh holds a warehouse slot for as long as its scan takes. How much of the table does one refresh read, and in how many files? Then picture five times as many refreshes at once.

## 2

Partition the table by day, so a refresh reads 2 GB instead of the whole year. Then look at the files: 2 GB in 2 MB files is 1,024 files to open, and opening them takes longer than reading the data. Compaction rewrites them into a few large files. On the ingest side, a launch day sends five times the batches, so give the Stream Processor slots to spare and the Stream enough partitions to feed them; that same headroom drains the backlog after the processor comes back.

## 3

Put an AWS ALB in front of 6 Go App Servers that publish to Apache Kafka with 96 partitions. An Apache Flink Stream Processor with 4 instances and a concurrency of 20 writes to an Apache Iceberg Lakehouse Table partitioned by day, with hourly compaction and a target file size of 256 MB. A Snowflake Data Warehouse with 4 instances, a concurrency of 32 and a scan rate of 4 Gbps reads that table over a Query edge.
