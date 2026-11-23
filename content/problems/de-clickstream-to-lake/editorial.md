# Editorial

## Where the load goes

Two flows share this design. Click batches arrive at 180 a second and go through the Stream and the Stream Processor into the table. Chart refreshes arrive at 20 a second, and each one is a scan that holds a warehouse slot until it ends.

The scan is where the time goes. The table holds 730 GB, a year of clicks. Without partitions, a refresh reads all of it: at 4 Gbps that is about three minutes. Partitioned by day, it reads only the last day, about 2 GB, which takes 0.5 s plus the warehouse's 300 ms of overhead. Then come the files. Each one costs 20 ms to open, eight at a time. In 2 MB files, the day is 1,024 files, which adds 2.56 s. In 256 MB files, it is 8 files, about 20 ms. So the same 2 GB answer takes about 3.4 s from small files and about 0.82 s from compacted ones.

That time is also how long the refresh holds its slot. At 20 refreshes a second, small files keep about 67 slots busy and compacted files about 16. On a launch day the traffic is five times higher: about 336 slots against about 82.

## The reference design

An AWS ALB sends the traffic to 6 Go App Servers. They publish click batches to Apache Kafka with 96 partitions. An Apache Flink Stream Processor with 4 instances and a concurrency of 20 (80 slots) consumes them and writes them to an Apache Iceberg Lakehouse Table partitioned by day, with hourly compaction, a target file size of 256 MB, and 0.73 TB stored. Chart refreshes go from the App Servers to a Snowflake Data Warehouse with 4 instances, a concurrency of 32 (128 slots) and a scan rate of 4 Gbps, which reads the table over a Query edge. This costs about $6,735 per month: $5,840 for Snowflake, $420 for the App Servers, $280 for Flink, $153 for Kafka, $25 for the ALB and $17 for the table's storage. A normal day has a p95 of about 834 ms, with no errors and no lost batches. The warehouse is about 13% busy and Flink about 14%.

"A launch day brings 5× the traffic" multiplies all traffic by five from 20 seconds in, for 20 seconds: 900 batches and 100 refreshes a second. Each refresh still takes about 0.82 s, so about 82 of the 128 warehouse slots are busy, and nothing queues. Flink needs about 56 of its 80 slots, and Kafka's 96 partitions let all of them work at once. The p95 stays at about 834 ms, with no errors.

"The stream processor is down for 20 s" kills Flink 20 seconds in, for 20 seconds, in a run of 90 seconds. This test allows a p95 of 30 s, because clicks may arrive late, but none may be lost. The App Servers keep publishing, and Kafka keeps the 3,600 batches that arrive meanwhile (its retention is 168 hours). When Flink comes back, its spare slots drain the backlog in a few seconds. The batches that waited push the p95 to about 15.6 s, about half the limit, and none is lost. The run also reports 3,591 duplicates: the batches Flink processed since its last checkpoint are processed again on restart. This problem does not judge duplicates (see below).

The cost challenge search found no design at 80% of this cost or less (its best was 95%), so this problem has no cost challenge.

## Why the first try breaks

A common first try is the same design, but the table keeps the files Flink writes: compaction is off, so every file is about 2 MB. It costs the same $6,735, because compaction is not billed in the simulator. It passes "A normal day" with a p95 of about 3.4 s, under the 4 s limit: each refresh opens 1,024 files, and the warehouse is about 52% busy.

On the launch day, 100 refreshes a second each hold a slot for about 3.4 s, which needs about 336 slots. The warehouse has 128. It finishes about 38 refreshes a second, so the rest queue, and the queue grows by about 60 a second. Refreshes that wait too long fail: about 7.4% of all requests fail, about three quarters of the chart refreshes in the test. The click batches never notice, because they don't touch the warehouse.

Buying more warehouse instances is the other way out, at $1,460 each. With 8 the launch day still fails; with 10 it passes, but the design costs about $15,495, over the $12,000 budget. Compaction makes each scan four times shorter instead, at no cost here. The point is this: a streaming writer makes many small files, and every query pays to open each one. Compact them, partition so a query reads only what it needs, and the same warehouse answers far more queries.

## What the simulator leaves out

Compaction is free and instant here: a compacted table is treated as made only of target-size files. A real compaction job costs compute, runs on a schedule, and leaves the newest files small until it runs. With daily compaction, today's files would stay small all day, and a query over the last day would still open them; that is why the reference compacts hourly. Snowflake bills in credits per warehouse size; here an instance costs $1,460 a month while it is busy, and the prices are the registry's teaching figures, not quotes. A real Flink sink to Iceberg commits its files at each checkpoint, so a restart rewrites uncommitted files instead of adding rows twice; the simulator counts the replays as duplicates, and this problem does not judge them. Here clicks become visible to queries as soon as they are written, not at the next commit. Query planning, metadata reads, data skew between customers and object-store request costs are not simulated.

## Things to try

- Set compaction to none and run the launch day. Then add warehouse instances until it passes, and compare the cost.
- Set partitioning to none and run a normal day. Count how much of the table one refresh scans.
- Lower Kafka to 12 partitions and run the launch day. Watch how long the click batches take to land.
