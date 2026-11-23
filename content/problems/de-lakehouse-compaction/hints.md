## 1

Work out how long one query takes on its own, with nobody else in the warehouse. How many files does it open for a normal day, and for a sale day? If that alone is close to the limit, can more warehouse instances help?

## 2

A query's time is its overhead, plus the data it reads at the scan rate, plus about 20 ms for every 8 files it opens. Small files make the last part dominate: 2 GB in 4 MB files is 512 files. Merge them into big files in the background, so each query opens a handful instead of hundreds. Then check that the warehouse has enough slots for four times the queries.

## 3

Keep the Lakehouse Table (Delta Lake) partitioned by day, set Compaction to Hourly and the target file size to 256 MB. Read it from a Data Warehouse (Databricks SQL) with 4 instances, a concurrency of 8 and a scan rate of 5 GB/s, with the result cache on. Feed the table through an Apache Kafka stream into 2 Apache Flink instances, behind an AWS ALB and 2 Go App Servers, and set the Client's timeout to 60,000 ms.
