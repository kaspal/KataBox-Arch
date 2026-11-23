# Editorial

## Where the load goes

At 2,000 requests a second, 97% are new events: about 1,940 a second, each a write to the search index. Searches are only 60 a second. So the index spends almost all its time indexing, and the question is what happens when it can't keep up.

The hidden tests ask exactly that. When a batch job logs everything at once, traffic triples for 20 seconds: about 5,800 events a second, some 116,000 events in all. When the search cluster is down for 30 seconds, about 58,000 events arrive with nowhere to be written. If the App Server writes the index itself, each of those events waits on the index or fails. If the event is safe somewhere else first, it can be indexed later.

## The reference design

A Client calls an AWS ALB in front of 2 Go App Servers. For each event, the App Server publishes to an Apache Kafka Stream with 128 Partitions and answers at once. 6 Celery indexer instances, each with a Concurrency per instance of 32 and a Service time of 5 ms, consume the stream and write to OpenSearch with 6 Shards. Searches go straight from the App Servers to OpenSearch. This costs about $1,488 per month: OpenSearch $750, the indexers $420, Kafka $153, the App Servers $140 and the ALB $25.

On "A normal day" the p95 is about 24 ms with no errors. OpenSearch is about 61% busy and the indexers about 33%.

Why 128 partitions? A stream gives each partition to one consumer at a time. With the default 6, only 6 events are being indexed at once, and the indexers could take only a few hundred events a second, far below 1,940. With 128, the 192 indexer slots have enough work to share.

"A batch job logs everything at once" triples the traffic from 15 seconds in, for 20 seconds: about 6,000 events a second. The App Servers only publish, so the answer doesn't wait on the index: the p95 is about 25 ms with no errors. The stream holds what the index can't take yet, and the indexers catch up after the burst.

"The search cluster is down for 30 s" stops OpenSearch from 20 seconds in, in a run of 120 seconds. It allows 5% errors and 95% availability. The searches during the outage fail, and nothing can fix that: they are 3% of the traffic for a quarter of the run, about 0.78% errors in all. No event is lost. Kafka keeps every record, and the indexers retry the ones that failed until OpenSearch is back: a stream doesn't drop a record its consumer failed on.

For the cost challenge, a proof design comes in at 76% of the reference's cost, under the 80% target. It keeps the same shape, with fewer App Servers and fewer indexer instances.

## Why the first try breaks

A common first try is 4 Go App Servers that write each event to OpenSearch inline, with 6 Shards, and a Celery card left on the canvas but not wired. It costs about $1,475, almost the same as the reference, and passes "A normal day" with a p95 of about 65 ms.

In the batch job, about 6,000 events a second arrive, and OpenSearch can't index that many. Every request waits for its write, the waits pile up, and many time out: 26.8% errors, with a p95 of about 786 ms. When the search cluster is down, every event in those 30 seconds is refused, because there is nowhere else to put it: 25% errors, far over the 5% the test allows.

The point is this: acknowledge an event once it is durable in a log, and index it from the log at the index's own pace. The stream absorbs both a burst and an outage, and the cost is the same.

## What the simulator leaves out

Kafka's replication and its acks settings are not simulated: an event is durable the moment it is published. Retention isn't either: records are kept for the whole test. Ordering within a partition, OpenSearch's bulk indexing, and the immutability and tamper evidence a real audit log needs (hash chains, WORM storage) are left out too.

## Things to try

- Set the Stream's Partitions back to 6 and run "A normal day". Watch the indexers' busy share and how far indexing falls behind.
- Remove the Stream and wire the App Servers straight to OpenSearch, then run the batch job. Compare the errors with the reference.
- Cut the indexers to 2 instances and run "The search cluster is down for 30 s". Watch how long the backlog takes to drain after OpenSearch is back.
