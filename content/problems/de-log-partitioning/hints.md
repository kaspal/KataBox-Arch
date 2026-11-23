## 1

A search for the last 24 hours needs 0.1 GB. How much does the warehouse have to read to find it, and how long does that hold a slot? Two queries arrive every second.

## 2

A table can be split by the column queries filter on, so the warehouse skips the parts it doesn't need. Once searches are fast, count the slots again: a year-long query still reads everything and holds its slot for over half a minute. What happens to the slots when an audit runs many of them, or when everyone searches at once?

## 3

Wire Client (Timeout 60,000 ms) → 2 Go App Servers → Google BigQuery (1 instance, Concurrency per instance 32) → Apache Iceberg with Partitioning By day, Compaction Daily and Stored data (TB) 0.04.
