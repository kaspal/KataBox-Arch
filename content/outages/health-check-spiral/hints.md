## 1

The cards were only about two thirds busy during the surge, yet a third of the requests failed. Watch the load balancer: what happens to the other cards when it takes one out of rotation?

## 2

The health check reads a long queue as a dead card. Each ejection moves that card's traffic onto the rest, whose queues then grow past the limit too. A busy card must answer fast instead of queuing, and the load balancer must never remove too many cards at once.

## 3

On every app card set `shed_queue` to 28 (16 slots at 100 ms each: under 200 ms of waiting). On the load balancer set `min_healthy_pct` to 60. Keep the six cards and the 200 ms health timeout.
