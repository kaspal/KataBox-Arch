# Editorial

## Where the load goes

Measure the database in busy connections. A PostgreSQL card has a concurrency of 100: it works on 100 queries at once, and the rest wait in line. Reading one order takes about 5 ms. A page of 576 KB takes 5 × (1 + 576 / 64) = 50 ms at the median, and about 64 ms on average once the slow tail is counted.

The app sends 1,400 queries a second at about 6 ms each: roughly 9 busy connections. The export sends 600 pages a second at about 64 ms each: roughly 38. Together that is under half of one database, and the visible test passes on a single primary.

"The export carries 3× the data" keeps the same night window, so the export reads pages three times as fast: 1,800 a second, roughly 115 busy connections. That is more than one database has. Once every connection is busy, every new query waits in the same line, and the app's quick reads wait behind export pages that each take 50 ms or more.

The export only reads, and it can live with data a few milliseconds old. Those reads don't have to land on the card that must take every write.

## The reference design

An AWS ALB sends the app's traffic to 2 Go App Servers, wired with Query to a PostgreSQL SQL Database: the primary. An Airflow Scheduler publishes the export's pages to a RabbitMQ Queue. A Celery Worker with 4 instances, a concurrency per instance of 64 and a service time of 2 ms consumes them and is wired with Query to a second PostgreSQL SQL Database, which the primary feeds with Replicate. "Read from" stays on "Primary and replicas". This costs about $1,055 per month: $260 for each database, $280 for the Workers, $140 for the App Servers, $70 for the Queue, $25 for the ALB and $20 for the Scheduler.

The two databases form one cluster. Writes go to the primary; reads spread over the primary and the replica. In "A night with the export running" the primary is 24% busy and the replica 23%, where a lone primary would be 47% busy. The p95 is about 104 ms, which is mostly the export pages' own read time, with no errors. About 0.09% of the reads are stale: an app read that reached the replica within its replication lag of 50 ms after the order was updated.

In "The export carries 3× the data" each database takes about half of the export's roughly 115 busy connections, plus its share of the app. Neither runs out, and the p95 is about 132 ms with no errors.

"A replica fails during the export" kills the replica 20 seconds in, for 40 seconds, in a run of 90 seconds. This test allows 3% errors and 97% availability. For about two seconds, until the health checks notice, reads still go to the dead replica and fail: 0.67% of the requests over the whole run. Then the primary takes every read. At the normal export rate one database can do that, as the first try shows. Export pages caught by the failure are tried again from the Queue, which costs them a few seconds. The p95 is about 106 ms.

For the cost challenge, a proof design comes in at 78% of the reference's cost ($825), under the 80% target. It keeps both databases but runs 1 App Server, 2 Worker instances and a Temporal Scheduler instead of Airflow, which is billed per action and costs almost nothing at this rate. It can't drop either database: one card can't carry the tripled export together with the app.

## Why the first try breaks

A common first try is the same design with one PostgreSQL SQL Database: the App Servers and the export Workers both read the primary. It costs about $795 and passes "A night with the export running" with a p95 of about 104 ms and the primary 47% busy.

In "The export carries 3× the data" the primary runs out of connections within the first seconds. The export pages pile up in the Queue faster than the database can read them, so their wait grows every second, and the p95 reaches about 11.8 seconds. The app suffers with it: its reads and updates wait in the same line as the export pages, the App Servers' slots fill up with requests waiting on the database, and from about 3 seconds in, around 10% of all requests fail. Over the whole test that is 9.87% errors, every one of them a shopper's request.

Turning the export Workers down so they hold fewer connections doesn't save it: then the export can't read 1,800 pages a second, and the pages waiting in the Queue count in the p95 too.

The point is this: an analytics read and a shopper's read compete for the same connections, and the heavy one wins. Give the heavy, read-only work its own copy of the data, so the primary keeps its connections for the app and for writes.

## What the simulator leaves out

In a real setup you point the export at the replica's own address, so the primary never sees it. The simulator spreads every read, the app's and the export's, over all the healthy members of the cluster, so here the replica takes half the read load instead of taking the export alone. The replication lag is a fixed 50 ms: a replica falling behind while it replays a heavy write load isn't simulated, nor are queries on a replica cancelled because they conflict with that replay. Disks, the buffer cache that a big scan pushes out, vacuum and the write-ahead log aren't simulated either. A real export runs for hours; the tests simulate a minute or a minute and a half of it at its working rate, and the Scheduler's interval doesn't change the load. Writing the analytics files isn't part of the problem. Raising a database's concurrency per instance costs nothing here; a real database has a fixed number of cores and disks, so it would not buy capacity.

## Things to try

- Set "Read from" on the reference's primary to "Primary only" and run the 3× test. Compare it with the first try.
- Give the first try's Celery Worker 1 instance and run the visible test. Watch the export pages' wait.
- Run "A replica fails during the export" on the reference and look at the seconds right after the kill.
