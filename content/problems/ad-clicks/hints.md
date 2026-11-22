## 1

Every click becomes one write to a counter. How many writes a second is that, and what happens to that number when clicks arrive several times faster for a few seconds?

## 2

An increment is tiny work. A store that keeps counters in memory does it in a fraction of a millisecond, while a database write takes a few milliseconds and the database has a fixed number of slots; an aggregator waiting on a busy database holds its slot the whole time. Count the stream's partitions too: each hands out one click at a time, so together they cap how fast clicks are counted. Leave room for four times the traffic for a few seconds.

## 3

Keep the counters in a Redis Cache with a long TTL, such as a day (86,400 s), and read the totals from it as well. Give the Kafka stream about 128 partitions, and give the Aggregators as many slots: 16 Celery instances at the default concurrency of 8.
