# Editorial

## Where the load goes

Two flows share this design. Click batches arrive at 180 a second and go through the Stream and the Stream Processor into the table. Chart refreshes arrive at 20 a second, and each one is a scan that holds a warehouse slot until it ends.

The scan is where the time goes. The table holds 730 GB, a year of clicks. Without partitions, a refresh reads all of it: at 2 Gbps that is about six minutes. Partitioned by day, it reads only the last day, about 2 GB, which takes 1 s plus the warehouse's 300 ms of overhead. Then come the files. Each one costs 20 ms to open, eight at a time. In 2 MB files, the day is 1,024 files, which adds 2.56 s. In 256 MB files, it is 8 files, about 20 ms. So the same 2 GB answer takes about 3.9 s from small files and about 1.3 s from compacted ones.

That time is also how long the refresh holds its slot. At 20 refreshes a second, small files keep about 77 slots busy and compacted files about 26. On a launch day the traffic is five times higher: about 386 slots against about 132.

## The reference design

An AWS ALB sends the traffic to 6 Go App Servers. They publish click batches to Apache Kafka with 96 partitions. An Apache Flink Stream Processor with 20 instances and the default concurrency of 4 (80 slots) consumes them and writes them to an Apache Iceberg Lakehouse Table partitioned by day, with hourly compaction, a target file size of 256 MB, and 0.73 TB stored. Chart refreshes go from the App Servers to a Snowflake Data Warehouse with 5 instances, a concurrency of 32 (160 slots) and a scan rate of 2 Gbps, which reads the table over a Query edge. This costs about $16,615 per month: $14,600 for Snowflake, $1,400 for Flink, $420 for the App Servers, $153 for Kafka, $25 for the ALB and $17 for the table's storage. A normal day has a p95 of about 1.34 s, with no errors and no lost batches. The warehouse is about 17% busy and Flink about 14%.

The scan rate is the warehouse's size, and it is billed: an instance at the default 1 Gbps costs $1,460 a month, one at 2 Gbps twice that, $2,920. A faster scan shortens only the reading, not the overhead or the file opening. At 1 Gbps a compacted refresh takes about 2.3 s, and 8 instances ($11,680) carry the launch day; at 2 Gbps, 4 instances do it for the same price, with the launch day just past their 128 slots. The reference takes a fifth instance at 2 Gbps, so the launch day has room to spare. Flink is billed the same way: an instance has 4 slots, and more slots per instance cost more in proportion, so 80 slots cost $1,400 however you split them.

"A launch day brings 5× the traffic" multiplies all traffic by five from 20 seconds in, for 20 seconds: 900 batches and 100 refreshes a second. Each refresh still takes about 1.3 s, so about 132 of the 160 warehouse slots are busy, and nothing queues. Flink needs about 56 of its 80 slots, and Kafka's 96 partitions let all of them work at once. The p95 stays at about 1.34 s, with no errors.

"The stream processor is down for 20 s" kills Flink 20 seconds in, for 20 seconds, in a run of 90 seconds. This test allows a p95 of 30 s, because clicks may arrive late, but none may be lost. The App Servers keep publishing, and Kafka keeps the 3,600 batches that arrive meanwhile (its retention is 168 hours). When Flink comes back, its spare slots drain the backlog in a few seconds. The batches that waited push the p95 to about 15.6 s, about half the limit, and none is lost. The run also reports 3,591 duplicates: the batches Flink processed since its last checkpoint are processed again on restart. This problem does not judge duplicates. To avoid them, turn on Idempotent writes (dedup) on the table: writes then merge by key, the replays add no rows, and the run reports no duplicates, at no cost here.

The cost challenge search found no design at 80% of this cost or less (its best was 98%), so this problem has no cost challenge.

## Why the first try breaks

A common first try is the same design, but the table keeps the files Flink writes: compaction is off, so every file is about 2 MB. It costs the same $16,615, because compaction is not billed in the simulator. It passes "A normal day" with a p95 of about 3.8 s, just under the 4 s limit: each refresh opens 1,024 files, and the warehouse is about 48% busy.

On the launch day, 100 refreshes a second each hold a slot for about 3.9 s, which needs about 386 slots. The warehouse has 160. It finishes about 41 refreshes a second, so the rest queue, and the queue grows by about 59 a second. Refreshes that wait too long fail: about 7.4% of all requests fail, about three quarters of the chart refreshes in the test. The click batches never notice, because they don't touch the warehouse.

Buying more warehouse instances is the other way out, at $2,920 each. With 10 the launch day still fails (5.2% errors); with 12 it passes, but the design costs about $37,055, over the $30,000 budget. Compaction makes each scan about three times shorter instead, at no cost here. The point is this: a streaming writer makes many small files, and every query pays to open each one. Compact them, partition so a query reads only what it needs, and the same warehouse answers far more queries.

## What the simulator leaves out

Compaction is free and instant here: a compacted table is treated as made only of target-size files. A real compaction job costs compute, runs on a schedule, and leaves the newest files small until it runs. With daily compaction, today's files would stay small all day, and a query over the last day would still open them; that is why the reference compacts hourly. Snowflake bills in credits per warehouse size; here an instance costs $1,460 a month while it is busy at the default scan rate, more in proportion for a faster scan, and the prices are the registry's teaching figures, not quotes. A real Flink sink to Iceberg commits its files at each checkpoint, so a restart rewrites uncommitted files instead of adding rows twice; the simulator counts the replays as duplicates unless the table has Idempotent writes (dedup) on, and this problem does not judge them. Here clicks become visible to queries as soon as they are written, not at the next commit. Query planning, metadata reads, data skew between customers and object-store request costs are not simulated.

## Things to try

- Set compaction to none and run the launch day. Then add warehouse instances until it passes, and compare the cost.
- Set partitioning to none and run a normal day. Count how much of the table one refresh scans.
- Lower Kafka to 12 partitions and run the launch day. Watch how long the click batches take to land.
