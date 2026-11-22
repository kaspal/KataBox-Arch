# Editorial

## Where the load goes

At 500 emails a second, the work is waiting on the provider: a sender holds a slot for each call. By the median alone, 500 × 0.15 s keeps about 75 slots busy, and the slow tail, up to 1.2 s, adds many more. The provider's limit of 800 calls a second leaves room on a normal day.

The hidden tests take that room away. When the limit drops to 250 calls a second, 500 emails still arrive each second, so about 250 a second are refused. Over 40 seconds that is about 10,000 emails that cannot get through yet. When two campaigns overlap, 1,000 emails a second arrive, more than the provider's 800. The question is whether those emails are still around when they can go out.

## The reference design

A Campaign scheduler, an Airflow card firing every second, publishes the emails to an SQS Queue with max_retries set to 10. A Dead-letter Queue is wired from that queue. 4 Celery Mail sender instances, each with a concurrency of 64, consume it: 256 slots in all, about two-thirds busy on a normal day. This costs about $865 per month, most of it the queue, and the normal campaign's p95 is about 786 ms.

"The provider cuts your rate limit" lowers the limit to 250 calls a second from 20 seconds in, for 40 seconds, in a run of 120 seconds. It allows an error rate of 60% and a p95 of 120 seconds, but no lost emails. Each refused email goes back to the queue and is retried after 1, 2, 4, 8, 16, 32, 32, 32, 32 and 32 seconds: 191 seconds in all, far longer than the cut. The backlog drains once the limit is back to 800: the p95 is about 25 s, with nothing lost. Anything that still failed would land in the Dead-letter Queue.

"Two campaigns overlap" doubles the traffic for 30 seconds, from 20 seconds in, with the same limits. The extra emails wait in the queue and go out as the senders and the provider allow, and the p95 is about 9.7 s.

## Why the first try breaks

A common first try is the same design with the queue's default of 3 retries and no Dead-letter Queue. It costs the same and passes "Normal campaign" with the same p95.

During the rate cut, a refused email is retried after 1, 2 and 4 seconds. Those waits add up to only 7 seconds, while the cut lasts 40, so an email that keeps meeting a full limit runs out of retries and is dropped: about 1,891 emails are lost. The overlap test still passes, with the same p95 of about 9.7 s and nothing lost: there the provider still takes 800 calls a second, so the refused emails find room within their three retries.

The point is this: retries spread the load over time, so a lower limit means a slower campaign, not lost mail, as long as the retries last longer than the trouble. A Dead-letter Queue keeps whatever still fails.

## What the simulator leaves out

Real providers send a Retry-After header with a 429 and throttle per receiving domain; here the queue's backoff is the only pacing. Bounces, unsubscribes and sender reputation are not simulated either.

## Things to try

- Set max retries to 5 (31 seconds of waits) and run the rate cut again. Is it long enough for a 40-second cut?
- Keep 10 retries but remove the Dead-letter Queue, then compare the two designs' results.
- Raise the Mail senders to 8 instances and run the overlap again. Watch whether the p95 drops, or whether the provider's limit is now what holds it.
