# Editorial

## Where the load goes

Every request is an order: 10,000 a second. A matcher spends 0.5 ms on each and writes it to the Redis order book. From the moment the stream hands an order out until the matcher is done, about 3 ms pass: the hop, the match and the round trip to Redis.

A stream hands out one message per partition at a time, so one partition matches about 330 orders a second. 48 partitions match about 16,000 a second, and 128 about 42,700. A Go gateway instance holds 64 requests at 5 ms each, about 12,800 a second. At the open, four times the orders arrive, 40,000 a second: that needs at least 4 gateway instances and about 120 partitions.

## The reference design

The Order gateway, 6 Go instances behind a Network Load Balancer, publishes each order to a Kafka stream, the Order log, with 128 partitions. Two Celery matcher cards consume it, Matchers A and Matchers B, each with 12 instances at Celery's default concurrency of 8: 96 slots per card, 192 in all. Both write to a Redis order book with a TTL of 86,400 seconds. This costs about $2,314 per month, mostly the matchers: 24 instances at $70 is $1,680. 10,000 orders a second is about 26 billion messages a month, but Kafka (Amazon MSK) is billed per broker, not per message: the stream is one broker at $153. On a normal session the p95 is about 19 ms, with the gateway and the matchers about 16% busy.

"The market opens" runs four times the traffic for 10 seconds, from 20 seconds in, and allows a p95 of 1 s and a p99 of 3 s. 40,000 orders a second fit under the ceiling of 128 partitions, and 6 gateway instances can take about 76,800 a second, so no backlog forms: the p95 stays at about 19 ms.

"A matcher dies" kills the first Worker card, Matchers A, at 20 seconds for good, in a run of 90 seconds, and allows a p95 of 100 ms and a p99 of 1 s. Matchers B takes every new order at once; its 96 slots match about 32,000 a second. The handful of orders A was holding stay assigned to it until the stream's session timeout (Kafka's default, 45 seconds) runs out, then go to B. They are a few dozen out of about 900,000, far fewer than 1 in 100, so neither the p95 (about 19 ms) nor the p99 (about 26 ms) moves.

## Why the first try breaks

A common first try uses 3 gateway instances, 48 partitions and one matcher card of 8 instances: 64 slots, for about $984 per month. It passes "Normal session" with the same p95 of about 19 ms; the matchers are about 47% busy.

At the open it runs short twice. 3 gateway instances take about 38,400 orders a second, just under the 40,000 that arrive, and 48 partitions match only about 16,000. The backlog grows for the whole 10 seconds: about 16% of the orders fail and the p95 reaches about 6.4 s.

When its only matcher card dies, nothing is matched from 20 seconds on: about 67% of the orders count as errors and the p95 reaches about 66 seconds. No order is lost; the stream keeps them all.

## What the simulator leaves out

Real exchanges match each symbol on a single thread in memory, in strict price and time order, usually without a broker in the path, and their latencies are in microseconds. The simulator's milliseconds show the shape of the design, not the absolute numbers. Fairness and ordering within a symbol are not simulated either. The session timeout is a real trade-off (too long and a dead consumer's orders wait; too short and a consumer that is only slow for a moment is thrown out of the group), but here it delays too few orders to show in the percentiles.

## Things to try

- Merge the two matcher cards into one card of 24 instances and run the matcher test again. Same slots, same cost, different result.
- Merge the two matcher cards into one card of 24 instances and run "A matcher dies". Same slots, same cost, a very different result.
- Keep 128 partitions but drop the gateway to 3 instances. See which part of the open still breaks.
