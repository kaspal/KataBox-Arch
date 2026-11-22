# Editorial

## Where the load goes

At 2,000 requests/s, about 1,700 are reads and about 300 are creates. The weight is in the creates: each one carries up to 1 MB. In this simulator a SQL Database slows down with payload size, by a factor of 1 + KB / 64. A 1 MB write therefore takes about 17 times a small one, around 85 ms, and holds one of the database's connections the whole time. Object Storage behaves differently: about 50 ms whatever the size, and far more requests in parallel.

Reads follow Zipf 1.0 over 5 million pastes, so a cache in front of the store catches a good share of them.

## The reference design

A Load Balancer spreads traffic over 4 Go App Servers. Reads check a Redis Cache of 4 GB first and fall through to Object Storage (S3) on a miss. Creates write the body straight to Object Storage. There is no SQL Database in the design.

It costs about $2,579 per month, and most of that is Object Storage requests, priced at $1 per million in this model. It passes the burst with a p95 near 127 ms, and it survives the cold cache because Object Storage can serve every read on its own while the cache warms up.

## Why the first try breaks

A natural first try is a Load Balancer, 2 Go App Servers and one PostgreSQL database holding everything. On a normal day it passes, with a p95 near 125 ms.

"A burst of large pastes" sends 5 times the usual creates, about 1,500 large writes per second. Each one holds a database connection for roughly 85 ms, so the 100 connections fill up. App Server threads wait on the database while holding their own slot, the App Servers run out of threads too, and reads get stuck behind writes. About 42% of requests fail. "Cold cache after a restart" starts every cache empty, which a design leaning on the cache to protect a slow store will also feel.

## Things to try

- Put PostgreSQL back as the paste store, keep the Cache, and run the burst. See how far the cache alone gets you.- Shrink or remove the Cache and compare cost: fewer hits means more Object Storage requests.
- Run "Cold cache after a restart" and watch the hit rate climb over time.
