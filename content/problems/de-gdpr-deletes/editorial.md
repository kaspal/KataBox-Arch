# Editorial

## Where the load goes

Every deletion request takes the same path: the App Server publishes it, a Worker takes it and writes three stores in turn. The Worker holds its slot the whole time: 100 ms of its own work, about 50 ms for the Lakehouse Table, about 300 ms for the Data Warehouse and a few milliseconds for DynamoDB, so about 0.46 s per deletion. At 20 a second that keeps about 20 × 0.46 ≈ 9 Worker slots busy, and 20 × 0.3 = 6 of the Data Warehouse's slots.

Two numbers set how fast deletions get done: the Worker slots (deletions in progress at once ÷ time per deletion) and the Data Warehouse's slots (each instance finishes at most 8 ÷ 0.3 s ≈ 27 deletes a second). The slower of the two is the pipeline's speed. Arrivals above it are not refused; they wait in the queue, and the deadline is about how long they wait.

The hidden tests push on each number:
- **A privacy story goes viral** triples the requests to 60 a second for 40 seconds, from second 10. That needs about 60 × 0.3 = 18 warehouse slots and 60 × 0.46 ≈ 28 Worker slots.
- **The lake table's rewrites slow down** adds 3 s to every Lakehouse Table write for 50 seconds, from second 15. Nothing else changes, but each deletion now holds its Worker slot for about 3.5 s: at 20 a second, that is about 70 slots busy.

## The reference design

The web client calls the Go App Servers (2 instances, the default), which publish to an Amazon SQS Queue with "Maximum retries" set to 8 and a Dead-Letter Queue wired from it with Publish. 8 Celery Workers with the default concurrency of 8 (64 slots) consume it and write an Apache Iceberg Lakehouse Table, a Snowflake Data Warehouse with 2 instances (16 slots) and a DynamoDB NoSQL Store. It costs about $3,947 per month: $2,920 for Snowflake, $560 for the Workers, $236 for the lake table's storage, $140 for the App Servers, $60 for SQS and $31 for DynamoDB. A normal day has a p99 of about 1.2 s; the Workers are 16% busy and the warehouse 40%.

During the surge, 60 requests a second meet a warehouse that finishes about 50. The queue grows by about 10 a second for 40 seconds, roughly 400 requests, and then drains at about 30 a second once the surge ends. The last requests of the surge wait about 10 s: the p99 is about 11.6 s, well inside 30 s. The design doesn't have room for the whole surge, and it doesn't need it: it needs a backlog that drains before it gets old.

When the lake slows, the 64 Worker slots finish about 64 ÷ 3.5 ≈ 18 deletions a second, close to the 20 arriving. A small backlog builds and clears: the p99 is about 7.5 s.

Retries and the Dead-Letter Queue are there for failures: no hidden test makes a store fail, and none of the reference's requests is retried. With 8 retries a request keeps trying for about 2 minutes, long enough to ride out a store restarting, and whatever still fails waits in the Dead-Letter Queue instead of being lost. A deletion is safe to retry: deleting a user who is already gone from a store changes nothing.

There is no cost challenge for this problem: the search found a design at 91% of the reference's cost ($3,597), not under the 80% a challenge needs.

## Why the first try breaks

A common first try is the same path with the defaults: 2 Celery Workers (16 slots), one Snowflake instance (8 slots), 3 retries and no Dead-Letter Queue. It costs about $2,067 per month and passes "A normal day" with a p99 of about 1.4 s. But it is already busy: the Workers are 69% busy and the warehouse 81%.

In the surge, the one warehouse instance finishes about 25 deletes a second while 60 arrive. The queue grows by about 35 a second, some 1,400 requests by the time the surge ends, and afterwards it drains at only about 5 a second. Requests wait more than a minute, and the p99 is about 62.5 s. Adding a second warehouse instance isn't enough on its own: 16 Worker slots can't hold the 28 deletions in progress the surge needs, and the p99 is still about 41 s.

When the lake slows, each deletion holds a Worker slot for about 3.5 s, and 16 slots finish fewer than 5 a second while 20 arrive. Nothing fails and nothing is lost: the requests just wait, and the p99 is about 38 s. Eight Workers on their own fix this test (p99 about 8.6 s) but not the surge.

The point is this: a deadline is a throughput problem. Find the slowest stage (Worker slots when a store slows down, the warehouse when the traffic surges), give it enough room that a backlog drains well inside the deadline, and remember that one slow store holds the slots every other store's deletes need. A design with 4 Workers and 2 warehouse instances also passes, but the slow lake pushes its p99 to about 27 s, with little room left.

## What the simulator leaves out

Nothing is actually erased. A deletion is three write calls with a fixed cost; the simulator doesn't find the user's rows, doesn't rewrite the lake table's files and doesn't check that any data is gone. In a real Iceberg, Delta or Hudi table, a delete rewrites every file that holds one of the user's rows (or writes delete files that are merged later), and old snapshots still hold the data until they are expired and cleaned up. Those costs depend on the table's layout and aren't modelled; the "slow lake" test stands in for them as extra time per write.

Real erasure pipelines batch the work: they collect a day's requests and run one delete per table for all of them, which is far cheaper than one rewrite per user. The simulator has no batching of writes, so this problem sizes the per-request path. Backups, logs, caches and copies held by partners also hold the user's data, and none of them is here.

The Worker writes the three stores one after another. A real design might give each store its own queue and Workers, so a slow lake doesn't hold up the warehouse and DynamoDB deletes; the simulator runs a caller's calls in order, so here a slow store slows every deletion.

The deadline is scaled: 30 seconds stands for 24 hours, and the tests last a minute or two. Over a real day, the same arithmetic holds with bigger numbers: a backlog that grows for hours still has to drain before the oldest request is 24 hours old.

## Things to try

- Drop to 4 Workers and run the slow lake again; then try an autoscaling group for the Workers and see how long it takes to react.
- Raise the Data Warehouse's "Concurrency per instance" instead of adding an instance, and compare the cost.
- Give the 2 default Workers a "Concurrency per instance" of 32 instead of adding instances, and compare the cost: their slots spend most of their time waiting on the stores.
