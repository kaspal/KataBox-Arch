# Editorial

## Where the load goes

20,000 requests a second arrive, and the provider takes at most 200 calls a second. Without a cache, 99% of the requests would be refused. So the cache does the real work, and the question is how often it has to ask the provider.

There are 40,000 cities. With a TTL of 600 s, each city expires about once every 10 minutes, so a warm cache asks the provider for about 40,000 / 600 ≈ 67 forecasts a second, far under the limit of 200. A cold cache is a different story: refilling all 40,000 cities at 200 calls a second takes 200 s, and in the meantime most requests miss.

The hidden tests take each half of that away. When the provider is down for 40 seconds, about 67 cities a second keep expiring, and nobody can refresh them. When the cache node restarts, the copy itself is gone.

## The reference design

An AWS ALB sends the traffic to 8 Go App Servers with a call timeout of 200 ms. They read from a Redis Cache of 2 GB with a TTL of 600 s, "Serve stale on error for" set to 3,600 s, the stampede guard on and a failover time of 5 s. It is wired with Replicate to a second Redis Cache with the same settings. On a miss, the App Server calls the provider and the answer fills the cache. This costs about $657 per month: $560 for the App Servers, $25 for the ALB and $36 for each Redis. A normal day has a p95 of about 19.0 ms, no errors, and 0.58% stale reads.

"The weather provider is down for 40 s" stops the provider from 20 seconds in, for 40 seconds, in a run of 90 seconds. When a city has expired and the provider call fails, the cache serves the expired copy instead of an error, and the requests waiting on the same city's miss get that copy too. The p95 is about 22.3 ms, with no errors and 2.98% stale reads. The price of serving stale on error is that the request first waits for the call to fail. As more cities expire during the outage, more requests wait at once. That is why the App Servers have headroom and a short call timeout: with 6 instances and a 300 ms timeout, the last 10 seconds of the outage reached up to 24% errors, because every App Server slot was full.

"A cache node restarts" kills the first Redis Cache 20 seconds in, for 5 seconds. This test allows 3% errors and 97% availability. The design fails about 1.65% of the requests, all in the roughly 2 seconds before the health checks notice the dead node; then the replica takes over. When the node comes back, it resyncs the replica's copy instead of starting empty, so the provider never sees a flood. The p95 stays at about 19.0 ms.

For the cost challenge, a proof design comes in at 70% of the reference's cost, under the 75% target. It keeps the same shape and the same cache settings, with fewer App Servers and much smaller caches: 40,000 forecasts take very little memory.

## Why the first try breaks

A common first try is the same edge with 6 Go App Servers and no call timeout, and one Redis Cache of 2 GB with a TTL of 600 s: no serving stale on error, no stampede guard and no replica. It costs about $481 and passes "A normal day" with a p95 of about 18.6 ms.

During the outage, every city that expires has no answer to give: the provider call fails and the request fails with it. About 26.7% of the requests fail, and the p95 climbs to about 519 ms as requests wait on calls that never succeed.

The restart is worse: about 59.3% errors. The cache comes back empty, and 20,000 requests a second miss at once. The provider refuses everything over 200 calls a second, and refilling 40,000 cities at that rate takes minutes.

The point is this: in front of a slow, limited provider, the cache is your availability, not just your speed. An expired copy is better than an error, so keep it for when the provider fails. And never let the cache come back empty: a replica that can take over and resync it keeps the full load off the provider.

## What the simulator leaves out

Redis persistence (RDB snapshots or the AOF log) is not simulated, so a lone Redis always comes back empty. Real stale-while-revalidate answers with the stale copy at once and refreshes it in the background; here the request waits for the failed call first. A real forecast changes on the provider's schedule, not when your TTL runs out. The provider's price per call is not billed, and Retry-After headers are not simulated.

## Things to try

- Turn the stampede guard off and run the outage again. Watch how many more calls reach the provider and how the errors change.
- Set the App Servers to 6 instances and the call timeout to 300 ms, and run the outage. Look at its last 10 seconds.
- Remove the replica and run the restart. Compare its errors with the provider's limit of 200 calls a second.
