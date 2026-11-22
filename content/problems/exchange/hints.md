## 1

At the opening bell, orders arrive several times faster than during the session, and a matcher can crash at any moment. What limits how many orders your exchange matches at once, and what keeps matching when a matcher dies?

## 2

A stream hands out one message per partition at a time, so the partition count caps how many orders are being matched at once; size it, and the Order gateway, for four times the normal flow. Then split the matchers across more than one card. If one card holds every matcher, a single crash stops all matching.

## 3

Use 6 Go gateway instances behind a Network Load Balancer, a Kafka stream with 128 partitions, and two Celery matcher cards of 12 instances each: either card alone can carry a normal session three times over. The matchers write to a Redis order book with a long TTL.
