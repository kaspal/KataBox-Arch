# Editorial

## Where the time goes

Start with one query, alone in the warehouse. Its time is the query overhead (300 ms), plus the data it reads at the scan rate, plus about 20 ms for every file it opens, 8 files at a time.

A normal day is 250 MB. Partitioned by day, the query reads only today: 250 MB at 5 GB/s is 0.05 s. In 500 KB files, those 250 MB are 512 files, and opening them takes 512 × 20 ms ÷ 8 ≈ 1.28 s. The files cost about twenty-five times the reading. Compacted into 256 MB files, the same day is one file: a few milliseconds. So one query takes about 1.63 s on the small files and 0.35 s compacted.

A sale day is 2.5 GB, and the pipeline wrote it in the same 500 KB files: 5,120 files, 12.8 s just to open them, and about 13.6 s per query. That is over the 8 s limit before any query waits for another. Compacted, it is 10 files and about 0.83 s.

That is the core of the problem: the file count sets a floor under every query, and no number of warehouse instances lowers it. Instances decide how many queries run at once, and how much they read in all: a warehouse's instances share their scan rate among the queries running at once. Slots matter for the month-end review, when four times the queries arrive: about 33 a second. The total scan rate matters for the sale day, when every query reads ten times as much.

## The reference design

An AWS ALB sends the traffic to 2 Go App Servers. Event batches go to an Apache Kafka stream with 48 partitions, consumed by 2 Apache Flink instances (the default concurrency of 4) that write into a Lakehouse Table (Delta Lake), partitioned by day, with Compaction set to Hourly and a target file size of 256 MB. The App Servers send queries to a Data Warehouse (Databricks SQL) with 4 instances, a concurrency of 8, a scan rate of 5 GB/s, the default 300 ms query overhead and the result cache on, which reads the table in place. The Client waits up to 60,000 ms.

It costs about $41,338 per month: $40,880 for the warehouse, $153 for the stream, $140 for the stream processor, $140 for the App Servers, $25 for the ALB and less than $1 for the table's storage. A normal day has a p95 of about 370 ms with no errors, and the warehouse is busy about 7% of the time.

The scan rate is the warehouse's size, and it is billed: an instance at the default 1 GB/s costs $2,044 a month, one at 5 GB/s five times that, $10,220. The sale day sets it: about 8 queries a second, one in five answered by the result cache, each read 2.5 GB, about 16.7 GB a second in all. The 4 instances at 5 GB/s read 20 GB a second together. With 4 instances at 4 GB/s (16 GB a second, $33,162 in all) the queue grows through the sale day and the p95 reaches about 3.4 s; with 3 at 5 GB/s ($31,118) about 5.5 s. Both pass, but with little room; 5 instances at 4 GB/s cost the same as the reference and reach about 1.0 s.

"A sale day: ten times the events, and analysts look at today" sends ten times the event batches, and every query reads the 2.5 GB sale day. The p95 is about 868 ms with no errors and no lost events: each compacted query takes about 0.83 s, and the queries read about 16.7 of the warehouse's 20 GB a second.

"Month-end review: four times the queries" keeps the p95 at about 370 ms. At about 0.35 s a query, 33 queries a second keep about 10 of the 32 slots busy, and the result cache answers one in five almost at once.

For the cost challenge, a proof design comes in at about 10% of the reference's cost ($4,068), under the 50% target. It keeps the compacted table but runs 1 App Server, 1 Flink instance and 3 ClickHouse nodes at 5 GB/s ($3,750) instead of Databricks SQL. On the sale day its 15 GB a second fall behind and the p95 reaches about 5.5 s.

## Why the first try breaks

A common first try is the same design with Compaction set to None: the stream processor's files stay as they were written, and the target file size does nothing. It costs the same $41,338 and passes "A normal day" with a p95 of about 1,635 ms, but each query holds a slot almost five times longer, and the warehouse is busy about 35% of the time.

On the sale day, every query takes about 13.6 s on its own. The p95 climbs to about 50,287 ms and about 2.6% of the requests fail: queries pile up behind each other, and those that wait past the Client's 60 s timeout give up. Adding instances does not help: the same design with 16 warehouse instances ($163,978, more than twice the budget) still has a p95 of about 13,610 ms on the sale day, because that is how long one query takes.

At month end, four times the queries at about 1.6 s each need about 44 slots, and there are 32. The queue grows all through the test and the p95 reaches about 23,695 ms. Here more instances would help, but they would cost far more than compaction does.

The point is this: streaming ingest trades file size for freshness, and every reader pays for it on every query. Compaction is the background work that pays it back once. Size the warehouse for how many queries run at once, but fix the file count first, because it sets how long each one takes.

## What the simulator leaves out

Each test lasts 60 s and stands for an hour of that traffic; the table does not grow during a run. Compaction is free and complete here: the job that merges the files costs nothing and uses no warehouse or stream processor time, and every file is already compacted, while real hourly compaction leaves the newest hour in small files until its next run. Delta Lake's file statistics, data skipping and Z-ordering, which let a query skip files inside a partition, are not simulated. The 20 ms per file and the 8 parallel readers are modelling choices, not measurements. Databricks SQL bills only the seconds it is busy plus its auto-suspend delay, but this steady traffic never lets it suspend, so it bills every hour; a faster scan is a bigger size, billed in proportion. The prices are the registry's teaching prices.

## Things to try

- Set Compaction to None and run the sale day with 4, then 16 warehouse instances. Watch the p95 stop falling.
- Set the target file size to 32 MB. Work out the file count for a sale day and compare the p95.
- Set Partitioning to None. Each query now reads all 7.5 GB instead of one day: compare the p95 with the effect of compaction.
