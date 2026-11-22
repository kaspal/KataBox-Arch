## 1

Almost every request asks the same small question about the same few thousand flags, and the answer rarely changes. Does each one really need to reach the database?

## 2

Put a cache in front of the database (cache-aside) and let entries expire after a set time. That time is the trade-off: the longer an entry lives, the fewer reads reach the database, but the longer a flip takes to show. A cache that still holds the flags also keeps answering while the database is away.

## 3

Add a Redis Cache and set its TTL to a few seconds, around 3, instead of the default 300: a flip then shows within seconds, and the database sees only a trickle of misses. Keep the Go App Servers and a PostgreSQL primary with a replica joined by a Replicate edge. Without the cache, one PostgreSQL member handles roughly 20,000 queries a second, far short of 60,000.
