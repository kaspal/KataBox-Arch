# Editorial

## Where the load goes

A tile query that the Result cache can't answer costs 300 ms of overhead plus 0.5 GB of scanning. One cluster has 8 slots, but it scans 1 GB/s in all: a warehouse's instances share their scan rate among the queries running at once, so eight scans together each get an eighth of it. One cluster finishes at most 2 scanning queries a second, however many slots it has.

On a normal hour, 20 tile loads a second with 60% answered by the Result cache leave 8 full queries a second: 4 GB/s of scanning, four clusters' worth. At 9 a.m. the traffic is ten times that: 80 full queries a second need 40 GB/s, or 40 clusters, and on Snowflake each cluster costs $1,460 a month while it runs.

But the rush is not 200 different questions a second. It is the same 400 tiles, asked again and again. A cache in front, keyed by tile with a TTL of 600 s, asks the warehouse about 400 / 600 ≈ 0.7 times a second once it is warm, at 20 loads a second or at 200. The cold start is the only hard moment: at most 400 queries, one per tile, if requests for a missing tile wait for one query instead of each running their own. About 40% of those scan, up to 80 GB in the first seconds. One cluster needs over a minute for that, far past the client's 5 s; five clusters scan 5 GB/s and catch up while the popular tiles are already cached.

## The reference design

An AWS ALB sends the traffic to 2 Go App Servers. They read tiles from a Redis Cache with a TTL of 600 s and "Coalesce misses (stampede guard)" on; on a miss they query one Snowflake Data Warehouse with 5 clusters (Instances 5) and its other defaults (Concurrency per instance 8, Result cache on, Auto-suspend after 60 s). On a normal hour it costs about $4,448 per month: $4,258 for the warehouse, $140 for the App Servers, $24 for Redis and $25 for the ALB. The warehouse is billed only for the seconds it runs, and with the cache in front it sits idle for long stretches. The p95 is about 18.6 ms, with no errors.

"9 a.m.: empty caches, then ten times the traffic for 40 s" starts with an empty Redis and multiplies the traffic by ten from 5 seconds in. The first loads of each tile miss together; the stampede guard sends one query per tile, the Result cache answers 60% of those in about 30 ms, and the others share the five clusters' 5 GB/s for a few seconds. The test's p95 is about 37.2 ms, with no errors. The warehouse runs more, so this test bills about $7,489.

"The warehouse slows down" adds 3 s to every query that scans, from 20 seconds in, for 40 seconds. Only the few cache misses reach the warehouse, and its Result cache answers some of those without scanning. The p95 is about 20.2 ms, with no errors.

The cache in front and the Result cache are both needed. With the Result cache off, every one of the cold-start queries scans, and the rush fails with about 25% errors. With one cluster instead of five, the cold start's scans queue behind each other and the rush fails with about 23% errors; with four, about 0.24% of the loads fail, more than the 99.9% availability allows. The stampede guard is cheap insurance: with five clusters the rush still passes without it, but every load of a missing tile then starts its own query.

The cost challenge's only cheaper design moves the tables to ClickHouse, which this brief rules out (the data lives in Snowflake), so this problem has no cost challenge: the warehouse is most of the bill.

## Why the first try breaks

A common first try is the same ALB and 2 Go App Servers querying one Snowflake cluster directly, Result cache on, with no cache in front. It costs about $1,625 per month, because the warehouse is busy all the time. It fails even "A normal hour": 8 scanning queries a second ask for 4 GB/s from a cluster that scans 1 GB/s, the queue grows from the first seconds, queries wait past the client's 5 s timeout or are refused, and about 85% of the loads fail (p95 about 4,957 ms).

At 9 a.m. it needs 40 GB/s and has 1, and about 97% of the loads fail. The Result cache doesn't save it: a repeated query is quick, but it still takes a slot and a place in the queue, and 40% of the rush still scans. When the warehouse slows down, about 88% of the loads fail.

Adding clusters is the expensive way out. Here, 40 clusters busy through the rush would bill about 40 × $1,460 = $58,400 a month, far over the budget; even a real warehouse that auto-suspends would run all 40 every morning to answer the same 400 tiles. The point is this: a dashboard is a few questions asked many times. Answer each once, keep the answer as long as the data allows, and leave the warehouse the work that is actually new.

## What the simulator leaves out

The warehouse's Result cache is modelled as a fixed share of queries (60%) answered quickly; a real one depends on the exact query text, the user's role and whether the tables changed since, and the hourly load empties it. Snowflake's multi-cluster auto-scaling (adding clusters when queries queue) is not simulated: Instances is a fixed number of clusters. Raising Concurrency per instance adds slots but no scanning: the queries on a cluster share its scan rate, as on a real warehouse. Snowflake's minimum charge of 60 s each time a cluster resumes, and the cost of loading the data every hour, are not billed. The prices are the registry's teaching prices. The rush is scaled down to 40 seconds.

## Things to try

- Set the warehouse's Instances to 1, then 4, and run the 9 a.m. test. Watch the warehouse's queue in the first seconds.
- Turn "Coalesce misses (stampede guard)" off on the Redis Cache and run the 9 a.m. test. Count the queries that reach the warehouse.
- Turn the Result cache off on the warehouse and run the 9 a.m. test again.
- Remove the Redis Cache and give the warehouse more clusters until the 9 a.m. test passes. Compare the bill with the reference's.
