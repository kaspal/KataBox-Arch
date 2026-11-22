## 1

The provider decides how fast you may send, and it can change its mind. What happens to an email it refuses, and how long can that email wait for another try?

## 2

A queue that retries with growing waits spreads the load out over time: when the provider accepts fewer calls, the refused emails come back a little later and get through then. A lower limit should mean a slower campaign, not lost mail, as long as the retries last longer than the limit does. Wire a dead-letter queue so anything that still fails is kept instead of dropped.

## 3

Raise the queue's max retries to about 10 (the waits run 1, 2, 4, 8, 16, 32, 32… seconds, more than three minutes in all) and wire a Dead-letter Queue from it. Celery Mail senders with a high concurrency, around 64, suit a job that is mostly waiting on the provider; 4 instances of 64 are plenty for 500 emails a second.
