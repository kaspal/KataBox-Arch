# Editorial

## Where the load goes

At 10,000 requests/s, about 9,900 are clicks and about 100 are reads of an ad's totals. Every click is one message on the stream and one increment of a counter.

Kafka (Amazon MSK) is billed per broker, not per message: one broker is $153 a month, however many clicks pass through it, and partitions are free. Most of the bill is the instances that do the work.

Each partition hands out one click at a time, so the partitions cap how many clicks are counted at once, and where the counter lives sets how long each click takes. A Redis increment takes a fraction of a millisecond; a PostgreSQL write about 5 ms, plus any wait for a database slot. In the judge's runs a click holds a slot for about 3.6 ms with Redis counters and about 10 ms with PostgreSQL. With 128 partitions, that is a ceiling of about 35,000 clicks a second with Redis and about 12,800 with PostgreSQL.

## The reference design

A Load Balancer (AWS ALB) sends the ad servers to 4 Go instances of the Click collector. The collector publishes each click to a Kafka stream with 128 partitions. 16 Celery Aggregator instances, 1 ms of work per click, consume it; at Celery's default concurrency of 8 that is 128 slots, one per partition. They increment the ad's counter in a Redis Cache with a TTL of 86,400 s, and the collector reads the totals from the same Redis. This costs about $1,614 per month, of which the Aggregators are $1,120 and Kafka $153. On a normal day clicks are counted with a p95 of about 20 ms, and the Aggregators are about 28% busy.

"One ad goes viral" doubles the traffic for 20 seconds, from 20 seconds in, and piles the clicks onto a few ads (Zipf 1.8), with a p95 limit of 10 s. About 19,800 clicks a second is well under the ceiling, so the p95 stays at about 20 ms. "Late events arrive all at once" multiplies the traffic by 4 for 10 seconds, from 20 seconds in, with the 30 s limit. About 39,600 clicks a second is a little past what 128 partitions count, so a backlog of some 40,000 clicks builds and then drains in a couple of seconds: the p95 is about 1.2 s. Kafka does not bill the extra messages, so the bill stays at about $1,614 in both.

## Why the first try breaks

A common first try is the same pipeline with the counters in PostgreSQL: each click is an increment in the database, and the totals are read from it too. It costs about $1,928 per month and passes "Normal day" with a p95 of about 31 ms, but PostgreSQL is already about 65% busy and the Aggregators about 78%: their slots spend most of their time waiting on the database.

Twice the clicks in the viral test is more than PostgreSQL can write and more than the 12,800 a second the Aggregators can push through it. The Aggregators fill up waiting, the reads of totals queue behind the writes, and about 5% of requests are dropped, with a p95 of about 8 s. In the late-events test about 19% are dropped. More Aggregators would only wait on the same database.

## What the simulator leaves out

Real aggregators pre-aggregate: they add up clicks per ad in memory and write each ad's total once per window, a second or a minute, which needs far fewer writes. Real systems also count by event time, by the minute a click happened rather than when it arrived, and decide how long to wait for late clicks. Neither is simulated, nor is exactly-once counting: a retried click could be counted twice. Counters kept only in Redis memory can be lost on a restart; a real system snapshots them to durable storage. Nor does the simulator model one hot ad overloading the single partition its clicks go to: the viral test's load is the extra traffic.

## Things to try

- Cut the reference's partitions to 64 and run "Late events arrive all at once". Watch the backlog grow and the p95 climb.
- Keep 128 partitions but drop the Aggregators to 8 instances (64 slots). See how many partitions sit idle.
- In the first try, double the Aggregators and run "One ad goes viral". See whether more slots help when the database is the limit.
