# Editorial

## Where the load goes

30 batches a second is little for an App Server. The work is at the warehouse: a load takes about 0.3 s, and a Snowflake instance runs 8 at once, so one instance loads about 8 ÷ 0.3 ≈ 27 batches a second. Two instances (about 53 a second) carry a busy hour at about 60% busy.

The outage changes the question. For 60 s nothing can be loaded. Whatever arrives in that minute either fails or waits somewhere: 30 × 60 = 1,800 batches on a busy hour, 45 × 60 = 2,700 at the peak. Writing straight from the App Server, they fail. Published to a Stream, they wait: a load that fails is retried, and the record stays in the Stream until it goes in.

Waiting is only half of it. When Snowflake is back, the backlog is drained by the capacity left over after the live traffic, not by the full capacity, because new batches keep arriving:

drain time = backlog ÷ (catch-up rate − live rate)

At the peak, with 2 instances: 2,700 ÷ (53 − 45) ≈ 340 s, almost six minutes of stale dashboards after a one-minute outage. With 3 instances (80 a second): 2,700 ÷ 35 ≈ 77 s. With 4 (107 a second): 2,700 ÷ 62 ≈ 44 s, inside the 50 s the catch-up limit allows (in real life an hour's outage and a catch-up of about an hour; scaled down here). The catch-up rate has to beat the live rate by enough, not just beat it.

The retries add a delay of their own. During the outage every record's load fails again and again, and its wait doubles each time up to 32 s. When Snowflake returns, the oldest records come back over the next 32 s, so even with unlimited capacity the backlog's last batches go in about 32 s after the return.

## The reference design

An AWS ALB sends the batches to 2 Go App Servers, which publish each one to an Apache Kafka Stream with 48 partitions and answer the collector at once. Kafka Connect, 3 instances with a concurrency of 16 (48 slots), consumes the Stream and loads each batch into Snowflake, with 4 instances (32 loads at once). The partitions and the connector's slots stay above the warehouse's 32, because a record also spends time in the connector: with only 32 partitions, the partitions become the cap, and old batches keep landing until 131 s, past the limit.

This costs about $6,368 per month: $5,840 for Snowflake ($1,460 per instance), $210 for Kafka Connect, $153 for Kafka, $140 for the App Servers and $25 for the ALB. The fourth Snowflake instance is the price of the catch-up.

On a normal day the p95 freshness is about 0.74 s, Snowflake is about 30% busy and Kafka Connect about 23%.

"The warehouse is down for 60 s" kills Snowflake from 20 s to 80 s at 30 batches a second, in a run of 600 s. No batch is refused or lost. About 1,800 batches wait in the Stream, the last ones go in at about 112 s (the 32 s retry wait), and the p95 over the whole test is about 34.5 s.

"The warehouse is down for 60 s at the evening peak: 1.5 times the events" does the same at 45 a second. About 2,700 batches wait; the last second in which batches landing had waited over a minute is 123 s, and from 126 s they land in under 10 s again. The catch-up limit starts at 130 s, so it passes, and the p95 over the whole test is about 42.9 s.

For the cost challenge, the search found no design under 80% of the reference's cost that passes every test (its best was 99%), so this problem has no challenge. The catch-up needs about 99 loads a second at the peak, and only 4 Snowflake instances give that.

## Why the first try breaks

A common first try has each App Server load its batch straight into Snowflake, with 2 instances. It costs about $3,085 and passes "A normal day" with a p95 of about 0.67 s, Snowflake about 59% busy.

When Snowflake goes down, every load fails at once, and nothing holds the batch: the collector gets an error. For 60 s of the 600, every batch fails, about 10.0% of them, against a limit of 0.1%. The peak test fails the same way, with 10.1%. The warehouse came back fine; the batches sent while it was away were refused.

The next try adds a Stream (24 partitions) and Kafka Connect (2 instances of 12 slots) but keeps 2 Snowflake instances, sized for the busy hour. It costs about $3,378. On a busy hour it passes the outage: 1,800 batches drain at 53 − 30 = 23 a second, in about 78 s, and the p95 is about 52 s. At the peak it fails: the spare capacity is 53 − 45 = 8 a second, the 2,700 batches need about 340 s, and batches landing still waited over a minute until 167 s and over 50 s until about 226 s. Nothing is lost, but the dashboards stay minutes behind. With 3 instances ($4,908) the drain takes about 77 s, and batches that waited over a minute keep landing until 138 s, past the 130 s limit.

The point is this: a buffer turns an outage into a delay, and the spare capacity decides how long the delay lasts. Size the catch-up for the peak, with enough above the live rate that the backlog of a whole outage drains in the time you can afford.

## What the simulator leaves out

Each batch is one load of about 0.3 s. Real pipelines tune the batch size and the number of files per load (Snowpipe, COPY, a sink connector's buffer), and larger batches during a catch-up load faster per event; that is not modelled. Snowflake instances are fixed here: a real multi-cluster warehouse can add clusters while the backlog lasts and drop them afterwards, which makes catch-up capacity cheaper than paying for it all month. Raising Concurrency per instance doesn't slow loads down here, while real loads on one cluster share its compute; that is why the statement fixes it at 8. The retry wait (1 s doubling to 32 s) is the simulator's; Kafka Connect's own retry and error-tolerance settings, its offsets, rebalances and dead-letter topics are not simulated. Duplicates from a load retried after it had in fact succeeded, schema changes and the correctness of the data itself are not simulated. Time is scaled down: the outage lasts 60 s and the catch-up window 50 s. Kafka Connect's price is a teaching price, not a quote.

## Things to try

- Set Snowflake to 3 instances, run the peak outage, and compare when the old batches stop landing with the 77 s the drain formula gives.
- Set Snowflake to 6 instances. The old batches still land until about 114 s: most of that is the retry wait, not the warehouse.
- Set the Stream to 32 partitions and keep 4 Snowflake instances. Which card is the cap now?
