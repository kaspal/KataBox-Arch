## 1

A customer's endpoint stops answering for a while. What happens to the events you were about to send it, and how long are you willing to keep trying?

## 2

Retry failed deliveries with growing waits, for long enough to outlast an outage of a minute or more, and wire a dead-letter queue so anything that still fails is kept for a replay instead of being dropped. Then count sender slots: each call holds one for as long as the endpoint takes, and a dead endpoint holds one for a full second just to fail.

## 3

Raise the queue's max retries to about 8 (the waits run 1, 2, 4, 8, 16, 32, 32… seconds, more than two minutes in all), wire a Dead-letter Queue from it, and give the Celery Senders a high concurrency, around 64, since sending is mostly waiting on the network. Then add instances until there are clearly more slots than events a second times the seconds each call can take when endpoints slow down.
