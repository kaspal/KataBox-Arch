# One Data Platform, Two Jurisdictions

## The product

You run the data platform of a fitness app with users in the United States and in the European Union. Every tap in the app sends an event to your collectors. The events go into a stream, a stream processor checks each user's consent record and writes the event to a lake table, and analysts in the US watch a global dashboard built from aggregates.

Your EU users' contracts say where their data may be kept: **an EU user's raw events, and their consent record, must be stored only in EU Regions**, and that holds on a bad day too, when something fails over. The legal team has cleared the aggregates (counts per country and day, with no user in them) to be stored in the US. US users' data has no such rule.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 1,000 requests/s: 995 events and 5 dashboard loads |
| p95 latency (an event: from send until it is in the lake) | 5 s or less |
| Availability | 99% or more |
| Errors | 1% or less |
| Events lost | none |
| EU data stored outside the EU | never (0 residency violations) |
| Budget | $21,500 per month |

## The traffic

- **A US user's app sends an event** (59.5%).
- **An EU user's app sends an event** (40%). Its data must stay in EU Regions.
- **An analyst opens the global dashboard** (0.5%).

Both kinds of event go the same way. The request may pass a DNS card and a load balancer and reaches your collectors (App Servers), which publish the event (about 2 KB) to a stream and answer. A Stream Processor consumes it, reads the user's consent record from a NoSQL Store, and writes the event to a Lakehouse Table. An event's latency runs until it is in the lake.

A dashboard load reaches an App Server that queries a Data Warehouse holding the aggregates; each load scans about 1 GB, and about 30% repeat a recent query. A warehouse's instances share their scan rate among the queries running at once. The job that builds the aggregates every hour is not part of the tests.

## Regions and residency

Draw a Region frame for each cloud region you use, and give the Region a **Residency tag** in its inspector: `eu` for an EU region, `us` for a US one. Each time a stream, a NoSQL Store, a lake table or a warehouse stores or serves an EU user's event, the test checks the Region it sits in: outside a Region tagged `eu`, that is a residency violation. A card in no Region at all counts as outside. App Servers and Stream Processors only pass the event on and are not checked.

Nothing routes an event by where its user lives: a DNS card sends apps to the Regions behind it, US and EU users alike, and a card wired to several others takes turns between them. Your design must make sure that wherever an event lands, everything that keeps it is in the EU.

A call from one Region to another takes about 70 ms each way in the simulator.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The Region with the consent table's primary is lost** and **An EU-only audit day: the consent table's primary fails**. In the audit test every event comes from an EU user, at the same total rate. Each hidden test runs 3 minutes and allows 8% errors and 92% availability while things fail over, but no residency violations and no lost events. Your design has to pass all three.
