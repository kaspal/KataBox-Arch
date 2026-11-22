# Editorial

## Where the load goes

At 5,000 requests/s, about 4,750 are searches and about 250 are listing updates. The searches return about 8 KB each, some 38 MB/s of results. All 200,000 answers together are about 1.6 GB, so they fit in a Redis Cache of the default 2 GB.

Elasticsearch takes about 20 ms per call, and a query always lands on the same shard. With Zipf 1.1, the most popular queries pile onto a few shards, and the busiest one sets the limit, not the average.

Each update ends with a trip through the stream: an Indexer spends about 10 ms on it and then writes it into Elasticsearch, roughly 30 ms in all. A stream keeps one message per partition in flight at a time, so 6 partitions move about 6 ÷ 0.03 s = 200 changes a second, fewer than the 250 that arrive.

## The reference design

A Load Balancer (AWS ALB) sends requests to 8 Go instances of the Search API, 512 slots at the default concurrency of 64. Searches check a Redis query cache with a TTL of 30 s first, and on a miss ask Elasticsearch, with 4 shards. Updates are written to PostgreSQL, which is wired to a Kafka stream of 24 partitions by its change data capture edge. 2 Celery Indexers, with a service time of 10 ms, consume the stream and write each change into Elasticsearch. This costs about $1,760 per month, with a p95 of about 50 ms.

"A reindex at peak" adds 30 ms to every Elasticsearch call from 15 seconds, for 30 seconds, and allows a p95 of 400 ms. Only cache misses are slowed: the p95 rises to about 143 ms. Each change now takes about 60 ms; 24 partitions still allow about 400 a second and the Indexers' 16 slots (2 × the default concurrency of 8) about 260, so the index keeps up. "Everyone searches for the same thing" makes query popularity much steeper (Zipf 1.5). The cache absorbs the hotter queries, and the p95 drops to about 38 ms.

In tuning, a stream left at Kafka's default 6 partitions fell further and further behind; 24 keep the index fresh.

## Why the first try breaks

A common first try is the same design without the query cache, for about $1,644 per month. Every search reaches Elasticsearch, and even on a normal day the busiest shard cannot keep up, because the popular queries all land on it. The App Servers fill up waiting on it, and about 61% of requests are dropped, with a p95 of about 4.5 s. The reindex makes it worse, about 73%, and so do the hot queries, about 66%.

More shards do not fix it. A query still goes to one shard, so the hot one stays hot: 10 shards still dropped requests in tuning. A small cache in front of the hot queries does what more shards cannot.

## What the simulator leaves out

Relevance and ranking are not simulated: every answer is just an 8 KB result. A real reindex rebuilds the index while it serves traffic, which costs disk, memory and CPU; here it is only slower calls. The search index's own replicas are not modelled in detail either.

## Things to try

- Set the stream back to 6 partitions and watch the updates' completion latency grow through the run.
- Remove the query cache and raise Elasticsearch to 10 shards. Compare the cost and the drops.
- Lower the cache's TTL to 5 s and run "Normal day" again.
