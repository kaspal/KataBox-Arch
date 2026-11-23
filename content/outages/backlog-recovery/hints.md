## 1

S3 was slow for 20 seconds, but order confirmations were late for more than a minute after it recovered. What was every new confirmation waiting behind?

## 2

The queue is first in, first out, and most of the pile is thumbnail backfill that nobody is waiting for. The pool is 90% busy, so it has only a tenth of its power to work the pile off. Let the urgent messages skip the background ones, and give the workers room to drain.

## 3

On the queue, turn priority on (the thumbnail backfill is bulk work, so confirmations go first), and run 3 workers instead of 2 so the pile drains even at a busier hour.
