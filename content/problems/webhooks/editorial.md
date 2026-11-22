# Editorial

## Where the load goes

At 2,000 events/s, about 1,400 go to customers' endpoints in general and about 600 go to Acme. The work is waiting: a Sender holds a slot for each call. By the medians alone, 1,400 × 0.2 s + 600 × 0.1 s keeps about 340 slots busy, and the slow tail of the general endpoints adds many more.

The hidden tests change the sums. When Acme is down, its 600 events a second each hold a slot for the 1 s connect timeout just to fail, and every retry holds one again. When the general endpoints slow down by 1 s, their 1,400 calls a second take about 1.2 s each: about 1,700 slots for them alone, before Acme and the tail.

## The reference design

The Events API runs on 2 Go instances and publishes to an SQS Queue with max_retries set to 8. A Dead-letter Queue is wired from that queue. 28 Celery Senders, each with a concurrency of 64, consume it: 1,792 slots in all. Many slots per instance is right here, because sending is waiting on the network, not using the CPU. This costs about $4,243 per month, with a p95 near 1 s on a normal day.

"Acme's endpoint is down for a minute" takes Acme down at 20 seconds for 60 seconds, in a run of 150 seconds. It allows a p95 of 120 seconds and no lost messages. Failed calls that are retried aren't errors (the event is still on its way); Results counts them separately as retried attempts. The retries wait 1, 2, 4, 8, 16, 32, 32 and 32 seconds, about two minutes in all, which outlasts the outage, so every event reaches Acme once it is back, with a p95 of about 41 seconds. Anything that still failed would land in the Dead-letter Queue. "Customers' endpoints slow down" adds 1 s to every call to the general endpoints from 15 seconds, for 30 seconds, with a p95 limit of 10 seconds. The 1,792 slots carry it with a p95 of about 5.9 seconds.

## Why the first try breaks

A common first try is the same queue with its default of 3 retries and no Dead-letter Queue, and 22 Senders of 64: 1,408 slots, for about $3,823 per month. It passes "Normal day" with the same p95 near 1 s.

During Acme's outage, each event fails after 1 s and is retried after 1, 2 and 4 seconds. Those waits add up to only 7 seconds, so the retries run out while Acme is still down, and about 5,198 messages are lost. When the endpoints slow down, the 1,408 slots are fewer than the slow calls need, a backlog builds in the queue, and the p95 reaches about 10.7 seconds, over the limit.

## What the simulator leaves out

The usual next step is per-customer isolation: a separate queue and sender pool for each big customer, so one customer's outage cannot take everyone's slots. The simulator does not route calls to a particular provider through a particular pool, because external providers are not cards, so it cannot show isolation; this problem measures retries, dead letters and sender capacity instead. Signatures, idempotency keys and ordering per customer are not simulated either.

## Things to try

- Keep 8 retries but remove the Dead-letter Queue. Then try 5 retries (31 seconds of waits) with it, and see where the failed events go.
- Cut the Senders back to 22 instances and run "Customers' endpoints slow down" again. Then try a concurrency of 8, the Celery default, and count the instances you would need.
