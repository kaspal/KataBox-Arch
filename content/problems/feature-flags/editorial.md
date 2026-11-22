# Editorial

## Where the load goes

At 60,000 requests/s with 99.9% reads, about 59,940 requests/s are flag reads and only about 60 are flips. With 20,000 flags, each flag is flipped about once every 333 seconds on average, while the popular ones are read thousands of times a second. All 20,000 flags at 1 KB fit in about 20 MB. That is the perfect shape for a cache: tiny, hot and rarely changed.

Without one, every read is a database query. A PostgreSQL member works 100 queries at a time at about 5 ms each, roughly 20,000 a second, so even a primary and a replica together are far short of 60,000. The App Servers feel it too: each one holds its slot while it waits on the cache and the database, so a slow database fills their slots.

## The reference design

An ALB Load Balancer spreads traffic over 10 Go App Servers, the Flags API. Each reads a Redis Cache first, with the TTL set to 3 seconds; a miss reads PostgreSQL and fills the cache. PostgreSQL runs as a primary and one replica joined by a Replicate edge; flips go to the primary. This costs about $1,461 per month, with a p95 near 20 ms and about 1% stale reads on a normal day, most of them from the replica's lag.

The TTL is the lesson. At 3 seconds a cached flag is never more than 3 seconds behind its latest flip, and the database still sees only a small stream of misses. "A flip storm" multiplies flips by 5, to about 300 a second, so each flag changes about once a minute. More reads land in the window after a flip, and stale reads rise to about 2.4%, still under the 5% limit. At 5 seconds they would reach about 5.4%, just over it. "The config database is down for 20 s" kills the PostgreSQL primary at 20 seconds for 20 seconds, allowing 1% errors and 99% availability. The cache keeps answering for the flags it holds, so only some misses and the flips fail: about 0.17% of requests.

## Why the first try breaks

A common first try is the same design without the cache: the ALB, 10 Go App Servers and the PostgreSQL primary and replica. Every read becomes a query. The two members cannot carry 60,000 queries a second, the App Servers' slots pile up waiting on them, and about 49% of requests fail even on "Normal day", with a p95 near 209 ms. The flip storm is no better, and with the primary down about 59% fail. Its reads are almost never stale, since there is no cached copy to go old, only a little replica lag, but half of them never arrive.

## Things to try

- Set the TTL back to the default of 300 seconds and run "A flip storm". A flip now takes minutes to show, and the stale limit fails.
- Set the TTL to 1 second and run "The config database is down for 20 s". More misses reach the database, and more of them fail while it is down.
- Cut the App Servers to 5 and see whether the p95 holds.
