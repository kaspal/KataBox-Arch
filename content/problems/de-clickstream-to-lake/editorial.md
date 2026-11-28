# Editorial

## Where the load goes

Two flows share this design. Click batches arrive at 180 a second and go through the Stream and the Stream Processor into the table. Chart refreshes arrive at 20 a second, and each one is a scan of the last hour of clicks that holds a warehouse slot until it ends.

The scan is where the time goes. The table holds 730 GB, a year of clicks. Without partitions, a refresh reads all of it: at 2 Gbps that is about six minutes. Partitioned by day, it reads the whole last day, about 2 GB. Partitioned by hour, it reads only the last hour, about 85 MB: about 42 ms of reading plus the warehouse's 300 ms of overhead. Then come the files. Each one costs 20 ms to open, eight at a time. In 50 KB files, the hour is about 1,707 files, which adds about 4.27 s. In 256 MB files, it is a single file, a few milliseconds. So the same answer takes about 4.6 s from small files and about 0.35 s from compacted ones.

The warehouse's instances share their scan rate among the queries running at once: 5 instances at 2 Gbps read 10 GB a second in all, however many queries share it. That is why the hour partitions matter. At 20 refreshes a second, hour partitions read about 1.7 GB a second (17% of it), and on a launch day about 8.3 GB a second (83%). Day partitions would need 40 GB a second on a normal day, four times what the warehouse has.

## The reference design

An AWS ALB sends the traffic to 6 Go App Servers. They publish click batches to Apache Kafka with 96 partitions. An Apache Flink Stream Processor with 20 instances and the default concurrency of 4 (80 slots) consumes them and writes them to an Apache Iceberg Lakehouse Table partitioned by hour, with hourly compaction, a target file size of 256 MB, and 0.73 TB stored. Chart refreshes go from the App Servers to a Snowflake Data Warehouse with 5 instances, a concurrency of 32 (160 slots) and a scan rate of 2 Gbps, which reads the table over a Query edge. This costs about $16,615 per month: $14,600 for Snowflake, $1,400 for Flink, $420 for the App Servers, $153 for Kafka, $25 for the ALB and $17 for the table's storage. A normal day has a p95 of about 349 ms, with no errors and no lost batches. The warehouse is about 4% busy and Flink about 14%.

The scan rate is the warehouse's size, and it is billed: an instance at the default 1 Gbps costs $1,460 a month, one at 2 Gbps twice that, $2,920. What you buy is the total rate, instances × scan rate, and the launch day needs about 8.3 GB a second of it. With 4 instances at 2 Gbps (8 GB a second, $13,695 in all) the launch day still passes, with a p95 of about 0.61 s, because it lasts only 20 seconds; the reference takes a fifth instance so the launch day has room to spare. Slots are not the limit here: a refresh holds one for about 0.35 s, so the launch day keeps about 35 of the 160 busy. Flink is billed the same way: an instance has 4 slots, and more slots per instance cost more in proportion, so 80 slots cost $1,400 however you split them.

"A launch day brings 5× the traffic" multiplies all traffic by five from 20 seconds in, for 20 seconds: 900 batches and 100 refreshes a second. The refreshes read about 8.3 of the warehouse's 10 GB a second, so nothing queues. Flink needs about 56 of its 80 slots, and Kafka's 96 partitions let all of them work at once. The p95 stays at about 356 ms, with no errors.

"The stream processor is down for 20 s" kills Flink 20 seconds in, for 20 seconds, in a run of 90 seconds. This test allows a p95 of 30 s, because clicks may arrive late, but none may be lost. The App Servers keep publishing, and Kafka keeps the 3,600 batches that arrive meanwhile (its retention is 168 hours). When Flink comes back, its spare slots drain the backlog in a few seconds. The batches that waited push the p95 to about 15.6 s, about half the limit, and none is lost. The run also reports 3,591 duplicates: the batches Flink processed since its last checkpoint are processed again on restart. This problem does not judge duplicates. To avoid them, turn on Idempotent writes (dedup) on the table: writes then merge by key, the replays add no rows, and the run reports no duplicates, at no cost here.

For the cost challenge, a proof design comes in at about 22% of the reference's cost ($3,605), under the 50% target. It keeps the hour partitions and the compaction but runs 1 App Server, 12 Flink instances and 5 ClickHouse nodes at 2 Gbps ($2,500) instead of Snowflake. On the launch day its single App Server is nearly full and the p95 reaches about 3.5 s, close to the limit.

## Why the first try breaks

A common first try is the same design, but the table keeps the files Flink writes: compaction is off, so every file is about 50 KB. It costs the same $16,615, because compaction is not billed in the simulator. It fails even "A normal day": each refresh opens about 1,707 files and takes about 4.6 s on its own, so the p95 is about 4.58 s, over the 4 s limit. The warehouse is about 57% busy, mostly with queries waiting on their files.

On the launch day, 100 refreshes a second each hold a slot for about 4.6 s, which needs about 460 slots. The warehouse has 160, so the rest queue, and many refreshes pass the dashboard's 5 s timeout: about 7.8% of all requests fail. The click batches never notice, because they don't touch the warehouse.

Buying more warehouse instances does not help. With 12 instances at 2 Gbps the design costs about $37,055, over the $30,000 budget, and a normal day still has a p95 of about 4.58 s: more instances run more queries at once, but each query still opens the same 1,707 files. Compaction makes each refresh about thirteen times shorter instead, at no cost here. The point is this: a streaming writer makes many small files, and every query pays to open each one. Compact them, partition so a query reads only what it needs, and the same warehouse answers far more queries.

## What the simulator leaves out

Compaction is free and instant here: a compacted table is treated as made only of target-size files. A real compaction job costs compute, runs on a schedule, and leaves the newest files small until it runs. With daily compaction, today's files would stay small all day, and a query over the last hour would still open them; that is why the reference compacts hourly. Snowflake bills in credits per warehouse size; here an instance costs $1,460 a month while it is busy at the default scan rate, more in proportion for a faster scan, and the prices are the registry's teaching figures, not quotes. A real Flink sink to Iceberg commits its files at each checkpoint, so a restart rewrites uncommitted files instead of adding rows twice; the simulator counts the replays as duplicates unless the table has Idempotent writes (dedup) on, and this problem does not judge them. Here clicks become visible to queries as soon as they are written, not at the next commit. Query planning, metadata reads, data skew between customers and object-store request costs are not simulated.

## Things to try

- Set compaction to none and run a normal day. Then add warehouse instances and watch the p95 stay where it is.
- Set partitioning to day and run a normal day. Work out how many GB a second the refreshes ask of the warehouse.
- Lower Kafka to 12 partitions and run the launch day. Watch how long the click batches take to land.
