# Editorial

## Where the time goes

Start with one query, alone in the warehouse. Its time is the query overhead (300 ms), plus the data it reads at the scan rate, plus about 20 ms for every file it opens, 8 files at a time.

A normal day is 2 GB. Partitioned by day, the query reads only today: 2 GB at 5 GB/s is 0.4 s. In 4 MB files, those 2 GB are 512 files, and opening them takes 512 × 20 ms ÷ 8 ≈ 1.28 s. The files cost more than three times the reading. Compacted into 256 MB files, the same day is 8 files: 0.02 s. So one query takes about 1.98 s on the small files and 0.72 s compacted.

A sale day is 20 GB, and the pipeline wrote it in the same 4 MB files: 5,120 files, 12.8 s just to open them, and about 17.1 s per query. That is over the 8 s limit before any query waits for another. Compacted, it is 80 files and about 4.5 s.

That is the core of the problem: the file count sets a floor under every query, and no number of warehouse instances lowers it. Instances only decide how many queries run at once. Those matter for the month-end review, when four times the queries arrive: about 33 a second.

## The reference design

An AWS ALB sends the traffic to 2 Go App Servers. Event batches go to an Apache Kafka stream with 48 partitions, consumed by 2 Apache Flink instances (the default concurrency of 4) that write into a Lakehouse Table (Delta Lake), partitioned by day, with Compaction set to Hourly and a target file size of 256 MB. The App Servers send queries to a Data Warehouse (Databricks SQL) with 4 instances, a concurrency of 8, a scan rate of 5 GB/s, the default 300 ms query overhead and the result cache on, which reads the table in place. The Client waits up to 60,000 ms.

It costs about $41,339 per month: $40,880 for the warehouse, $153 for the stream, $140 for the stream processor, $140 for the App Servers, $25 for the ALB and $1 for the table's storage. A normal day has a p95 of about 741 ms with no errors, and the warehouse is busy about 15% of the time.

The scan rate is the warehouse's size, and it is billed: an instance at the default 1 GB/s costs $2,044 a month, one at 5 GB/s five times that, $10,220. The sale day sets it, because its 20 GB must be read well inside 8 s. At 4 GB/s a compacted sale-day query takes about 5.5 s, and 4 instances fall behind (a p95 of about 11.4 s); 5 instances at 4 GB/s pass for the same $40,880, and 3 at 8 GB/s pass for $49,056.

"A sale day: ten times the events, and analysts look at today" sends ten times the event batches, and every query reads the 20 GB sale day. The p95 is about 5,366 ms with no errors and no lost events: each compacted query takes about 4.5 s, and the roughly 8 queries a second, one in five answered by the result cache, keep about 30 of the 32 slots busy.

"Month-end review: four times the queries" keeps the p95 at about 741 ms. At about 0.72 s a query, 33 queries a second keep about 19 of the 32 slots busy, and the result cache answers one in five almost at once.

For the cost challenge, the search found no design under 80% of the reference's cost (its best came in at almost 100%), so this problem has no challenge. The warehouse is most of the bill, and with fewer instances there is little room left for the sale day.

## Why the first try breaks

A common first try is the same design with Compaction set to None: the stream processor's files stay as they were written, and the target file size does nothing. It costs the same $41,339 and passes "A normal day" with a p95 of about 1,994 ms, but each query holds a slot almost three times longer, and the warehouse is busy about 43% of the time.

On the sale day, every query takes about 17.1 s on its own. The p95 climbs to about 43,778 ms and about 8.9% of the requests fail: queries pile up behind each other, and those that wait past the Client's 60 s timeout give up. Adding instances does not help: the same design with 16 warehouse instances ($163,979, more than twice the budget) still has a p95 of about 17,260 ms on the sale day, because that is how long one query takes.

At month end, four times the queries at about 2.0 s each need about 52 slots, and there are 32. The queue grows all through the test and the p95 reaches about 35,913 ms. Here more instances would help, but they would cost far more than compaction does.

The point is this: streaming ingest trades file size for freshness, and every reader pays for it on every query. Compaction is the background work that pays it back once. Size the warehouse for how many queries run at once, but fix the file count first, because it sets how long each one takes.

## What the simulator leaves out

Each test lasts 60 s and stands for an hour of that traffic; the table does not grow during a run. Compaction is free and complete here: the job that merges the files costs nothing and uses no warehouse or stream processor time, and every file is already compacted, while real hourly compaction leaves the newest hour in small files until its next run. Delta Lake's file statistics, data skipping and Z-ordering, which let a query skip files inside a partition, are not simulated. The 20 ms per file and the 8 parallel readers are modelling choices, not measurements. Databricks SQL bills only the seconds it is busy plus its auto-suspend delay, but this steady traffic never lets it suspend, so it bills every hour; a faster scan is a bigger size, billed in proportion. The prices are the registry's teaching prices.

## Things to try

- Set Compaction to None and run the sale day with 4, then 16 warehouse instances. Watch the p95 stop falling.
- Set the target file size to 32 MB. Work out the file count for a sale day and compare the p95.
- Set Partitioning to None. Each query now reads all 60 GB instead of one day: compare the p95 with the effect of compaction.
