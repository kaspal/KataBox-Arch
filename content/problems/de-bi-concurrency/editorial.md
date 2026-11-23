# Editorial

## Where the load goes

A warehouse doesn't run queries one by one: each cluster has 8 slots, and every query holds one slot until it ends. So the question is how many slots are busy at once: queries a second × seconds per query.

An ad-hoc query takes the 300 ms overhead plus 0.1 GB at 1 GB/s: about 0.4 s. Only 5% repeat an earlier query, so the Result cache answers few of them and the rest scan. On a normal hour, 12 queries a second keep about 12 × 0.38 ≈ 4.6 slots busy. One cluster has 8, so it copes.

At 9 a.m., four times as many arrive: about 18 slots busy. One cluster has 8; two have 16, still short. When arrivals outrun the slots, the queue grows every second, and each analyst waits for everyone ahead of them. Caching doesn't help here, because the queries are all different. Only more slots do.

An exploration query scans 60 GB and holds its slot for about 60 s. The rare ones of a normal day don't matter. When the data science team starts exploring, about 0.48 of them arrive a second, and after the first minute about 0.48 × 60 ≈ 29 slots are taken by notebooks alone. On a warehouse the analysts share, those slots are gone for them.

## The reference design

The Client has a Timeout of 120,000 ms. The App Server (2 Go instances) runs the analysts' queries on a Snowflake Data Warehouse with 3 instances: 24 slots. The Microservice (2 instances) runs the notebooks' queries on a second Snowflake Data Warehouse with 5 instances: 40 slots. Both keep Concurrency per instance at 8 and Auto-suspend after at 60 s.

On a normal hour it costs about $4,660 per month: $140 for the App Servers, $140 for the Microservice, $4,380 for the analysts' warehouse and $0 for the notebooks' warehouse. No exploration query arrived in that hour, so the second warehouse stayed suspended and billed nothing. The p95 is about 417 ms, with no errors.

"9 a.m.: four times the queries for 40 s" brings about 18 busy slots to the analysts' warehouse: 24 slots run them at about 76%. The p95 is about 443 ms, with no errors. With 2 instances, the same rush reaches a p95 of about 7.1 s.

"The data science team starts exploring the full history" sends about 29 slots' worth of exploration to the notebooks' warehouse, which runs them on 40 slots. The analysts' warehouse doesn't see them: the p95 stays at about 417 ms, with no errors. The exploration queries take about 60 s each, but they are under 4% of the queries, so they sit above the p95.

The budget is judged on a normal hour. While the notebooks' warehouse works, it bills like any other: in the hidden tests the same design's monthly rate reads about $10,300 and $11,900. Auto-suspend is what makes that a cost of the busy hours only.

Why not one shared warehouse with more clusters? It works only with enough slots for both workloads at once. With 3 instances, the exploration test reaches a p95 of about 44.7 s; with 4 instances, about 8.6 s. With 5 it passes, but it costs about $7,580, over the budget: the analysts keep a shared warehouse busy all day, so all 5 clusters bill all day, while a separate warehouse for the notebooks bills only while they explore.

For the cost challenge, the search found nothing below 80% of the reference's cost (its best was 97%, one App Server instance fewer), so this problem has no challenge.

## Why the first try breaks

A common first try wires both the App Server and the Microservice to one Snowflake Data Warehouse with 1 instance. It costs about $1,740 and passes "A normal working hour" with a p95 of about 479 ms: 4.6 busy slots fit in 8.

At 9 a.m. about 18 slots' worth of queries arrive at 8 slots. The queue grows from the first seconds of the rush: the p95 of each second climbs from about 2.3 s to about 49 s, and the queue doesn't drain before the run ends. The test ends with a p95 of about 65 s and 2.39% of the queries refused when the queue is full.

When the notebooks start exploring, the exploration queries take every slot and keep it for a minute. The analysts' queries wait behind them, the queue fills, and the warehouse refuses what doesn't fit: about 69.9% of the queries fail, with a p95 of about 65 s.

The point is this: a warehouse's capacity is slots, clusters × concurrency, and you size it for the rush, not the average. And a workload that holds slots for minutes doesn't belong with one that needs answers in seconds: give it its own warehouse, and let auto-suspend make that cheap.

## What the simulator leaves out

Real multi-cluster warehouses add and remove clusters as the queue grows and shrinks (between a minimum and a maximum); here the clusters are fixed, and the card bills all of them while any query runs. Resuming a suspended warehouse takes a few seconds in real life; here it is instant. Raising Concurrency per instance doesn't slow queries down here, while real queries on one cluster share its CPU and memory; and a bigger warehouse size (a faster Scan rate per query) costs nothing extra here. That is why the statement fixes the size. Queue timeouts, per-user routing rules and the correctness of the data itself are not simulated: separating the workloads is drawn as wiring. Snowflake's price is the registry's teaching figure ($2 per credit), and time is scaled down: the rush lasts 40 s and the exploration 2 minutes.

## Things to try

- Set the analysts' warehouse to 2 instances and run the 9 a.m. rush. Watch the queue grow second by second.
- Wire the Microservice to the analysts' warehouse instead, with 4 instances, and run the exploration test.
- Set Concurrency per instance on the first try to 32 and run the rush. Then read "What the simulator leaves out" again: why wouldn't that work on a real warehouse?
