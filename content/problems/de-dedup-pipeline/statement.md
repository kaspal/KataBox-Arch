# Billing Events, Exactly Once

## The product

You run the billing pipeline of a software company that charges by usage. Every billable thing a customer does (an API call over the free tier, a gigabyte stored, a seat added) becomes a billing event. The services that produce them send each event to a Go ingest service, which writes it to a Kafka stream. A stream processor reads the stream, enriches each event (price list, currency, customer account) and writes it as one row into the billing ledger, a table in a data warehouse. At the end of the month the invoices are summed from that table.

Finance has one rule above all others: **every event is in the ledger exactly once**. A missing row is revenue lost; a repeated row is a customer billed twice, and a support ticket, and a refund. So the pipeline must lose nothing and duplicate nothing, even when the stream processor crashes and restarts. Invoices are not urgent, but finance's live revenue dashboard reads the ledger, so an event should reach it within 30 s.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 40 events/s |
| p95 time from event to the ledger | 30 s or less |
| Availability | 99.9% or more |
| Errors | 0.1% or less |
| Lost events | none |
| Duplicate rows in the ledger | none |
| Budget | $4,000 per month |

## The traffic

- **Record a billing event** (100%). The event may pass a load balancer, reaches your App Servers, goes into a Stream, is processed by a Stream Processor and is written to a Data Warehouse. It is measured until it is in the warehouse. Every event has its own id (10,000,000 of them in play).

How the cards behave here:

- The stream processor does no windowing (keep **Window** at `none`): each event is enriched and written on its own. A processor slot waits for its write to finish, so the processor needs as many slots (**Concurrency per instance** × **Instances**) as there are writes in flight, and a Stream hands out one event per partition at a time, so its **Partitions** cap the slots that can work.
- A write to a Data Warehouse node takes about 300 ms, and each node takes 8 writes at a time (the defaults).
- A Stream Processor saves its progress at every **Checkpoint interval**. When it crashes, it restarts from its last checkpoint and processes everything since then again, and those rows reach the ledger a second time, unless the warehouse has **Idempotent writes (dedup)** on (a write keyed by the event id: writing the same event again changes nothing).
- When producers retry, they send the same events again. The simulator does not track event ids, so it treats each resent event as a new event: a retry storm is more load, not more duplicates. Here duplicates come only from the stream processor's replays.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The stream processor crashes** and **A retry storm: 3 times the events for 30 s, and the processor crashes in the middle**. Your design has to pass all three.
