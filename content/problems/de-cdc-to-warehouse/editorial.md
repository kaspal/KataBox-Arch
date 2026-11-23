# Editorial

## Where the load goes

12 changes a second is very little for PostgreSQL. The hard part is the other end: the warehouse writes each change on its own, in about 300 ms at the median and a little more on average (about 0.34 s, because of the slow tail). A change that waits for its write keeps something busy the whole time, so the number of writes in flight is the rate times the write time: 12 × 0.34 ≈ 4 on a normal day.

The hidden tests push that number up in two ways. The flash sale triples the rate: 36 × 0.34 ≈ 12 writes in flight. The slow warehouse makes each write about 1.84 s: 12 × 1.84 ≈ 22 writes in flight, at the normal rate.

Three cards cap the writes in flight, and the smallest cap wins:
- the Stream hands out one record per partition at a time, so the partitions are a cap;
- the Stream Processor holds a slot (instances × concurrency) for the whole write;
- the Data Warehouse runs 8 writes per instance (its concurrency).

What the pipeline moves in a second is that smallest cap divided by the write time. When changes arrive faster, the Stream keeps them in order and nothing is lost, but each one waits longer, and the freshness limit of 15 s (5 minutes in real life, scaled down for the simulator's short tests) is how long that wait may grow.

Change data capture is what makes this possible in the first place. An export that reads the orders table every hour, or every night, would leave the warehouse up to that interval behind and read the whole table from the shop's database each time. The change log already has every committed change, in order, as it happens.

## The reference design

An AWS ALB sends the traffic to 2 Go App Servers, which write each change to PostgreSQL. PostgreSQL is wired with a Publish edge (change data capture) to an Apache Kafka Stream with 24 partitions. Apache Flink, 2 instances with a concurrency of 12 (24 slots), consumes the Stream and writes each change into Snowflake, with 3 instances (24 writes at once). The three caps are equal at 24 writes in flight.

This costs about $5,188 per month: $4,380 for Snowflake ($1,460 per instance), $350 for PostgreSQL, $153 for Kafka, $140 for Flink, $140 for the App Servers and $25 for the ALB. The warehouse is almost all of it.

On a normal day the p95 freshness is about 0.70 s, and the Stream Processor is about 17% busy: 4 writes in flight out of 24.

"A flash sale: 3 times the changes for 30 s" sends 36 changes a second from 20 seconds in, in a run of 90 seconds. 24 writes in flight move about 24 / 0.34 ≈ 70 changes a second, so the burst fits, and the p95 stays at about 0.68 s.

"The warehouse slows down: every write takes 1.5 s longer for 30 s" makes each write about 1.84 s from 20 seconds in. 24 writes in flight still move about 24 / 1.84 ≈ 13 changes a second, just over the 12 that arrive. The p95 rises to about 2.4 s, the shop's writes don't notice, and no change is lost.

For the cost challenge, a proof design comes in at 69% of the reference's cost ($3,588), under the 70% target. It keeps 24 partitions but runs 1 App Server, 1 Flink instance with a concurrency of 12 and 2 Snowflake instances. During the slowdown it moves fewer changes than arrive, and the Stream holds the difference; the backlog clears fast enough afterwards to stay under 15 s at p95.

## Why the first try breaks

A common first try has the same cards with their defaults: Kafka with 6 partitions, Flink with 2 instances of 4 slots (8), and Snowflake with 1 instance (8 writes). It costs about $2,268 and passes "A normal day" with a p95 of about 0.80 s. The partitions are the smallest cap: 6 writes in flight move about 6 / 0.34 ≈ 18 changes a second, and a normal day needs 12.

The flash sale needs 36 a second. For 30 seconds about 18 more changes arrive each second than are written, so about 540 pile up in the Stream, and once the sale ends they clear at only the 6 a second the pipeline has to spare. The p95 freshness reaches about 23.7 s, over the 15 s limit.

The slowdown is the same story from the other side. With writes of 1.84 s, 6 in flight move only about 3.3 changes a second, while 12 keep arriving. About 260 changes pile up in 30 seconds, and the p95 reaches about 22.3 s.

Nothing is lost in either test, and the shop never slows down: the Stream does its job. What breaks is freshness. The point is this: the throughput of a pipeline that writes to a slow sink is its writes in flight divided by the write time. Size the partitions, the processor's slots and the warehouse's writes together, for the slowest write you have to survive, not for the average day.

## What the simulator leaves out

The warehouse writes every change on its own. Real pipelines batch many changes into one load (Snowpipe, COPY, a sink connector's micro-batches), which needs far fewer writes in flight; the lesson about the smallest cap then holds per batch instead of per change. Debezium is not a separate card: the database's Publish edge sends every committed change at once, and the connector's own restarts, offsets and the initial snapshot of a large table are not simulated. Schema changes, merging updates and deletes into one row, and exactly-once delivery are not simulated either; no test crashes the Stream Processor, so no duplicates appear. Snowflake bills the seconds it is busy, extended by its auto-suspend time, and a pipeline writing all day keeps it busy, so each instance costs the full month here. More instances are modelled as more writes at once at the same speed; queries competing for one warehouse's compute are not. The Flink price is a teaching price, not a quote.

## Things to try

- Set Kafka back to 6 partitions and keep everything else. Run the flash sale and see which card is the smallest cap now.
- Give Snowflake 1 instance and run the slowdown. Compare 8 writes in flight with the 22 the slow warehouse needs.
- Set Flink back to its defaults (8 slots), keep 24 partitions and 3 Snowflake instances, and run the slowdown again.
