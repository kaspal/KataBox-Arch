## 1

An incident makes every service log three times as much, and a consumer can crash at any moment. What limits how many batches your pipeline archives at once, and what happens to the batches a dead consumer was holding?

## 2

A stream hands out one message per partition at a time, so the partition count caps how many batches are archived at once; give it enough partitions for a tripled flow, and enough Worker slots to keep every partition busy. Then split those Workers across more than one card. When a consumer dies, the batches it held wait out the stream's session timeout before the others take them over, and if it was your only consumer, nothing is shipped at all.

## 3

Use a Kafka stream with about 128 partitions, and two Celery Shipper cards of 10 instances each: at Celery's default concurrency of 8, that is 160 slots, and either card can carry a normal day alone. The Shippers write to S3; the Collector sends searches to Elasticsearch.
