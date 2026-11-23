# Editorial

## Where the load goes

Two very different things happen for each message. The model answer takes seconds: about 0.6 s before the first token, then about 300 tokens at 80 tokens a second, so roughly 4 to 5 s at the median and about 8 s at p95. The trace write is small, but a warehouse is slow at small writes: each one holds one of its 8 slots for about 300 ms.

At the peak of 5 messages a second, the traces need 5 × 0.3 = 1.5 of the warehouse's 8 slots, about 19%. The launch multiplies that by 4: 20 traces a second need 6 slots, 75%. The model account is not the limit: 20 messages a second of about 1,100 tokens is 1,320,000 tokens a minute, under the 2,000,000 allowed.

The slowdown is where the shape matters. With 8 s added, each write takes about 8.3 s, so 8 slots finish about 8 / 8.3 ≈ 1 trace a second while 5 arrive. The warehouse falls behind by about 4 traces every second for 40 seconds. The question is who waits for that backlog.

## The reference design

A Web client with a Timeout of 30,000 ms sends messages to an AWS ALB, which spreads them over 4 Go App Servers. They call a Claude API (Requests per minute limit 4,000, Tokens per minute limit 2,000,000), answer the chat, and publish its trace to an Apache Kafka Stream with 12 partitions. An Apache Flink Stream Processor with 2 instances and a Concurrency per instance of 8 consumes the Stream and writes each trace to a Snowflake Data Warehouse with its default settings (1 instance, 8 slots). This costs about $95,645 per month, and the tokens are almost all of it: $93,660 for the Claude API, $1,387 for Snowflake (billed while it runs, which on a busy day is almost always), $280 for the App Servers, $153 for Kafka, $140 for Flink and $25 for the ALB. A normal day has a p95 of about 7.8 s and no errors.

"Launch day" multiplies the traffic by 4 from 30 seconds in, for 2 minutes, in a run of 3 minutes. A real launch lasts hours; the test is long enough for any backlog to show. The warehouse is about 75% busy with traces, 12 partitions let 12 writes run at once, and the App Servers have room. There are no errors and the p95 is about 8.3 s.

"The warehouse slows down" adds 8 s to every warehouse write from 20 seconds in, for 40 seconds, in a run of 90 seconds. The App Servers don't notice: they publish the trace to Kafka in a few milliseconds and answer the chat. Flink's writes now take about 8.3 s each, and with at most 12 in flight the traces pile up in the Stream, roughly 140 of them by the end of the slowdown (a rough count from the numbers above). None is lost: they stay in Kafka until Flink writes them, and the backlog clears within seconds once the warehouse is fast again. The dashboards are behind for about a minute, which they can afford. The chat has a p95 of about 8.1 s and no errors.

There is no cost challenge for this problem. The tokens are about 98% of the bill and the traffic sets them; the cheapest design found that passes every test costs the same as the reference.

## Why the first try breaks

A common first try is the same Web client, ALB, App Servers and Claude API, with the App Servers writing each trace to the same Snowflake warehouse themselves, right after the model answers. It costs about $95,352, a little less without Kafka and Flink, and it passes "A normal day" with a p95 of about 8.3 s: the 300 ms write is added to every answer, but there is room for it. It even passes "Launch day", with a p95 of about 8.8 s, because the warehouse still keeps up.

The slowdown breaks it: a p95 of about 27.2 s and about 14.9% errors. Every message now waits for its own trace write, and the writes queue behind the warehouse's 8 slow slots. Within 10 seconds the answers take 15 to 25 s, although the model is as fast as before. From about 20 seconds into the slowdown, messages start reaching the client's 30 s Timeout and fail, until the warehouse is fast again and its queue clears.

A bigger warehouse doesn't fix it. With 4 Snowflake instances (32 slots) the first try costs about $99,513 and still fails the slowdown with a p95 of about 17.6 s: 32 slots at 8.3 s finish about 3.9 traces a second, still fewer than arrive, and each answer still waits at least 8.3 s for its own write.

The point is this: work the user doesn't need to see shouldn't sit in the user's path. A trace is for the analysts, so write it down safely (a publish to a Stream) and let a consumer move it to the warehouse at the warehouse's pace. Then a slow warehouse costs you fresh dashboards for a minute, not failed chats.

## What the simulator leaves out

The quality of the answers is not simulated, and neither is what is in a trace: its size, its fields or the cost figure it carries. The dashboards' own queries are not part of the traffic, so they never compete with the trace writes for warehouse slots. A real Flink job (or Snowpipe Streaming) sends traces to the warehouse in batches, which makes each trace much cheaper to load; here every trace is one 300 ms write in both designs, so the warehouse costs the same. Exactly-once delivery is not checked, and Kafka's retention (7 days) is never reached in a test. Streaming of the answer is modelled only as timing, and the p95 is measured when the App Server answers. The token prices are the registry's teaching figures.

## Things to try

- Set the Stream's Partitions to 2 and run "Launch day". Watch the Stream's backlog: 2 writes in flight at 300 ms each move fewer traces than arrive, while the chat stays fast.
- Give the first try's Snowflake 4 instances and run the slowdown. Compare the p95 with the reference's.
- In the reference, kill the Snowflake card for 30 s and watch the Stream hold the traces until it is back.
