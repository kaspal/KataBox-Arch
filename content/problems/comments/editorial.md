# Editorial

## Where the load goes

At 6,000 requests a second, about 5,760 are reads and about 240 are new comments. On a normal evening the popularity is a Zipf of 0.9 over 100,000 threads: the popular threads get more, but the reads still spread over many keys. Almost any store keeps up with that.

The viral test changes the shape, not just the size. Traffic triples to 18,000 requests a second, and the popularity sharpens to a Zipf of 1.4. At that skew the top thread alone gets about a third of everything (1/ζ(1.4) ≈ 32%): about 5,800 requests a second for one key. A partitioned store places a key on one partition, so that whole load lands on one machine, whatever the number of partitions.

## The reference design

An AWS ALB spreads the traffic over 4 Go App Servers. They read from a Redis Cache of 4 GB with a TTL of 300 s, and on a miss they read Cassandra with 1 partition. A new comment is written to Cassandra, then the App Server refreshes that thread's entry in the cache. This costs about $604 per month: $280 for the App Servers, $250 for Cassandra, $49 for Redis and $25 for the ALB. A normal evening has a p95 of about 18.6 ms, no errors and 0.09% stale reads. Cassandra is only about 4% busy: it sees the misses and the comments, not the reads.

"One thread goes viral" triples the traffic from 15 seconds in, for 30 seconds, with a Zipf popularity of 1.4. Redis answers the hot thread from memory, so the p95 stays at about 18.6 ms with no errors. Stale reads rise to 6.65%: the hot thread gets new comments every second, and a read can land between the write to Cassandra and the cache refresh. That is fine here; a comment that shows up a moment late does no harm.

"The cache is flushed on a busy evening" doubles the traffic with a Zipf popularity of 1.2 and empties the cache 20 seconds in. The hot threads miss once and are back in the cache within milliseconds, and Cassandra, normally 4% busy, has room for the cold misses that follow. The p95 is about 19.0 ms, with no errors.

There is no cost challenge for this problem. The best cheaper design found still cost about 82% of the reference, over the 80% target: the Redis Cache that carries the lesson is already a small part of the bill.

## Why the first try breaks

A common first try is the same edge and App Servers with no cache, and Cassandra with 2 partitions, sized for a normal evening: each partition is about 34% busy. It costs about $805, more than the reference, and passes "A normal evening" with a p95 of about 22.7 ms.

The viral test breaks it: about 3.6% of the requests are dropped, and the p95 climbs to about 62 ms, over the 60 ms limit. The viral thread lives on one partition, because the key decides the partition. That partition gets the thread's ~5,800 requests a second plus half of everything else, about 12,000 a second, while the other partition gets about 6,000. Adding partitions doesn't help: they split the other threads more finely, but one key still has one home. The flush test passes, since there is no cache to flush.

The point is this: a cache absorbs a hot key, because one Redis node answers it from memory, and partitioning can't spread a single key. The cached design is also cheaper, because the store only sees the misses.

## What the simulator leaves out

Cassandra replicas and consistency levels are not simulated: replication_factor and consistency have no effect on the run. Real hot-partition rules, such as DynamoDB's adaptive capacity, are not simulated either. Comment order and paging are left out: each read is one key. The cache's memory per thread is only its value size, and App Servers have no local caches of their own.

## Things to try

- Remove the Redis Cache, raise Cassandra to 4 partitions and run the viral test. Watch whether the hot partition gets any cooler.
- Lower the cache's TTL to 10 s and run the visible test. Compare how busy Cassandra gets with the reference.
- Halve the cache's memory a few times, running the flush test after each step. Watch the hit rate and how busy Cassandra gets.
