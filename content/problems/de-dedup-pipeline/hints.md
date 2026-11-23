## 1

The limit on duplicate rows is zero, not "a few". Ask what the stream processor does when it restarts after a crash, and what the ledger sees. Then ask which setting makes that harmless instead of just smaller. Also work out how many writes are in flight at once when every write takes 300 ms, and in a storm of three times the events.

## 2

On a crash the processor goes back to its last checkpoint and writes every event since then again. A shorter Checkpoint interval only makes that window smaller: even at 1 s some events land twice. Writes keyed by the event id make a second write of the same event change nothing, so the replay costs work but no duplicates. For capacity: 120 events/s × 0.3 s is 36 writes in flight, before the backlog the crash leaves behind; give the processor slots, the Stream partitions and the warehouse nodes for more than that.

## 3

Keep 2 Go App Servers and an Apache Kafka Stream with 64 partitions. Use an Apache Flink Stream Processor with 2 instances, Concurrency per instance 32 and Checkpoint interval 10 s. Write to a ClickHouse Data Warehouse with 8 instances and "Idempotent writes (dedup)" on.
