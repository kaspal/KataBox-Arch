# Editorial

## Where the load goes

At 1,500 notifications/s, about 900 are push, 450 are email and 150 are SMS. The real cost is Worker time, mostly spent waiting on providers. On a normal day about 150 sender slots are busy on average, more in the slow tails.

A dead provider changes the sums. Each call to it fails only after a 1 s connect timeout, so 450 emails a second hold about 450 slots just to fail, and every retry holds one again. If the senders run out of slots, push and SMS queue behind emails that cannot go anywhere.

## The reference design

The Notify API runs on 2 Go instances and publishes to an SQS Queue with max_retries set to 10. A Dead-letter Queue is wired from that queue. 32 Celery Senders, each with a concurrency of 32, consume it: 1,024 slots in all. This costs about $4,001 per month.

"The email provider goes down" stops email for 40 seconds, starting at 15 seconds. It allows half the requests to error and a p95 of 60 seconds, but no lost messages. Retries come after 1, 2, 4, 8, 16 and 32 seconds, which outlasts the outage, so the emails go through once the provider is back. Anything that still failed would land in the Dead-letter Queue. "A marketing blast" sends 5 times the traffic for 10 seconds and allows a p95 of 15 seconds; this design clears it with a p95 of about 1.2 seconds.

## Why the first try breaks

A common first try is the same design, but the queue keeps its default of 3 retries and has no Dead-letter Queue. It passes "Normal day" and the blast. During the outage, each email fails after 1 s and is retried after 1, 2 and 4 seconds. All three retries are used up in about 11 seconds, while the provider is still down. About 4,575 messages are lost, and the test fails.

The lesson: an outage longer than your retry schedule loses messages, unless the retries last long enough or failures land in a dead-letter queue to replay later. Sender capacity matters too.

## Things to try

- Keep 10 retries but remove the Dead-letter Queue. Then try 5 retries with it, and see where the failures go.
- Cut the Senders to 8 instances and watch push and SMS latency during the email outage.
