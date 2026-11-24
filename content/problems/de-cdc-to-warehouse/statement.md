# Orders Into the Warehouse

## The product

You run an online shop. Every order lives in a PostgreSQL database: placed, paid, packed, shipped, returned. The analysts want the same orders in the data warehouse, Snowflake, so their dashboards show today's sales and not yesterday's. The shop's own database must not slow down for them, and no change may go missing on the way.

The analysts accept a warehouse that is a few minutes behind, about 5 minutes. The simulator's tests last 60 to 90 seconds and its percentiles stop at about a minute, so the problem is scaled down: here a change must reach the warehouse **within 15 s** of its commit, and the flash sale and the slowdown last 30 s instead of the half hour they would last in real life.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 12 order changes/s |
| Freshness (p95, from the commit until the warehouse has the change) | 15 s or less (5 minutes in real life, scaled down) |
| Availability | 99.9% or more |
| Errors | 0.1% or less |
| Lost changes | 0 |
| Budget | $7,900 per month |

## The traffic

- **Change an order** (100%). The request may pass a load balancer, reaches your App Servers, and they write the change to the SQL Database. Then change data capture takes over: the database publishes each committed change to a Stream, a consumer (a Stream Processor or a Worker) takes it from the Stream and writes it into the Data Warehouse. There are 1,000,000 orders, each about equally likely to change.

Latency is measured to completion: from the moment the user changes the order until the warehouse has the change. The user's own answer leaves at the commit, so the shop never waits for the warehouse.

In the simulator, change data capture is a Publish edge from the SQL Database straight to the Stream. It stands for a connector such as Debezium reading PostgreSQL's write-ahead log; you don't add a separate Data Connector card for it. If the Stream can't take a change, the change waits in the database's log (its "CDC buffer", 300 s by default) instead of being lost.

The warehouse writes each change on its own, and a write takes about 300 ms at the median, with a slow tail. The consumer keeps its slot for the whole write.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A flash sale: 3 times the changes for 30 s** and **The warehouse slows down: every write takes 1.5 s longer for 30 s**. Your design has to pass all three.
