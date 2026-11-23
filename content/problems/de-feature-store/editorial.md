# Editorial

## Where the load goes

At 1,000 requests a second, 800 are online reads, 198 are feature updates and 2 are training reads. They want different stores.

A Data Warehouse spends about 300 ms on every query, and an instance runs 8 at a time, so one instance answers at most about 8 / 0.3 ≈ 27 queries a second. Serving all 800 online reads from it would keep 800 × 0.3 = 240 queries in flight: 30 instances, about $44,000 a month, and still a p95 of at least 300 ms against a target of 50 ms. The 2 training reads a second are different: each takes about 0.8 s (300 ms plus 0.5 GB at 1 GB/s), so 2 × 0.8 ≈ 1.6 are in flight, an easy fit for one instance. That is the work a warehouse is made for.

The updates are a freshness question. In the reference each one waits only a few milliseconds for its write, so a handful of slots keep up with 198 a second. The hidden slowdown adds 100 ms to every record: then 198 × 0.1 ≈ 20 records must be in flight at once, and the number in flight is capped by the Stream's partitions or the Stream Processor's slots, whichever is smaller. Below 20, the Stream holds the excess and the lag grows every second.

## The reference design

An AWS ALB sends the traffic to 2 Go App Servers and a Generic Microservice. For a prediction, the App Server reads the user's row from a ScyllaDB NoSQL Store with 1 partition: the online store, which holds every user. For an event, it publishes to an Apache Kafka Stream with 32 partitions. Apache Flink, 4 instances with a concurrency of 8 (32 slots), consumes the Stream and writes each user's new features into the same ScyllaDB. The Microservice, the offline feature service, sends the training reads to a Snowflake Data Warehouse with 1 instance. Nothing online touches the warehouse.

It costs about $2,378 per month: $1,460 for Snowflake, $280 for Flink, $250 for ScyllaDB, $153 for Kafka, $140 for the App Servers, $70 for the Microservice and $25 for the ALB. Snowflake bills only while it runs, but a training read every half second keeps it running all month. On a normal day the p95 is about 23.2 ms with no errors; ScyllaDB and Flink are about 4% busy, the warehouse about 22%.

"A sale: three times the traffic for 30 s" triples everything from 20 seconds in, in a run of 90 seconds. ScyllaDB answers every user in a few milliseconds, however many different users arrive, and Flink has slots to spare. The p95 stays at about 23.2 ms, with no errors.

"The stream processor slows down for 30 s: features must still arrive within 1 s" adds 100 ms to every Flink record from 20 seconds in, for 30 seconds. Since every update is slower by design, this test's p95 limit is 1 s instead of 50 ms: what it checks is that the backlog doesn't grow. With 32 slots and 32 partitions, Flink needs about 20 records in flight and still has room, so each update simply takes about 100 ms longer. The p95 is about 112.9 ms, with no errors and no lost updates. The 4 instances are there for this test: on a normal day Flink is nearly idle.

There is no cost challenge for this problem. The warehouse is more than half of the bill, and the cheapest design the search found that passed every test still came to 94% of the reference, over the challenge's cap of 80%.

## Why the first try breaks

A common first try keeps the features where they already are, in the warehouse, and puts a Cache in front of it. The App Servers look in a Redis Cache with 4 GB of Memory and a TTL of 3,600 s (an hour), and on a miss they read a Snowflake Data Warehouse with 2 instances. Flink, with its defaults (2 instances, a concurrency of 4), writes each update straight into Redis, and Kafka has its default 6 partitions. It costs about $3,520, more than the reference: $2,920 of it is the two warehouse instances.

It passes "A normal day" with a p95 of about 23.2 ms. The busy users are always in Redis, and the misses are few enough for the warehouse, which is already about 84% busy.

The sale breaks it: about 37.3% errors and a p95 of about 1,315 ms. Three times the traffic brings many more users who aren't in the Cache, and each of their reads becomes a 300 ms warehouse query. Two instances run 16 at a time; the rest queue, the queue fills, and the reads behind it fail. A third instance doesn't save it: the design then costs about $5,050, over the budget, and still fails about 21.9% of the requests in the sale.

The slowdown breaks it too, for a second reason: a p95 of about 5,924 ms against the limit of 1 s. With 6 partitions, Flink can have only 6 records in flight; at about 100 ms each that is about 60 updates a second, against 198 arriving. The Stream keeps the rest and nothing is lost, but the lag grows by about 0.7 s every second, to about 20 s by the end of the slowdown.

The point is this: online reads and training reads are different workloads. Online serving needs a store that holds every user's features and answers any one of them in milliseconds, kept fresh by the stream as events arrive. The warehouse is where training reads history; a cache in front of it only hides its latency for the users it has seen lately. And a stream path needs headroom in partitions and slots, so a slow minute doesn't turn into a growing lag.

## What the simulator leaves out

The daily load into the warehouse and the first copy of every user into the online store are not simulated; the tests start with both in place. Point-in-time correctness of training reads (reading each feature as it was at the label's time, so a model never trains on the future) is not simulated, and neither is skew between the features used in training and in serving. Each event is one update of one user's row; Flink's windowed aggregations and its state are not modelled here. The values of the features are not checked, only when they arrive. ScyllaDB's replication across nodes is not simulated, and the prices of ScyllaDB and Flink are teaching figures in the registry that still need a source.

## Things to try

- Give the first try the reference's stream path (32 partitions, Flink with 32 slots). Which hidden test still fails, and why?
- Set Flink to 2 instances (16 slots) in the reference and run the slowdown. With about 20 records needed in flight, watch the lag.
- Lower Kafka to 6 partitions and keep Flink's 32 slots. Which of the two caps decides now?
