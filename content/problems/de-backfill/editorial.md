# Editorial

## Where the load goes

Every batch, live or backfill, ends in one load into the warehouse, and that is the only expensive step. A load holds a warehouse slot for about 300 ms at the median and longer in the tail, so one Snowflake instance with 8 slots loads about 8 / 0.3 ≈ 27 batches a second at best, and about 24 once the slow loads are counted. The ingest API, the Stream and the Stream Processor are cheap and far from busy.

At the normal peak, 24 live and 16 backfill batches a second ask for 40 loads a second. Two instances (about 48 a second) can do it, at about 86% busy. The hidden tests take that thin margin away twice:

- At 5 times its pace, the backfill sends 80 batches a second: 104 in all, more than twice what two instances load.
- In the live burst, 60 live batches a second arrive on their own, more than two instances can load even with no backfill at all.

When the pipeline asks for more loads than the warehouse can do, a pile forms in the Stream, and the order in which the Stream hands it out decides who waits. A live batch is measured by completion (until it is in the warehouse), so a live batch that waits makes the dashboards stale. A backfill batch is measured by response (until the Stream has it), so a backfill batch that waits costs nothing: it was acknowledged the moment it was stored. This is the same split the outage replay "The backlog that buried the orders" uses; there the pile came from an unplanned storage brownout, here from a planned job that someone sped up.

## The reference design

An AWS ALB sends the batches to 2 Go App Servers, which publish them to an Apache Kafka Stream with 32 Partitions and "Live before bulk (priority)" on. An Apache Flink Stream Processor with 2 instances and a Concurrency per instance of 16 consumes it and loads each batch into a Snowflake Data Warehouse with 3 Instances. It costs about $4,838 a month: $4,380 for the warehouse, $153 for Kafka, $140 for Flink, $140 for the App Servers and $25 for the ALB. Three instances load about 72 batches a second, and the 32 partitions and 32 Flink slots are enough to keep all 24 warehouse slots busy. A normal peak hour has a p95 of about 584 ms, with the warehouse about 57% busy.

"The backfill runs at 5 times its pace, at the peak" multiplies the backfill by 5 for the whole minute. The pipeline is asked for 104 loads a second and does about 72, so the backfill piles up in the Stream by about 30 batches a second. With priority on, each new live batch goes ahead of every queued backfill batch, so live batches only wait for a free slot. The p95 is about 573 ms, with no errors and nothing lost: the backfill batches are stored and acknowledged, and they wait their turn.

"A live burst: 2.5 times the live events" sends 60 live batches a second with the backfill at its normal 16. The 60 live batches fit in the 72 loads a second with room to spare, and the backfill takes what is left and waits for the rest. The p95 is about 818 ms.

The two fixes cover different tests. Priority alone, on the first try's two instances, passes the fast backfill (p95 about 479 ms) but not the burst: 60 live batches a second are more than two instances load, and the live batches pile up behind each other (p95 about 15.9 s). Three instances without priority pass the burst (p95 about 5.2 s) but not the fast backfill (p95 about 20.6 s). Four instances without priority pass all three tests at $6,298, but only because the test ends: 104 batches a second against about 96 loads a second is still a pile that grows, and the live batches are in it. Five instances are over the budget.

There is no cost challenge for this problem. The warehouse instances are about 90% of the bill, and the cheapest design the search found that passes every test costs 99% of the reference.

## Why the first try breaks

A common first try is the same pipeline sized for the normal peak: a Kafka Stream with 16 Partitions and no priority, Flink with 2 instances and a Concurrency per instance of 8, and a Snowflake warehouse with 2 Instances. It costs about $3,378 and passes the normal peak hour with a p95 of about 632 ms, the warehouse about 86% busy.

When the backfill runs at 5 times its pace, 104 batches a second arrive and about 48 are loaded. The Stream is first in, first out, so every new live batch joins the back of a pile that grows by about 55 batches a second. A live batch waits more than 10 s from about 19 s into the test, and more than 30 s by its end. The p95 is about 41.3 s. Nothing fails and nothing is lost: every batch is still in the Stream. The dashboards are just stale.

The live burst breaks it too: 76 batches a second against 48, the same growing pile, a p95 of about 33.8 s. Here priority would not have been enough on its own: the live batches alone are more than two instances can load.

The point is this: a backfill that shares the live pipeline must never be in front of the live work, and the pipeline must have room for the live peak on its own. Let the live records go first, let the backfill wait in the Stream where waiting is cheap, and size the expensive stage for the live burst, not for the average.

## What the simulator leaves out

Kafka has no message priority. "Live before bulk (priority)" stands for what teams build instead: two topics, with the processor always reading the live one first, or a separate pipeline (its own Stream, processor and warehouse cluster) for the backfill. This problem's two batch types share one route, so the tests judge the shared pipeline only. Real warehouses load micro-batches through files (Snowpipe, COPY) and bill compute time; here a load is a slot held for about 300 ms, and an instance is billed by the month. The backfill's own completion is not judged, and the test does not check that it ever finishes; reading the history from the archive is not simulated, and neither is overwriting the old rows with the fixed ones (or the duplicates that a rerun would make). A Stream here holds up to 100,000 waiting batches; past that, publishes are refused. A real backfill runs for days; each test sees one minute of it.

## Things to try

- Turn priority off in the reference and run the fast backfill. Watch the live batches wait behind the backfill pile.
- Turn priority on in the first try and run the live burst. Compare 60 live batches a second with what two warehouse instances can load.
- Give the reference 32 Flink slots but only 16 Partitions. Which limit decides how many loads run at once?
