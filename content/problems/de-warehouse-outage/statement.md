# The Warehouse Goes Down

## The product

Your mobile and web apps send their events (screens viewed, buttons tapped, purchases) to a fleet of event collectors. Each collector gathers about 500 events and sends them on as one batch, and every batch must end up in the data warehouse, Snowflake, where the analysts query it. On a busy hour that is 30 batches a second; at the evening peak, 45.

Snowflake is not always there. A maintenance window, a bad deploy of the loading job or a cloud incident can take it away for a while. When that happens, no batch may be lost, and once Snowflake is back the warehouse has to catch up quickly: an analyst who opens a dashboard after the incident should see fresh data again, not data still stuck far behind.

Real outages like this last tens of minutes, and the catch-up an hour or so. The simulator's percentiles stop at about a minute, so the problem is scaled down: here the warehouse is down for **60 s**, and the backlog must be worked off within **50 s** of its return.

## What you must meet

| Goal | Target |
| --- | --- |
| Traffic | 30 batches/s on a busy hour, 45 at the evening peak |
| Freshness on a normal day (p95, from the collector until the batch is in the warehouse) | 10 s or less |
| Freshness in a test with an outage (p95 over the whole test) | 60 s or less |
| Catch-up after an outage at the peak | from 50 s after the warehouse is back, in every second, 95% of the batches loaded waited under 60 s |
| Availability | 99.9% or more |
| Errors (batches refused) | 0.1% or less |
| Lost batches | 0 |
| Budget | $11,000 per month |

## The traffic

- **Load a batch of events** (100%). A collector sends the batch; it may pass a load balancer and reaches your App Servers. From there it goes to the Data Warehouse, either directly (the App Server writes it and answers once it is loaded) or through a Stream: the App Server publishes the batch and answers at once, and a consumer (a Data Connector such as Kafka Connect, a Worker or a Stream Processor) takes it from the Stream and loads it. There are 1,000,000 different batch keys, about equally likely.

Latency is measured to completion: from the moment the collector sends the batch until it is in the warehouse.

A load into Snowflake takes about 300 ms at the median, with a slow tail. An instance (one X-Small cluster) runs 8 loads at once; keep **Concurrency per instance at 8** and add instances for more loads. Snowflake bills an instance for every second it is busy, plus its Auto-suspend time, so a warehouse loading all day costs the full month.

When Snowflake is down, a load fails at once. A Stream keeps every record until it has been loaded: a consumer whose load fails tries again later, waiting 1 s, then 2, 4, 8, 16 and at most 32 s between tries, and the Stream's Retention (7 days by default) outlasts any outage here. A Stream hands out one record per partition at a time.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: 30 batches a second for 60 s.

Two more tests are hidden, and you see only their names: **The warehouse is down for 60 s** and **The warehouse is down for 60 s at the evening peak: 1.5 times the events**. Both run for 600 s, and the outage starts 20 s in, so the catch-up limit above applies from 130 s. Your design has to pass all three.
