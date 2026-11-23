# Editorial

## Where the load goes

30,000 lookups a second all end in one read of the IP table. The table never changes while the service runs, so there is nothing to keep in step: every copy is as good as any other. That makes copies cheap, and the real questions are how many App Servers you need and where the copies live.

With two Regions and DNS spreading callers over them, each Region carries about 15,000 rps on a normal day. The hidden tests take that comfort away. In the busiest hour the traffic doubles to 60,000 rps, so each Region gets 30,000. When a Region goes dark, the one left gets all 30,000 rps. Either way, one Region must be able to carry the full normal peak on its own.

## The reference design

Route 53 DNS, with a TTL of 30 s, spreads callers over two Regions, us-east and eu-west. Each Region has its own AWS ALB, Go App Servers with 6 instances and a Redis Cache with 2 GB of memory and a TTL of 0, so the entries never expire. Each Cache holds the whole IP table, and each App Server reads only from the Cache in its own Region. This costs about $1,279 per month: $420 of App Servers per Region, about $317 for Route 53 (billed per query, and 30,000 queries a second add up), $25 for each ALB and $36 for each Redis Cache.

On "A normal day" the p95 is about 19.8 ms with no errors, and the App Servers are about 28% busy.

"The busiest hour of the week" doubles the traffic from 15 seconds in, for 30 seconds. Each Region now carries what one Region would carry alone, and the p95 stays at about 19.4 ms with no errors.

"A Region goes dark" cuts us-east off at 30 seconds, for the rest of a 120-second run. This test allows 8% errors, 92% availability and a p95 of 1,000 ms. About 0.83% of requests fail, all in the first 2 seconds or so: until the DNS health checks notice, half the requests each second still go to the dark Region. Then everyone is sent to eu-west, which carries the full load on its own, because each Region was sized for all of it.

There is room to save. A design with 4 App Server instances per Region and 0.5 GB Caches passes every test for about three quarters of the reference's cost: the table still fits, and 4 instances can still carry the full load alone. The cost challenge asks you to get to 75% of the reference or below.

## Why the first try breaks

A common first try is one Region, us-east, with an AWS ALB, Go App Servers with 4 instances and one Redis Cache. It costs about $341 per month and passes "A normal day" with a p95 of about 18.3 ms. But the App Servers are already about 85% busy.

In "The busiest hour of the week" the traffic doubles and there is no headroom left: the App Servers saturate and about 27.3% of requests fail. In "A Region goes dark" it is worse. From 30 seconds in every request fails, because there is nowhere else to go, and the run ends with about 74.9% errors.

The point is this: a small read-only dataset is cheap to copy. Put a full copy in memory in each Region, size each Region to carry everyone, and let DNS with health checks move callers. Losing a Region then costs seconds, not the service.

## What the simulator leaves out

How the IP table is refreshed, a weekly file load into each Cache, is not simulated. Neither is resolver caching beyond the TTL model: real resolvers sometimes keep an answer longer than they should. Route 53 here round-robins between healthy Regions; anycast and latency-based routing are not modelled, and neither is the network distance from a caller to a Region, since callers sit outside both Regions.

## Things to try

- Lower the DNS TTL to 10 s and run "A Region goes dark" again. Watch how many requests still fail before callers move.
- Cut each Region to 3 App Server instances and run the busiest hour. Find the smallest count that still passes every test.
- Remove the eu-west Region and run "A Region goes dark". Compare the errors with the two-Region design.
