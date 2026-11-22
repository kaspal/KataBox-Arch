# Editorial

## Where the load goes

At peak, 6,400 requests per second read a top 100 and 1,600 submit a score. The data itself is tiny: 50 boards of 100 entries each. What makes it hard is how often that small data is touched, and how big each read is. Every read returns 32 KB.

A SQL Database pays for that size. A bigger result takes longer, about 1.5 times a small query here, and holds a connection the whole time. PostgreSQL has 100 connections, and every write needs one too.

## The reference design

A Load Balancer spreads traffic over four Go App Servers. They read and write a Redis Cache used as the store, with no TTL, so every read is answered by Redis itself and nothing expires. Redis runs each command in about 0.01 ms on one thread, around 100,000 operations per second, which is far above what this game asks for.

A Replicate edge links the primary to a replica, which makes a cluster with a failover time of 2 s. Four App Servers leave room for traffic to double. The design costs about $341 per month.

## Why the first try breaks

A natural first try keeps the same four App Servers and uses PostgreSQL as the store. On a normal day it passes with a p95 near 35 ms.

The hidden test "The tournament ends" is where it fails. For 20 s, score submissions run at five times their usual rate and all traffic doubles. The heavy top-100 queries and the flood of writes compete for the same 100 connections. They run out, requests queue, and about 33% fail. Redis takes the same burst without strain.

The other hidden test, "The store restarts", takes the store's primary down for 10 s and allows up to 8% errors. Health checks notice the dead primary within 2 to 4 s, and the replica is promoted after the 2 s failover. Requests fail only in that gap: about 2.7% errors in total. Without a replica, the store is gone for the full 10 s.

## Things to try

- Remove the replica and run the restart test again. Compare the error rate with the 8% limit.
- Drop to two App Servers and watch the tournament's end.
- Keep PostgreSQL but put a Cache with a short TTL in front of the top-100 reads. Check what still reaches the database when the tournament ends.
