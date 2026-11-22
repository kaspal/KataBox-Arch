# Editorial

## Where the load goes

At 10,000 requests/s with 97% reads, about 9,700 requests/s open a feed and about 300 publish a post. Each read returns a 16 KB page. The posts look small, but each one fans out: with 10 followers on average, 300 posts become about 3,000 feed writes per second, and a single post from an account with 50 times the followers turns into about 500. If those feed writes and the reads share one database, they fight over the same connections.

## The reference design

A Load Balancer spreads traffic over 6 Go App Servers. A read checks a Redis feed Cache of 16 GB first. A hit ends the request; a miss reads PostgreSQL, which runs as a primary with one replica, and fills the cache. A post is written to the primary and published to a Kafka Stream with 32 partitions, and the App Server is free again. 4 Fan-out Workers (Celery) consume the Stream and write the post into each follower's feed in the Cache.

This costs about $1,885 per month and has a p95 near 23 ms on a normal day. It passes both hidden tests. The feeds are already built when the readers arrive, and a burst of posts only keeps the Workers busier for a while, without slowing the read path.

## Why the first try breaks

A common first try keeps the same App Servers, publishes posts to a RabbitMQ Queue, and has Workers write feed rows into the one PostgreSQL database. Reads come straight from that database. But every 16 KB read and every feed write land on the same database, and p95 is about 155 ms on a normal day.

The hidden tests push further. "Everyone opens the app at once" triples the traffic for 15 seconds. The database has no slack, threads pile up, and about 33% of requests fail. "Breaking news" sends 4 times the posts. Feed writes take a larger share of the database, reads queue behind them, and about 5% of requests fail.

## Things to try

- Take the Cache out of the read path and run "Normal day" again. Watch p95 and the database load.
- Shrink the feed Cache to 2 GB and compare the hit rate.
- Remove Fan-out Workers one at a time and find how few can keep up.
