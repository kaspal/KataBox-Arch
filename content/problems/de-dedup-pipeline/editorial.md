# Editorial

## Where the load goes

40 events a second is little work for the ingest service and for Kafka. The work is the writes: each event becomes one row in the warehouse, and a write takes about 300 ms. At 40 events a second that is 40 × 0.3 = 12 writes in flight on average; in the retry storm, at 120 events a second, it is 36, plus the backlog the crash leaves. A Stream Processor slot waits for its write, so the processor needs that many slots, the Stream that many partitions (one event per partition at a time), and the warehouse that many write places (8 per node).

Then there is the crash, and the one number that cannot be anything but zero. A stream processor saves its progress (its place in the stream and its state) at every checkpoint. When it dies, it restarts from the last checkpoint and processes everything after it again. Those events were already written once, so without protection they reach the ledger a second time: a customer billed twice. A shorter Checkpoint interval shrinks how much is replayed, but it never reaches zero, because the crash can come a moment after any checkpoint. Only the sink can make the replay harmless: a write keyed by the event id (an upsert, or the warehouse dropping a row it already has) turns "at least once" delivery into an exactly-once effect.

## The reference design

2 Go App Servers write the events to an Apache Kafka Stream with 64 partitions. An Apache Flink Stream Processor with 2 instances and Concurrency per instance 32 (64 slots, one per partition) reads it, with Window `none` and Checkpoint interval 10 s. It writes each event to a ClickHouse Data Warehouse with 8 instances (64 writes at a time, about 210 a second) and "Idempotent writes (dedup)" on. This costs about $2,433 per month: $140 for the App Servers, $153 for Kafka, $140 for Flink and $2,000 for ClickHouse.

On a normal day the p95 from event to ledger is about 0.7 s, and the processor and the warehouse are each about 21% busy.

"The stream processor crashes" kills the Stream Processor 50 seconds in, for 10 seconds, in a run of 120 seconds. The events wait in Kafka and nothing is lost. On restart the processor replays from its last checkpoint and writes those events again, but the warehouse deduplicates them, so the ledger counts no duplicates. The p95 rises to about 6.2 s while the backlog drains.

"A retry storm" sends 3 times the events (120 a second) from 30 s to 60 s, and kills the processor at 45 s, in the middle of it. The replay is bigger, and again the sink absorbs it: no duplicates. The backlog of the storm and the crash together takes a while to drain, and the p95 reaches about 21.9 s, under the 30 s limit. A smaller design with 6 warehouse nodes, 48 slots and 48 partitions keeps up on a normal day and through the crash, but not here: its p95 climbs to about 40 s.

The Checkpoint interval of 10 s does not change the count of duplicates here (dedup makes it zero at any interval). It sets how much work a crash redoes, and in a real cluster how long recovery takes and how much a checkpoint costs; 10 s is a usual middle ground.

For the cost challenge, the search found a design at about 53% of the reference's cost ($1,293): one App Server, one Flink instance of 32 slots and 4 ClickHouse nodes, still with dedup. The target is 55% of the reference's cost. It works because the storm lasts only 30 s and Kafka holds the backlog; it has far less room for a longer storm.

## Why the first try breaks

A common first try has the same shape and the same capacity, and knows about crashes: "checkpoint often and little is replayed". It sets the Checkpoint interval to 5 s and leaves "Idempotent writes (dedup)" off. It costs the same $2,433 and passes "A normal day" with a p95 of about 0.7 s: with no crash there is nothing to replay.

In "The stream processor crashes" the processor restarts from a checkpoint a few seconds old and writes those events again: about 186 rows in the ledger twice (verdict "Dropped requests"; the limit is 0). In the retry storm the processor is busier when it dies, so the same few seconds hold more events: about 599 duplicates.

Checkpointing more often does not fix it. At 1 s the crash still leaves about 32 duplicates and the storm about 115. At 60 s (the default) it is about 1,955 and 2,954, because the crash comes before the first checkpoint and everything since the start is replayed. The interval sets how many customers are billed twice, never whether.

The point is this: a stream processor with checkpoints delivers each record at least once, and after a crash some records more than once. "Exactly once" is not a property of the delivery but of the effect: make the sink idempotent, keyed by an id that the event carries from the start, and the replays change nothing. Then size the pipeline so the replay and the storm drain in time.

## What the simulator leaves out

The simulator does not track event ids. Producer retries in the real world resend the same event, and those resends are duplicates too; the same idempotent sink, keyed by the event id, absorbs them. Here a retry storm is modelled honestly as what the simulator can count: three times the traffic (more load) plus a crash during it. Duplicates are counted only from a stream processor's replay after a crash. Exactly-once transactional sinks (Flink's two-phase commit, Kafka transactions) are not simulated; dedup in the sink is the only way to reach zero here. Dedup is modelled as free: real upserts or deduplicating table engines cost some write speed and storage, and ClickHouse's ReplacingMergeTree removes repeats only when it merges parts. Checkpoints cost nothing here, and recovery after a restart (restoring state, rewinding the stream) takes no time. Writing one row per event to a warehouse is a simplification; real pipelines batch their writes, which changes the sizing but not the lesson.

## Things to try

- Turn dedup off in the reference and run the crash with a Checkpoint interval of 1 s, 10 s and 60 s. The duplicates shrink with the interval, but never reach zero.
- Give the reference 6 warehouse nodes, 48 slots and 48 partitions and run the retry storm. It passes the crash, but the storm's backlog pushes the p95 to about 40 s.
- Build the challenge design (one App Server, one Flink instance with 32 slots, 4 ClickHouse nodes) and see how close its storm p95 comes to the limit.
