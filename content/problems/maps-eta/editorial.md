# Editorial

## Where the load goes

At 4,000 rps, 70% are tiles: 2,800 a second. They are cacheable, and a few busy tiles get most of the requests, so a CDN can answer nearly all of them and object storage sees only the misses. The CDN bills per request, so 2,800 tiles a second is also where most of the money goes.

The other 30% are routes: 1,200 a second. Each one walks 6 hops of the road graph with 3 roads per junction, about 1,100 relationships, so the graph database visits about 1.3 million relationships a second. Then each route calls the traffic feed. At 40 ms per call, 1,200 × 0.04 s keeps about 48 calls in flight, each holding a router slot. That is cheap while the feed answers. The danger is a feed that stops answering: a route that waits about 1 s for a dead feed holds its slot 25 times longer, and 1,200 routes a second would need about 1,200 slots just to wait.

## The reference design

The Client loads tiles from a CloudFront CDN with a TTL of 86,400 s (one day), in front of Amazon S3. Routes go through an AWS ALB to Go App Servers, the routers, with 5 instances, the Call timeout set to 150 ms and the Circuit breaker at errors set to 50%. They query a Neo4j Graph Database with 3 instances. This costs about $6,803 per month: $5,528 for CloudFront, billed per request at 2,800 tile fetches a second, $780 for Neo4j, $350 for the routers, $121 for S3 and $25 for the ALB.

"A normal afternoon" has a p95 of about 96 ms with no errors, and the routers are about 23% busy.

"Rush hour" raises the traffic 2.5 times, from 15 seconds in, for 30 seconds. The CDN absorbs the extra tiles and the routers have room for the extra routes: the p95 stays at about 96 ms, with no errors.

"The traffic feed goes down" stops the feed from 20 seconds in, for 40 seconds, in a run of 90 seconds. The first calls to it time out at 150 ms, and those routes fall back to typical speeds. Within seconds most calls are failing, the breaker opens and the routers stop calling the feed at all: they fail fast. Routes come back faster than usual, with typical-speed arrival times. The p95 is about 73 ms, with no errors.

There is no cost challenge for this problem. The best cheaper design found still cost about 98% of the reference: the CDN bill dominates, and every design has to serve the same tiles.

## Why the first try breaks

A common first try is the same design with 3 routers, no call timeout and no circuit breaker. It costs about $6,663 per month. It is Accepted on "A normal afternoon", with the routers about 39% busy, and it passes "Rush hour" with a p95 of about 102 ms.

When the feed goes down, about 12.3% of requests fail. Every route still waits for the dead feed, about 1 s before the call fails, and holds a router slot the whole time. The slots run out, new routes queue behind them, and many fail. The feed was optional on paper, but the routers treated it as required.

The point is this: a nice-to-have dependency must be optional in practice. A short timeout, a circuit breaker and a fallback (typical speeds) turn its outage into slightly worse answers instead of an outage of your own.

## What the simulator leaves out

The route search algorithm itself is not simulated: every route visits a fixed number of relationships. Map data updates and tile rendering are left out too. The accuracy of the arrival time is not scored, so a typical-speed arrival time counts the same as a live one. The CDN is billed per request only, with no bandwidth charge.

## Things to try

- Remove the CDN and send tiles straight to Amazon S3. Compare the cost and the p95 with the reference.
- Keep the Call timeout at 150 ms but set the Circuit breaker at errors to 0 (off), then run "The traffic feed goes down". How much does the timeout alone save, and what does the breaker add?
- Drop the routers to 3 instances with the timeout and the breaker on, and run all three tests. Is it the extra capacity or the breaker that saves the outage?
