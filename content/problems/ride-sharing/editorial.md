# Editorial

## Where the load goes

At 15,000 requests/s, about 14,700 are location pings and about 300 are ride requests. This is a write problem. A cache does not help the pings, because each one replaces the last and nobody reads it twice. What helps is spreading the writes over many independent machines. A NoSQL Store does that by key: each driver's key always lands on the same partition, so the load divides across partitions as long as no single driver dominates. Cassandra here works 32 requests at a time per partition at about 3 ms each, so each partition takes about 10,000 writes a second.

## The reference design

A Network Load Balancer spreads traffic over 8 Go App Servers. Pings are written to Cassandra with 16 partitions, about 160,000 writes a second of capacity. Ride requests read a Redis Cache of 2 GB that holds the nearby-drivers index.

This costs about $4,621 per month, most of it the Cassandra nodes: 16 partitions at $250 a node is $4,000. The p95 is about 23 ms on a normal day. The city-wide surge, which triples traffic for 25 seconds, still leaves each partition far below its limit. At the stadium, the driver keys become very skewed (Zipf 1.3), so one partition gets much more than its share. With 16 partitions the hot one stays under its limit, and the errors stay under 0.1%.

## Why the first try breaks

A common first try uses the same App Servers but writes every ping to one PostgreSQL primary. PostgreSQL has 100 connections at about 5 ms per small write, so it tops out near 20,000 writes a second. A normal day fits, with a p95 near 31 ms. The surge sends about 44,000 writes a second, the connections run out, App Server slots pile up waiting for them, and about 44% of requests fail.

## Things to try

- Drop Cassandra to 4 partitions and run the visible test. Then look for the hot partition.
- Halve the App Servers and see whether the surge still passes.
- Switch the store to DynamoDB, which bills each write instead of each partition, and compare the monthly cost. Which one moves the bill, the partitions or the requests?
