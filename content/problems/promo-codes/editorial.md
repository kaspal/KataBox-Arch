# Editorial

## Where the load goes

At 2,000 rps, 1,900 requests a second are lookups and 100 are redeems. The lookups spread over 5,000 codes, but a few popular codes get most of them, and a code's details are tiny: all 5,000 fit in a small cache many times over. The redeems are few, but 30 a second land on the same code. When the influencer test multiplies the traffic by 6, that is 12,000 rps: 11,400 lookups and 600 redeems a second, 180 of them on that one code.

So the load splits into three separate problems. The lookups are many, repeated and read-only. The redeems on the hot code race each other for the same count. And everything that writes goes through one primary database, which a hidden test kills.

## The reference design

An AWS ALB sends shoppers to 2 Go App Servers. Lookups go cache-aside through a Redis Cache with 0.5 GB of memory and a TTL of 60 s. The data lives in PostgreSQL: a primary with "Conditional writes (check and set in one step)" on and a Failover time of 10 s, wired with a Replicate edge to a second PostgreSQL with the same settings. This costs about $691 per month: $260 for each PostgreSQL, $140 for the App Servers, $25 for the ALB and $6 for Redis. On a normal day the p95 is about 22.3 ms, with no errors.

With conditional writes, the redeem is one statement: `UPDATE … SET uses_left = uses_left - 1 WHERE code = ? AND uses_left > 0`. The database checks the count and takes one off in the same step, so when two redeems race for the last use, one wins and the other is told "this code has run out".

"An influencer shares a code" multiplies the traffic by 6 from 15 seconds in, for 30 seconds, and allows no conflicts at all. The cache absorbs the extra lookups, the App Servers keep up, and the conditional write keeps the hot code within its limit: the p95 is about 21.8 ms, with no errors and 0 conflicts.

"The database primary fails" kills the PostgreSQL primary 20 seconds in, and allows 2% errors and 98% availability. For about 10 seconds, until the replica is promoted, about 5% of requests a second fail. After that the replica is the new primary and everything works again. The test ends with about 0.89% errors and a p95 of about 21.4 ms.

The cache is allowed to be a little stale. On a normal day about 27.6% of lookups come from a cached copy older than the code's last redeem, and about 58% during the influencer spike, so the remaining-uses count shown at checkout can be a little behind. That is fine: the lookup is only a hint, and the conditional redeem is what enforces the limit.

This problem has no cost challenge. No cheaper design passed every test: the reference is already lean, and its two PostgreSQL cards are most of the bill for a reason.

## Why the first try breaks

A common first try is the same edge and 2 Go App Servers, with one PostgreSQL, no cache and no conditional writes. It costs about $425 per month and is Accepted on "A normal day", with a p95 of about 29.9 ms. But it already over-redeems: 594 conflicts happen in that run; that test just doesn't judge them.

In the influencer test it breaks in two ways at once. Every lookup goes to the database, and the App Server holds its slot while it waits. On a normal day the 2 App Servers are about 22% busy; 6 times the traffic needs about 130% of them. They fill up, about 20.6% of requests fail and the p95 climbs to about 140 ms. Meanwhile the read-then-write redeems race on the hot code: 3,692 conflicts, each one a use the shop never meant to give away.

In the failover test, about 66.8% of requests fail. With no replica, nothing takes over when the primary dies, and every request that needs the database fails for the rest of the run.

The point is this: three separate habits. Cache read-mostly lookups. Enforce a limit with a conditional write, not read-then-write. Keep a replica to fail over to. Each one fixes a different failure, and none of them stands in for another.

## What the simulator leaves out

A real conditional UPDATE on one row takes a row lock, so redeems on the hot code queue up behind each other; here that serialisation costs no extra time. Per-user limits ("one use per customer"), code expiry dates and fraud checks are not simulated either.

## Things to try

- Turn off "Conditional writes (check and set in one step)" in the reference and run the influencer test. Count the conflicts.
- Remove the Redis Cache from the reference and run the influencer test again. Watch how busy the App Servers get.
- Set the cache's TTL to 5 s and run "A normal day". Compare the stale reads and the p95 with a TTL of 60 s.
