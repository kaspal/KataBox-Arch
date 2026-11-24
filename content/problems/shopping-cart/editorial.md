# Editorial

## Where the load goes

At 6,000 rps, 3,600 requests a second are cart views, 1,800 are cart changes and 600 are checkouts. The views can come from a cache; the 2,400 writes a second must reach PostgreSQL's primary.

The crowd lands on the app tier. Each request holds an app slot for at least its 40 ms of work, plus the time it waits on the cache or the database. When the sale starts, the traffic climbs to 5 times the peak, 30,000 rps, and the autoscaling group adds instances to keep up. A Spring Boot instance with a concurrency of 200 has a 200-thread pool, and each thread that talks to PostgreSQL directly keeps a connection open. So every instance the group adds brings 200 more connections, and PostgreSQL takes at most 1,700. 8 instances hold 1,600 connections; a 9th makes it 1,800, over the limit.

So the question is not only how many app instances you need, but how many database connections they bring with them.

## The reference design

An AWS ALB sends shoppers to Spring Boot App Servers with a concurrency of 200 and 40 ms of work per request, inside an AWS Auto Scaling Group with a minimum of 6 instances, a maximum of 24, a target utilisation of 60% and a scale-out delay of 20 s. Cart views go cache-aside through a Redis Cache with 8 GB of memory and a TTL of 600 s. Every database call goes through a PgBouncer Connection Pooler in transaction mode with 800 database connections, to PostgreSQL with 2 shards.

This costs about $1,093 per month on a normal day: $520 for PostgreSQL, $420 for the App Servers, $98 for Redis, $30 for PgBouncer and $25 for the ALB. During the sale it rises to about $1,263 as the group scales out, still well under the $1,800 budget. In "A normal afternoon" the p95 is about 117 ms, with no errors, and the App Servers are about 40% busy.

The pooler is what lets the app tier grow. However many instances the group runs, PostgreSQL sees at most PgBouncer's 800 connections. In transaction mode a connection is lent to a request only for its transaction and then goes back to the pool, so 800 connections serve many more app threads, most of which are doing something else at any moment.

"The sale starts" ramps the traffic from 1 to 5 times the peak over 180 s. The group keeps adding instances, the pool stays at 800, and the p95 stays at about 117 ms, with no errors.

"A flash deal at noon" steps the traffic up to 2.5 times the peak at 20 s and keeps it there for the rest of the 120 s run. The 6-instance floor has enough room to ride the step while the group grows behind it: the p95 is about 117 ms, with no errors.

There is no cost challenge for this problem. The cheapest design we found that passes every test cost about 93% of the reference: the database is nearly half the bill, and every write needs it.

## Why the first try breaks

A common first try is the same design without the pooler: the App Servers connect to PostgreSQL directly, and the group's minimum is 4 instances. It costs about $923 per month. It passes "A normal afternoon": 4 × 200 = 800 connections, far below 1,700.

It also passes "A flash deal at noon", with a p95 of about 138 ms: at 2.5 times the peak the group only needs about 7 instances, 1,400 connections.

"The sale starts" is where it breaks. From about 120 s in, the group passes 8 instances, and 9 × 200 = 1,800 connections is more than PostgreSQL's 1,700. The database refuses the new connections, and the requests that needed them fail: 3% a second at first, then 9%. Over the whole run 4.0% of requests fail, over the 1% limit. Scaling out, the thing meant to carry the sale, is what breaks it.

The point is this: autoscaling the app tier multiplies database connections. A pooler such as PgBouncer or RDS Proxy holds a fixed pool to the database however many app instances there are, and transaction pooling shares each connection between many requests.

## What the simulator leaves out

Redis persistence is not simulated, and neither is losing carts when Redis fails; here every cart change is written to PostgreSQL. Sessions and inventory checks at checkout are not modelled. The pooler itself never fails, prepared statements in transaction mode cause no trouble, and opening a connection takes no time.

## Things to try

- Remove the PgBouncer card, wire the App Servers straight to PostgreSQL, then run the sale. Watch the error rate once the group passes 8 instances.
- Lower the pooler's Database connections to 200 and run the sale again. Does the pool turn into a queue in front of PostgreSQL?
- Without a pooler, set the group's Maximum instances to 8. Do the connection errors go away, and what happens to the p95 instead?
