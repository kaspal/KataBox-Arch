# Editorial

## Where the load goes

Two things decide this design: which cards hold an EU user's event, and where each of them goes on a bad day.

Count the places first. An event is published to a stream, its consent record is read from a NoSQL Store, and the event is written to a lake table. All three hold EU data, so all three are checked. The App Servers and the Stream Processor only pass the event on. At the peak about 400 EU events arrive every second, but a DNS card can't tell them from the 595 US ones: every collector, stream, consent table and lake that can receive an event will receive EU events. The only way to keep EU data in the EU is to put the whole event path in Regions tagged `eu`. US events have no rule, so they can share it. The warehouse holds only aggregates, which may live in the US; dashboards never touch an event.

Then the bad days. When a Region is lost, the events that would have landed there must land in another EU Region, and that Region must take the whole peak. When the consent table's primary fails, a replica is promoted, so the replica's Region matters as much as the primary's. And a replica isn't only a standby: a Stream Processor reads from the copy in its own Region, or from any copy when its Region has none.

The Stream Processor's size follows from the lookup. Each event holds a slot for about 65 ms when the consent table is in its Region, and about 70 ms more each way when the lookup crosses to another Region. One Region carrying all 995 events a second needs about 65 slots with local lookups and about 205 when every lookup crosses Regions. Kafka hands out one record per partition at a time, so the stream needs at least as many partitions as there are busy slots.

## The reference design

A Client with 2 Retries, a Retry backoff of 200 ms and the Backoff jitter set to Full (random wait) calls a Route 53 DNS card (TTL 30 s, Round robin). It points at two Regions, **eu-west** and **eu-central**, both with the Residency tag `eu`. Each holds a whole pipeline: an AWS ALB, 2 Go App Servers, an Apache Kafka Stream with 256 Partitions, an Apache Flink Stream Processor with 2 Instances and a Concurrency per instance of 128, a DynamoDB NoSQL Store and an Apache Iceberg Lakehouse Table (Partitioning By day, Compaction Hourly). The DynamoDB in eu-west is the consent table's primary, wired with Replicate to the one in eu-central. A third Region, **us-east** (Residency tag `us`), holds an AWS ALB, 2 Go App Servers and a Snowflake Data Warehouse with the aggregates; the Client calls that ALB directly for the dashboard.

It costs about $3,352 per month: $1,460 for the warehouse, $472 for the two lake tables, $420 for the six App Servers, $330 for the two DynamoDB tables, $306 for the two Kafka Streams, $280 for the two Flink cards, $75 for the three ALBs and $11 for DNS. Kafka partitions and Flink slots are not billed in the simulator; the Flink card is billed by instance.

On a normal day each Flink card is about 13% busy, an event's p95 (until it is in the lake) is about 152 ms, and there are no errors and no residency violations. The warehouse is about 54% busy.

"The Region with the consent table's primary is lost" loses eu-west 30 seconds into a run of 3 minutes, for good: its ALB, App Servers, stream, Flink card, lake and the consent table's primary. For a few seconds the DNS card still sends half the apps there, until two health checks in a row find it down; the Client's retries land most of those events in eu-central. The consent replica in eu-central is promoted, still in the EU, and eu-central carries all 995 events a second. The run ends with about 0.12% errors, a p95 of about 158 ms and no residency violations.

"An EU-only audit day: the consent table's primary fails" sends every event from an EU user at the same total rate, so every one of them must stay in the EU. The DynamoDB primary in eu-west is stopped 30 seconds in, for good. The replica in eu-central is promoted, and from then on eu-west's Flink card reads every consent record across the Regions: those events take about 140 ms longer, and the p95 rises to about 265 ms. No errors, no residency violations.

There is no cost challenge for this problem: a search for a cheaper design that passes every test found one at 86% of the reference's cost, above the 80% a challenge needs.

## Why the first try breaks

A common first try keeps the data where it belongs on a normal day. Its DNS card uses Failover (first healthy) routing: every app is sent to a pipeline in eu-west, the same as the reference's, and a second, identical pipeline waits in us-east, where the company already runs its US platform, with a replica of the consent table there for disaster recovery. It costs the same $3,352 per month and passes "A normal day" with the reference's numbers: the US pipeline is idle and no EU event leaves the EU.

When eu-west is lost, the DNS card switches to us-east, the consent replica there is promoted, and every EU event from then on is published to a US stream, looked up in a US table and written to a US lake. The errors stay under the limit (about 5.2%, while the clients' cached DNS answers expire), the latency is fine, and the test fails anyway: about 167,000 residency violations, since each EU event counts once at every card that holds it. In the audit test the US replica is promoted when the primary stops, and eu-west's Flink card looks up every EU user's consent in us-east: about 150,000 violations, one for each event from 30 s on, and a p95 of about 443 ms from the lookups crossing the ocean. The standby moved the data to the place it must never go.

Other first tries break in instructive ways:

- **One pipeline per continent.** The same two pipelines behind a Round robin DNS card fail "A normal day" at once: half the EU events land in us-east.
- **The consent replica in the US.** The reference with its eu-central DynamoDB moved to us-east also fails "A normal day": eu-central's Flink card has no copy in its own Region, so it reads from both copies in turn, and half its lookups go to us-east.
- **Half the Flink slots.** 128 Partitions and a Concurrency per instance of 64 still pass every test: one Region at the peak with local lookups needs about 65 slots. The reference keeps double that so that one Region could carry the peak even with every lookup crossing Regions; it costs nothing here, since partitions and slots are not billed.

The point is this: residency covers every card that holds the data, and every place it can go. A stream and a lookup table hold events as surely as a lake does, a standby Region becomes the primary on a bad day, and a replica serves reads. Keep all of them in Regions with the same tag, and send out of the EU only what was cleared to leave, such as aggregates.

## What the simulator leaves out

Users have no location: a DNS card spreads apps over the Regions in turn, with no routing by geography, so a design can't send EU events to the EU and US events to the US, as a real platform with geolocation routing or separate endpoints would. That is why the reference keeps US events in the EU too. Residency is checked only when a stream, data store or cache stores or serves a request; copying data by replication is not counted, so the first try's US replica breaks no rule until it is promoted, while in reality it holds a copy of every EU consent record from the start. The processing itself, logs, backups and metrics are not checked. Real residency rules come from contracts and laws such as the GDPR, which often cover processing and access too; this problem's rule is the one in its statement, not legal advice.

The aggregates and the hourly job that builds them are not simulated: the dashboard reads a warehouse with no link to the events. Events already in eu-west's stream and lake when it is lost are out of reach until it returns; the test doesn't read them. A Region is lost all at once and stays lost; real outages are often partial. The 70 ms between Regions is one figure for every pair, while eu-west and eu-central are much closer in reality than eu-west and us-east. Prices are the registry's teaching figures, not quotes.

## Things to try

- Change the reference's DNS card to Failover (first healthy) routing and run "The Region with the consent table's primary is lost". It still passes, since the standby is in the EU, but the errors rise to about 5.2% while cached answers expire, against 0.12% when both Regions already take traffic.
- Move the reference's eu-central DynamoDB into us-east and run "A normal day". Watch the lookups alternate between the copies.
- Give the reference's Kafka Streams 64 Partitions and its Flink cards a Concurrency per instance of 32, and run both hidden tests. One Region with the whole peak, or with its lookups crossing Regions, needs more slots than that, and the backlog grows for the rest of the run.
