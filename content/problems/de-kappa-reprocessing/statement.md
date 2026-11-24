# Kappa Reprocessing

## The product

You run the event pipeline of a delivery app, built the Kappa way: there is no batch layer. Every order event goes once into an Apache Kafka topic that keeps 30 days of history, and a Stream Processor (an Apache Flink job) turns each event into a row of the serving table in PostgreSQL, which the operations dashboards read. An event should reach the table within seconds.

A bug in the job has computed delivery times wrongly for a month. In a Kappa architecture the fix is not a separate batch job: you deploy the fixed code and replay the retained events through it, from 30 days ago up to now, while the live events keep flowing. The replay rewrites the month's rows; nobody waits for any single replayed event, but the live rows must stay fresh the whole time. The obvious way to run the replay is to rewind the live job to 30 days ago. Decide whether that is what you want.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 600 events/s (300 live, 300 replayed) |
| p95 latency (see below) | 5 s or less |
| Availability | 99.9% or more |
| Errors | 0.1% or less |
| Lost events | none |
| Budget | $2,000 per month |

## The traffic

- **Process a live event** (50%). The request may pass a load balancer and reaches your App Servers (the ingest API), which publish it to a Stream; the Stream's consumer processes it and writes the row to the SQL database. A live event is measured by **completion**: from the moment the API receives it until its row is written. That is how fresh the dashboards are.
- **Reprocess a retained event (replay)** (50%). It starts at a Scheduler card, which stands for the replay reading the retained events back from Kafka at its pace: it publishes each one to a Stream, whose consumer processes it with the fixed code and writes the row to the SQL database. A replayed event is measured by **response**: until its Stream has taken it. After that it can wait.

So the p95 target applies to the completion of the live events and to the acknowledgement of the replayed ones.

The job is CPU-bound: one Flink slot processes about 200 events a second (set as "Records per second per slot" = 200), and each event then waits for its write to PostgreSQL. A Flink instance has 4 vCPUs and runs 4 slots ("Concurrency per instance" = 4); more slots per instance would not process more, so keep both values as they are and add instances or Stream Processor cards instead. The Stream hands out one event per partition at a time.

The replayed events are the same events as the live ones, read again: nothing marks them, so "Live before bulk (priority)" on a Stream cannot tell them apart.

Plan for losing one Stream Processor card (a whole Flink cluster) at any moment.

## The tests

One test is visible. You can run it as often as you like:

- **A normal hour, with the replay at its pace**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The evening peak, with the replay sped up to 6 times its pace** and **A Stream Processor is lost during the replay**. The whole replay takes days; a test sees one or two minutes of it. Your design has to pass all three.
