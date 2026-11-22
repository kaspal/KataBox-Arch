## 1

The Workers that count views take them from a stream. How many views can that stream hand out at the same time, and does adding Workers change that number?

## 2

On a stream, each partition hands out one message at a time, so the partition count caps how many views are being counted at once. A view takes about 8 to 10 ms to count. Work out how many views a second your partitions can count, and leave room for twice the traffic or for slower Workers.

## 3

Give the Kafka stream about 48 partitions, and give the Counters at least as many slots as partitions: Celery runs 8 per instance by default, so 8 instances give 64. Put a Redis Cache in front of the database for the dashboard, with a short TTL such as 10 seconds.
