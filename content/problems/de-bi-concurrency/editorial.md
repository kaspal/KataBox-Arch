# Editorial

## Where the load goes

A warehouse doesn't run queries one by one: each cluster has 8 slots, and every query holds one slot until it ends. But the slots share the cluster's scan rate: one cluster scans 1 GB/s in total, however many queries run on it at once, and a single query never scans faster than that. So the question is how many GB arrive a second, and how many GB a second the clusters can scan.

An ad-hoc query takes the 300 ms overhead plus 0.1 GB at 1 GB/s: about 0.4 s while the warehouse has scan rate to spare. Only 5% repeat an earlier query, so the Result cache answers few of them and the rest scan. On a normal hour, 12 queries a second bring about 12 × 0.095 ≈ 1.14 GB/s. One cluster scans 1 GB/s, so it already falls behind; two keep up.

At 9 a.m., four times as many arrive: about 4.6 GB/s. When the GB arriving outrun the scan rate, the queue grows every second, and each analyst waits for everyone ahead of them. Caching doesn't help here, because the queries are all different. Only more clusters do.

An exploration query scans 60 GB. On a shared warehouse with 5 clusters, one of them alone puts 12 s of scanning ahead of every analyst's query behind it. When the data science team starts exploring, about 0.48 of them arrive a second: 0.48 × 60 ≈ 29 GB/s, far more than the analysts' 1.1 GB/s. On a warehouse the analysts share, that scan rate is gone for them.

## The reference design

The Client has a Timeout of 120,000 ms. The App Server (2 Go instances) runs the analysts' queries on a Snowflake Data Warehouse with 5 instances: 5 GB/s. The Microservice (2 instances) runs the notebooks' queries on a second Snowflake Data Warehouse with 32 instances: 32 GB/s. Both keep Concurrency per instance at 8 and Auto-suspend after at 60 s.

On a normal hour it costs about $7,580 per month: $140 for the App Servers, $140 for the Microservice, $7,300 for the analysts' warehouse and $0 for the notebooks' warehouse. No exploration query arrived in that hour, so the second warehouse stayed suspended and billed nothing. The p95 is about 417 ms, with no errors.

"9 a.m.: four times the queries for 40 s" brings about 4.6 GB/s to the analysts' warehouse: 5 GB/s scans them at about 91%. The p95 is about 632 ms, with no errors. With 4 instances, the same rush reaches a p95 of about 6.2 s; with 2, about 50 s.

"The data science team starts exploring the full history" sends about 29 GB/s of exploration to the notebooks' warehouse, which scans 32 GB/s, so it keeps up and every exploration query ends well inside the 120 s timeout. The analysts' warehouse doesn't see them: the p95 stays at about 417 ms, with no errors. The exploration queries take about 60 s each, but they are under 4% of the queries, so they sit above the p95.

The budget is judged on a normal hour. While the notebooks' warehouse works, it bills like any other: in the hidden tests the same design's monthly rate reads about $43,400 and $53,900 (a single exploration query arrives during the rush, too). Auto-suspend is what makes that a cost of the busy hours only.

Why not one shared warehouse with more clusters? Every 60 GB scan holds up the analysts' queries behind it. With 5 instances, even the rush fails (p95 about 12.3 s), because the odd exploration query of a normal day lands in it. With 10 instances, the exploration test refuses about 31.9% of the queries; with 36 instances, it costs about $52,840 and the exploration test still reaches a p95 of about 8.1 s. The analysts keep a shared warehouse busy all day, so all its clusters bill all day, while a separate warehouse for the notebooks bills only while they explore.

For the cost challenge, the search's cheap designs swap the analysts' warehouse for another product, which the statement rules out (the company runs Snowflake), so this problem has no challenge.

## Why the first try breaks

A common first try wires both the App Server and the Microservice to one Snowflake Data Warehouse with 1 instance. It costs about $1,740 and already fails "A normal working hour": about 1.14 GB/s arrive at a cluster that scans 1 GB/s, the queue grows all hour, and the p95 reaches about 11.4 s.

At 9 a.m. about 4.6 GB/s arrive at 1 GB/s. The queue fills in the first seconds of the rush and the warehouse refuses what doesn't fit: about 68.5% of the queries fail, with a p95 of about 66 s.

When the notebooks start exploring, each exploration query puts a minute of scanning ahead of everyone. The analysts' queries wait behind them, the queue fills, and the warehouse refuses what doesn't fit: about 97.6% of the queries fail, with a p95 of about 61 s.

The point is this: a warehouse's capacity is the GB it scans a second, clusters × scan rate, and you size it for the rush, not the average. And a workload that scans the full history doesn't belong with one that needs answers in seconds: give it its own warehouse, and let auto-suspend make that cheap.

## What the simulator leaves out

Real multi-cluster warehouses add and remove clusters as the queue grows and shrinks (between a minimum and a maximum); here the clusters are fixed, and the card bills all of them while any query runs. Resuming a suspended warehouse takes a few seconds in real life; here it is instant. Here a cluster's queries share only its scan rate, in arrival order; real queries also share CPU and memory, and a scheduler interleaves them. A bigger warehouse size (a faster Scan rate per query) costs as much as the clusters it replaces, which is why the statement fixes the size. Queue timeouts, per-user routing rules and the correctness of the data itself are not simulated: separating the workloads is drawn as wiring. Snowflake's price is the registry's teaching figure ($2 per credit), and time is scaled down: the rush lasts 40 s and the exploration 2 minutes.

## Things to try

- Set the analysts' warehouse to 2 instances and run the 9 a.m. rush. Watch the queue grow second by second.
- Wire the Microservice to the analysts' warehouse instead, with 10 instances, and run the exploration test.
- Set Concurrency per instance on the first try to 32 and run "A normal working hour". Why doesn't it help?
