# The 9 a.m. Dashboard

## The product

Your company has one dashboard everybody opens: yesterday's sales, today's orders, stock by warehouse. Its numbers come from a Data Warehouse (Snowflake) that the data team loads every hour, so a number up to ten minutes older than the warehouse is fine. Most of the day the dashboard is quiet. Then at 9 a.m. the whole company sits down, opens it at once, and the same handful of queries arrive by the hundreds.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 20 tile loads/s on a normal hour |
| p95 latency | 2,000 ms or less |
| Availability | 99.9% or more |
| Errors | 1% or less |
| Budget | $2,000 per month |

## The traffic

- **Load a dashboard tile** (100%). The request may pass a load balancer and reaches your App Servers. They can read the tile from a cache (cache-aside); on a miss, or with no cache, the App Server runs the tile's query on the Data Warehouse and waits for it. There are 400 different tiles (charts times filters), and a few of them get most of the loads (a Zipf popularity of 1.0).

Each tile query scans about 0.5 GB of the warehouse's own tables. With the Snowflake card's defaults (Query overhead 300 ms, Scan rate per query 1 GB/s) that is about 0.8 s per query. One cluster runs 8 queries at once (Concurrency per instance) and queues the rest; each extra cluster (Instances) adds 8 more and its own price. About 60% of the queries repeat an earlier query whose data hasn't changed since, and the warehouse's Result cache answers those in about 30 ms. The Result cache is on by default.

Snowflake bills a cluster only while it runs: the seconds it is busy, each extended by Auto-suspend after (60 s). The client gives up after 5 s.

## The tests

One test is visible. You can run it as often as you like:

- **A normal hour**: the traffic above.

Two more tests are hidden, and you see only their names: **9 a.m.: empty caches, then ten times the traffic for 40 s** and **The warehouse slows down: queries take 3 s longer for 40 s**. Your design has to pass all three. The simulator runs short tests, so the morning rush is scaled down to 40 seconds; a real one lasts longer, which only makes a backlog worse.
