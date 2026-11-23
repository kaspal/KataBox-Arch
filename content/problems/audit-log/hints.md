## 1

The product only needs to know that its event is safe. Does it have to wait until the event is searchable too? And what happens to an event that arrives while the search cluster can't take it?

## 2

Put a stream between the App Servers and the index. The App Server acknowledges the event once it is in the stream, and workers index it from there at the pace the index can take. A burst then waits in the stream, and an outage only delays indexing: the stream keeps each record until a worker has written it.

## 3

Wire Go App Servers to publish to an Apache Kafka Stream, and Celery indexers to consume it and write to OpenSearch; searches still go straight from the App Servers to OpenSearch. A stream gives each partition to one consumer at a time, so raise Partitions well above the default (the reference uses 128) or the indexers sit idle. 6 Celery instances at a Concurrency per instance of 32, 2 App Servers and 6 OpenSearch Shards are plenty.
