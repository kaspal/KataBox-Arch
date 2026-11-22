# Editorial

## Where the load goes

At 12,000 requests/s with 95% reads, about 11,400 requests/s are redirects and about 600 are new links. The reads are the whole problem, and they are very uneven: with Zipf 1.1 over 10 million links, a small slice of hot links gets most of the clicks. That shape is exactly what a cache is for. If most redirects end at the cache, the database sees only misses and writes, and App Server threads spend far less time waiting on it.

## The reference design

A Load Balancer (NGINX) spreads traffic over 8 Go App Servers. Each App Server checks a Redis Cache of 8 GB first. A hit ends the request; a miss reads PostgreSQL and fills the cache. 8 GB holds about 8 million 1 KB entries, far more than the hot set. PostgreSQL runs as a primary and one replica joined by a Replicate edge, with a failover time of 10 seconds. Reads spread over both members; writes go to the primary.

This costs about $1,429 per month and has a p95 near 22 ms on a normal day. It absorbs the viral spike. When the primary dies, about 1.1% of requests fail while the failover runs, then the replica is promoted and traffic recovers.

## Why the first try breaks

A common first try is a Load Balancer, 4 Go App Servers and a single PostgreSQL database, with no cache and no replica. It passes both visible tests with a p95 near 29 ms, so it looks finished.

The hidden tests find two gaps. "A link goes viral" sends 4 times the traffic for 15 seconds, with a p95 limit of 120 ms. Every click lands on the one database, it saturates, App Server threads pile up behind it, and about 39% of requests fail. "The primary database dies" kills the SQL primary at 30 seconds and allows up to 2% errors. With no replica there is nothing to promote, and from then on every request that needs the database fails: about 50% of the whole run.

## Things to try

- Delete the replica and run "The primary database dies" again. Then lower the failover time and compare the error rate.
- Shrink the Cache to 1 GB and watch the hit rate and p95 on "One link everyone clicks".
- Remove App Servers one at a time and find the smallest count that still survives the viral spike.