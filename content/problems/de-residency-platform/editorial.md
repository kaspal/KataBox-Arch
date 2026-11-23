# Editorial

## Where the load goes

Two things decide this design: which cards hold an EU user's event, and where each of them goes on a bad day.

Count the places first. An event is published to a stream, its consent record is read from a NoSQL Store, and the event is written to a lake table. All three hold EU data, so all three are checked. The App Servers and the Stream Processor only pass the event on. At the peak about 400 EU events arrive every second, but a DNS card can't tell them from the 595 US ones: every collector, stream, consent table and lake that can receive an event will receive EU events. The only way to keep EU data in the EU is to put the whole event path in Regions tagged `eu`. US events have no rule, so they can share it. The warehouse holds only aggregates, which may live in the US; dashboards never touch an event.

Then the bad days. When a Region is lost, the events that would have landed there must land in another EU Region, and that Region must take the whole peak. When the consent table's primary fails, a replica is promoted, so the replica's Region matters as much as the primary's. And a replica isn't only a standby: a Stream Processor reads from the copy in its own Region, or from any copy when its Region has none.

The Stream Processor's size follows from the lookup. Each event holds a slot for about 65 ms when the consent table is in its Region, and about 70 ms more each way when the lookup crosses to another Region. One Region carrying all 995 events a second needs about 65 slots with local lookups and about 205 when every lookup crosses Regions. When only the lookups cross and both Regions still take their half, about 500 events a second, that Region needs about 102. Kafka hands out one record per partition at a time, so the stream needs at least as many partitions as there are busy slots.

## The reference design

A Client with 2 Retries, a Retry backoff of 200 ms and the Backoff jitter set to Full (random wait) calls a Route 53 DNS card (TTL 30 s, Round robin). It points at two Regions, **eu-west** and **eu-central**, both with the Residency tag `eu`. Each holds a whole pipeline: an AWS ALB, 2 Go App Servers, an Apache Kafka Stream with 128 Partitions, an Apache Flink Stream Processor with 32 Instances and the default Concurrency per instance of 4 (128 slots), a DynamoDB NoSQL Store and an Apache Iceberg Lakehouse Table (Partitioning By day, Compaction Hourly). The DynamoDB in eu-west is the consent table's primary, wired with Replicate to the one in eu-central. A third Region, **us-east** (Residency tag `us`), holds an AWS ALB, 2 Go App Servers and a Snowflake Data Warehouse with the aggregates; the Client calls that ALB directly for the dashboard.

It costs about $7,552 per month: $4,480 for the two Flink cards, $1,460 for the warehouse, $472 for the two lake tables, $420 for the six App Servers, $330 for the two DynamoDB tables, $306 for the two Kafka Streams, $75 for the three ALBs and $11 for DNS. A Flink instance has 4 slots and costs $70 a month; more slots per instance cost more in proportion, so the slots are most of the bill. Kafka partitions are not billed in the simulator.

On a normal day each Flink card is about 26% busy, an event's p95 (until it is in the lake) is about 152 ms, and there are no errors and no residency violations. The warehouse is about 54% busy.

"The Region with the consent table's primary is lost" loses eu-west 30 seconds into a run of 3 minutes, for good: its ALB, App Servers, stream, Flink card, lake and the consent table's primary. For a few seconds the DNS card still sends half the apps there, until two health checks in a row find it down; the Client's retries land most of those events in eu-central. The consent replica in eu-central is promoted, still in the EU, and eu-central carries all 995 events a second. The run ends with about 0.12% errors, a p95 of about 158 ms and no residency violations.

"An EU-only audit day: the consent table's primary fails" sends every event from an EU user at the same total rate, so every one of them must stay in the EU. The DynamoDB primary in eu-west is stopped 30 seconds in, for good. The replica in eu-central is promoted, and from then on eu-west's Flink card reads every consent record across the Regions: those events take about 140 ms longer, and the p95 rises to about 286 ms. eu-west's half of the events now holds about 102 of its 128 slots. No errors, no residency violations.

There is no cost challenge for this problem. A search for a cheaper design that passes every test found one at 74% of the reference's cost, but it runs the whole pipeline in eu-central and leaves eu-west with the consent table's primary and no traffic. It passes only because the lost Region in the test is the primary's: losing eu-central would stop every event. That is a gap in the test, not a lesson.

