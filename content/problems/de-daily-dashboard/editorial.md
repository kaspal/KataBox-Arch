# Editorial

## Where the load goes

A tile query that the Result cache can't answer costs about 0.8 s of a warehouse slot: 300 ms of overhead plus 0.5 GB at 1 GB/s. One cluster has 8 slots, so it finishes about 10 such queries a second.

On a normal hour, 20 tile loads a second with 60% answered by the Result cache leave 8 full queries a second, 8 × 0.8 = 6.4 busy slots out of 8. One cluster just copes. At 9 a.m. the traffic is ten times that: 80 full queries a second need about 64 slots, or 8 clusters, and on Snowflake each cluster costs $1,460 a month while it runs.

But the rush is not 200 different questions a second. It is the same 400 tiles, asked again and again. A cache in front, keyed by tile with a TTL of 600 s, asks the warehouse about 400 / 600 ≈ 0.7 times a second once it is warm, at 20 loads a second or at 200. The cold start is the only hard moment: at most 400 queries, one per tile, if requests for a missing tile wait for one query instead of each running their own.

## The reference design

An AWS ALB sends the traffic to 2 Go App Servers. They read tiles from a Redis Cache with a TTL of 600 s and "Coalesce misses (stampede guard)" on; on a miss they query one Snowflake Data Warehouse with its defaults (1 cluster, Concurrency per instance 8, Result cache on, Auto-suspend after 60 s). On a normal hour it costs about $1,053 per month: $852 for the warehouse, $140 for the App Servers, $36 for Redis and $25 for the ALB. The warehouse is billed only for the seconds it runs, and with the cache in front it sits idle for long stretches. The p95 is about 18.6 ms, with no errors.

"9 a.m.: empty caches, then ten times the traffic for 40 s" starts with an empty Redis and multiplies the traffic by ten from 5 seconds in. The first loads of each tile miss together; the stampede guard sends one query per tile, the Result cache answers 60% of those in about 30 ms, and the others queue for the 8 slots for a few seconds. A few seconds of that start have a p95 over 2 s, but the test's p95 is about 42.8 ms, with no errors. The warehouse runs more, so this test bills about $1,661.

"The warehouse slows down" adds 3 s to every query that scans, from 20 seconds in, for 40 seconds. Only the few cache misses reach the warehouse, and its Result cache answers some of those without scanning. The p95 is about 20.2 ms, with no errors.

Each piece is needed. Without the stampede guard, the cold start sends a query for every load of a missing tile, and the rush fails with about 2.5% errors. With the Result cache off, every one of those queries scans, and the rush fails with about 23% errors.

The cost challenge found nothing under 80% of the reference's cost (its best proof came in at 91%), so this problem has no cost challenge: the warehouse is most of the bill, and one cluster is the least you can run.

## Why the first try breaks

A common first try is the same ALB and 2 Go App Servers querying one Snowflake cluster directly, Result cache on, with no cache in front. It costs about $1,625 per month, because the warehouse is busy 78% of the time and so is billed almost all the time. It passes "A normal hour" with a p95 of about 1,037 ms: the 40% of queries that scan take 0.8 s, plus some queueing.

At 9 a.m. it needs 64 slots and has 8. The queue fills within seconds, queries wait past the client's 5 s timeout or are refused, and about 81% of the loads fail. The Result cache doesn't save it: a repeated query is quick, but it still takes a slot and a place in the queue, and 40% of the rush still scans.

When the warehouse slows down, each scanning query takes about 3.8 s. 8 a second now need about 30 slots instead of 6.4; the queue grows from the first seconds, and about 39% of the loads fail.

Adding clusters is the expensive way out. Here, 8 clusters busy through a test bill about 8 × $1,460 = $11,680 a month, far over the budget; even a real warehouse that auto-suspends would run all 8 every morning to answer the same 400 tiles. The point is this: a dashboard is a few questions asked many times. Answer each once, keep the answer as long as the data allows, and leave the warehouse the work that is actually new.

## What the simulator leaves out

The warehouse's Result cache is modelled as a fixed share of queries (60%) answered quickly; a real one depends on the exact query text, the user's role and whether the tables changed since, and the hourly load empties it. Snowflake's multi-cluster auto-scaling (adding clusters when queries queue) is not simulated: Instances is a fixed number of clusters. Raising Concurrency per instance here adds slots for free, while on a real warehouse the queries would share the same compute and each would run slower. Snowflake's minimum charge of 60 s each time a cluster resumes, and the cost of loading the data every hour, are not billed. The prices are the registry's teaching prices. The rush is scaled down to 40 seconds.

## Things to try

- Turn "Coalesce misses (stampede guard)" off on the Redis Cache and run the 9 a.m. test. Watch the warehouse's queue in the first seconds.
- Turn the Result cache off on the warehouse and run the 9 a.m. test again.
- Remove the Redis Cache and give the warehouse more clusters until the 9 a.m. test passes. Compare the bill with the reference's.
