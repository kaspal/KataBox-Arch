## 1

A provider is down and a notification fails to send. What happens to it next, and what happens if it fails again, and again?

## 2

Retry failed messages with growing waits between tries, and keep the ones that still fail somewhere safe, a dead-letter queue, so they can be replayed later instead of being lost.

## 3

Raise the queue's max retries until the waits between tries add up to more than a long outage, wire a Dead-letter Queue from the queue, and give the Workers plenty of sender slots, with many instances and high concurrency, so a dead provider cannot hold all of them.
