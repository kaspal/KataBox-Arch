# Editorial

## Where the load goes

At 1,000 requests/s, about 990 are log batches and about 10 are searches. At 256 KB a batch, that is about 250 MB of logs a second, some 22 TB a day. A Shipper spends 5 ms on each batch and writes it to S3 (about 50 ms at the median). With the hops and the slow tail of the writes, a batch takes about 65 ms to archive.

A stream hands out one message per partition at a time, so the partitions cap how many batches are in flight. At about 65 ms a batch, 128 partitions archive about 2,000 batches a second, and 96 about 1,500. A normal day fits either way. An incident that triples the logs, about 2,970 a second, fits neither, so the stream holds a backlog until it drains.

## The reference design

The Collector, 2 Go instances behind a Load Balancer, publishes each batch to a Kafka stream with 128 partitions. Two Celery Shipper cards consume it, Shippers A and Shippers B, each with 10 instances at Celery's default concurrency of 8: 160 slots in all, more than the 128 partitions, so either card can carry a normal day alone. Both write to an S3 archive; searches go to Elasticsearch. This costs about $14,814 per month, mostly the archive: about $12,800, because S3 charges $5 for every million writes and 990 batches a second is about 2.6 billion writes a month. On a normal day the p95 is about 152 ms.

"An incident triples the logs" runs three times the traffic for 10 seconds, from 20 seconds in, and allows a p95 of 15 seconds. About 1,000 batches a second more arrive than 128 partitions can archive, so roughly 10,000 queue in the stream. Afterwards the spare 1,000 a second clears them in about 10 seconds: the p95 is about 5.4 s.

"A consumer dies and the group rebalances" kills the first Worker card, Shippers A, at 20 seconds for good, in a run of 120 seconds, with the same 15 s limit. The batches A was holding stay assigned to it until Kafka's session timeout of 45 seconds runs out, then go to Shippers B, which meanwhile takes all new batches. Its 80 slots carry a normal day by themselves, so only the batches A was holding are late, and they are too few to move the p95: it stays near 0.15 s.

## Why the first try breaks

A common first try uses 96 partitions and a single Shipper card of 12 instances: 96 slots, for about $14,254 per month. It passes "Normal day" with the same p95. It even passes the incident, though its bigger backlog drains more slowly and the p95 reaches about 10.5 s.

The rebalance test is where it fails. When the only Shipper card dies, the stream has no one left to hand batches to. Nothing is archived from 20 seconds on; every batch waits until the run ends and counts at its age then. The p95 reaches about 66 seconds. No batch is lost; the stream keeps them all.

## What the simulator leaves out

Real shippers compress the logs and group many batches into one larger object, which cuts both the bytes and the number of writes; here each batch is one write. Log retention is simplified too. The search index is a separate store that the queries read: the simulator does not index each batch, so this problem measures the archive path and its consumers.

## Things to try

- Merge the two Shipper cards into one card of 20 instances and run the rebalance test again. Same slots, same cost, different result.
- Keep two cards but drop to 96 partitions. Watch how long the incident's backlog takes to drain.
- Cut both Shipper cards to 6 instances and run the rebalance test: watch what the survivor's queue does while A's batches wait out the session timeout.