## Why the first try breaks

A common first try keeps the data where it belongs on a normal day. Its DNS card uses Failover (first healthy) routing: every app is sent to a pipeline in eu-west like the reference's, and a second, identical pipeline waits in us-east, where the company already runs its US platform, with a replica of the consent table there for disaster recovery. Every event goes through eu-west, so its Flink card is sized for the whole peak with every lookup crossing Regions: 64 Instances (256 slots, with 256 Kafka Partitions), and the same in us-east. It costs about $12,032 per month, under the budget, and passes "A normal day" with the reference's latency: the US pipeline is idle and no EU event leaves the EU.

When eu-west is lost, the DNS card switches to us-east, the consent replica there is promoted, and every EU event from then on is published to a US stream, looked up in a US table and written to a US lake. The errors stay under the limit (about 5.2%, while the clients' cached DNS answers expire), the latency is fine, and the test fails anyway: about 167,000 residency violations, since each EU event counts once at every card that holds it. In the audit test the US replica is promoted when the primary stops, and eu-west's Flink card looks up every EU user's consent in us-east: about 150,000 violations, one for each event from 30 s on, and a p95 of about 443 ms from the lookups crossing the ocean. The standby moved the data to the place it must never go.

Other first tries break in instructive ways:

- **One pipeline per continent.** The same two pipelines behind a Round robin DNS card fail "A normal day" at once: half the EU events land in us-east.
- **The consent replica in the US.** The reference with its eu-central DynamoDB moved to us-east also fails "A normal day": eu-central's Flink card has no copy in its own Region, so it reads from both copies in turn, and half its lookups go to us-east.
- **Fewer Flink slots.** 24 Instances per Region (96 slots) pass "A normal day" and the lost Region, where eu-central carries the peak with local lookups (about 65 slots). But on the audit day eu-west's half of the events crosses Regions for every lookup and needs about 102 slots: the backlog grows, and the p95 reaches about 12.3 s. 28 Instances (112 slots) pass every test, with an audit p95 of about 698 ms; the reference keeps a little more headroom.

The point is this: residency covers every card that holds the data, and every place it can go. A stream and a lookup table hold events as surely as a lake does, a standby Region becomes the primary on a bad day, and a replica serves reads. Keep all of them in Regions with the same tag, and send out of the EU only what was cleared to leave, such as aggregates.

## What the simulator leaves out

Users have no location: a DNS card spreads apps over the Regions in turn, with no routing by geography, so a design can't send EU events to the EU and US events to the US, as a real platform with geolocation routing or separate endpoints would. That is why the reference keeps US events in the EU too. Residency is checked only when a stream, data store or cache stores or serves a request; copying data by replication is not counted, so the first try's US replica breaks no rule until it is promoted, while in reality it holds a copy of every EU consent record from the start. The processing itself, logs, backups and metrics are not checked. Real residency rules come from contracts and laws such as the GDPR, which often cover processing and access too; this problem's rule is the one in its statement, not legal advice.

The aggregates and the hourly job that builds them are not simulated: the dashboard reads a warehouse with no link to the events. Events already in eu-west's stream and lake when it is lost are out of reach until it returns; the test doesn't read them. A Region is lost all at once and stays lost; real outages are often partial. The 70 ms between Regions is one figure for every pair, while eu-west and eu-central are much closer in reality than eu-west and us-east. Prices are the registry's teaching figures, not quotes.

## Things to try

- Change the reference's DNS card to Failover (first healthy) routing and run "The Region with the consent table's primary is lost". It still passes, since the standby is in the EU, but the errors rise to about 5.2% while cached answers expire, against 0.12% when both Regions already take traffic. Then run the audit day: every event now goes to eu-west and every lookup crosses Regions, about 205 slots against 128, and the backlog grows for the rest of the run.
- Move the reference's eu-central DynamoDB into us-east and run "A normal day". Watch the lookups alternate between the copies.
- Give the reference's Flink cards 16 Instances (64 slots) and run both hidden tests. One Region with the whole peak, or with its lookups crossing Regions, needs more slots than that, and the backlog grows for the rest of the run.
