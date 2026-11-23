## 1

The good orders didn't get any heavier, and only one message in ten was bad. So what were the workers busy with by the sixth minute?

## 2

A malformed message fails every time, and the queue gave it 20 retries: each one cost the workers 21 times the work of a good order, for nothing. Retries are for temporary failures; a message that keeps failing has to be set aside, not retried for minutes and then dropped.

## 3

On the Queue, set the retries to 3, add a Dead-letter Queue and wire the Queue to it with Publish. Nothing else needs to change.
