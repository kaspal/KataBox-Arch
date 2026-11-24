# Editorial

## Where the load goes

At 3,000 requests/s, about 2,970 are page views and about 30 are dashboard reads. Each view goes from the Collector to the stream, then to a Counter, which spends 2 ms on the increment and writes to PostgreSQL. With the hops and the write, a view takes about 8 to 10 ms to count.

Each partition hands out one message at a time, so the partition count caps how many views are being counted at once, however many Counters you add. At about 9 ms a view, one partition counts roughly 110 views a second: 32 partitions count about 3,500 to 4,000 a second, and 48 count about 5,000 to 6,000. The Counters need at least as many slots as there are partitions, or some partitions sit idle.

## The reference design

The Collector, 2 Go instances behind a Load Balancer, publishes each view to a Kafka stream with 48 partitions. 8 Celery Counter instances consume it; at Celery's default concurrency of 8 that is 64 slots, enough to keep all 48 partitions busy. The Counters write to PostgreSQL. For the dashboard, the Collector reads a Redis Cache first, with a TTL of 10 seconds, and fills it from PostgreSQL on a miss. This costs about $1,162 per month, and on a normal day views are counted with a p95 of about 32 ms.

"The homepage goes viral" doubles the traffic for 15 seconds, from 20 seconds in, and allows a p95 of 10 seconds. About 5,940 views a second arrive, at the edge of what 48 partitions can count, so a backlog builds and then drains: the p95 is about 5.5 s. "Consumers slow down" adds 10 ms to every message on every Worker card for 20 seconds, from 15 seconds in, with the same 10 s limit. At about 19 ms a view, 48 partitions count only about 2,500 a second, a little less than what arrives. The small backlog clears once the Workers recover: the p95 is about 4.4 s.

The cache keeps dashboard reads off the busy database. Its counts can be up to 10 seconds behind, and the judge reports about 25% of dashboard reads as stale: fine for a stats dashboard, so not judged here.

## Why the first try breaks

A common first try uses 32 partitions and 4 Counter instances, which is 32 slots, and no cache: the dashboard reads PostgreSQL directly. It passes "Normal day", but with a p95 of about 2.4 s, already close to the 3 s limit. 2,970 views a second is most of what 32 partitions can count, so any burst queues.

The viral test pushes far past that ceiling: the backlog grows for the whole 15 seconds and the p95 reaches about 18.3 s. When the Workers slow down, 32 partitions count only about 1,700 views a second, and the p95 reaches about 12.1 s. Adding Counter instances would not help either test: the partitions are the limit.

## What the simulator leaves out

Real counters add up many views in memory and write each page's total once a second, which cuts the database writes enormously. The simulator counts each view separately, so this problem measures partitions and consumer capacity instead.

Exactly-once counting is not simulated either. In real life, a retried view could be counted twice.

## Things to try

- Keep 32 partitions and raise the Counters to 16 instances. The viral p95 barely moves.
- Set 48 partitions but leave 4 Counter instances (32 slots). Watch how many partitions stay idle.
- Lower the cache TTL to 2 seconds and compare the stale rate.
