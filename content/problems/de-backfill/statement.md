# Backfill Beside the Live Stream

## The product

You run the event pipeline of a product analytics company. Apps send their users' clicks and page views to your ingest API, which groups them into batches of 500 events and publishes each batch to a Stream. A Stream Processor reads the batches and loads each one into a Data Warehouse, where the customers' dashboards read them. Customers expect a click to show up on their dashboard within seconds.

This week the team found a bug that dropped a field from three months of events, so the fixed history has to be loaded again. The backfill reads the old events from the archive and sends them through the same ingest API, Stream, Stream Processor and warehouse as the live events: one pipeline, one set of tables. It is paced to use the room the live traffic leaves. Nobody is watching the backfill; it only has to be accepted and loaded eventually, without losing a batch.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 40 batches/s (24 live, 16 backfill) |
| p95 latency (see below) | 10 s or less |
| Availability | 99.9% or more |
| Errors | 0.1% or less |
| Lost batches | none |
| Budget | $7,500 per month |

## The traffic

- **Load a batch of live events** (60%). The request may pass a load balancer and reaches your App Servers (the ingest API), which publish the batch to the Stream; the Stream's consumer loads it into the Data Warehouse. A live batch is measured by **completion**: from the moment the API receives it until it is loaded in the warehouse. That is how fresh the dashboards are.
- **Load a batch of historical events (backfill)** (40%). The same route. A backfill batch is measured by **response**: from the moment the API receives it until the Stream has stored it and the API answers. Once it is in the Stream, the backfill can wait.

So the p95 target applies to the completion of the live batches and to the acknowledgement of the backfill batches.

A Data Warehouse is built for big reads, not small writes: loading one batch holds a warehouse slot for about 300 ms (slower in the tail), and a Snowflake warehouse has 8 slots per instance. The Stream Processor keeps its slot while the load runs, and the Stream hands out one batch per partition at a time.

## The tests

One test is visible. You can run it as often as you like:

- **A normal peak hour, with the backfill at its pace**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The backfill runs at 5 times its pace, at the peak** and **A live burst: 2.5 times the live events**. The whole backfill takes days; a test sees one minute of it. Your design has to pass all three.
