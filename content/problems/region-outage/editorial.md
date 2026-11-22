# Editorial

## Where the load goes

At 6,000 requests/s, about 5,400 are reads and 600 are writes; spread over three Regions, each serves about 2,000 a second.

A local read takes about 10 ms: Go's 5 ms plus PostgreSQL's 5 ms. A write from a Region without the primary crosses twice, 70 ms each way, about 150 ms in all. About 400 writes a second cross, some 6.7% of requests, more than 5%, so the p95 is one of them: about 150 ms, inside 250 ms. Each holds an App Server slot the whole time, so those Regions are twice as busy.

Capacity is easy. The hard part is that a whole Region can fail at once.

## The reference design

Route 53 DNS, with a TTL of 30 s, spreads customers over three Regions: us-east, eu-west and ap-south. Each has an AWS ALB, a Go App Server with 2 instances and a PostgreSQL member. The primary is in us-east, and the members in eu-west and ap-south are replicas wired to it with Replicate edges. This costs about $1,608 per month, of which DNS is about $62, with a p95 of about 155 ms.

About 8% of reads are stale on a normal day, because the replicas are 50 ms behind; the problem judges staleness only during the split.

"The primary's region goes dark" loses the whole Region holding the database primary at 30 seconds, for good, in a 120-second run. It allows 8% errors, 92% availability and a p95 of 1 s. The test finds that Region by itself. DNS keeps sending a third of the traffic there until health checks notice, 2 to 4 seconds later, and those requests fail, with the ones in flight. Writes fail until a replica is promoted, after PostgreSQL's default failover time of 30 seconds. About 3% of requests fail in all, and the p95 of the answered ones is about 43 ms.

"The primary's region is cut off for 20 s" splits that Region from 20 seconds, for 20 seconds: it keeps running, but nothing crosses its border. It has the same limits, plus at most 10% stale reads. Requests sent there fail until routing learns of the split, and then DNS sends everyone to the other two Regions. Reads go on from their replicas. The primary is alive, only out of reach, so nothing is promoted, and writes fail until the split heals. About 4.3% of requests fail, and about 6.6% of reads are stale.

## Why the first try breaks

A common first try is one Region, us-east, with two Availability Zones, each holding a Go App Server of 3 instances and a PostgreSQL member: the primary in one, its replica in the other. It costs about $1,208 per month and passes "Normal day" with a p95 of about 32 ms.

When its Region goes dark nothing is left: about 75% errors, every request from 30 seconds on. The split is as bad. The customers and DNS are outside the Region, so for 20 of 60 seconds no request gets in: about 33% errors. Zones protect you from a data centre failing, not from a Region failing.

## What the simulator leaves out

The simulator doesn't cache DNS answers when it routes: every request follows the current answer. Real clients keep an answer for the TTL, so they make far fewer queries, but they also keep going to a dead Region until it runs out. The bill does count that caching: only about 1 request in 100 is billed as a query. Writing in every Region at once (multi-primary with conflict resolution) isn't simulated, and data sent between Regions isn't billed.

## Things to try

- Make eu-west's member the primary and run both hidden tests again. They follow the primary wherever it is.
- Delete ap-south and run "The primary's region goes dark". One Region is then left to carry all the traffic.
- Add a Redis Cache in each Region and watch the stale reads in the split test.
