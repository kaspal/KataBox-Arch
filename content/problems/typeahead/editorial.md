# Editorial

## Where the load goes

Every one of the 6,000 requests/s is a read, and they are very uneven. With Zipf 1.1 over 200,000 prefixes, a few short prefixes such as single letters get a large slice of all keystrokes. The Search index places each key on one shard, and the same key always lands on the same shard. So the hottest prefix sends its whole load to one shard while the others sit mostly idle. More shards spread different keys further apart, but they never split one hot key. The answer for a popular prefix barely changes within an hour, which is exactly what a cache is for.

## The reference design

A Load Balancer spreads traffic over 3 Go App Servers. Each App Server checks a Redis Cache named "prefixes" (2 GB, TTL one hour) first. A hit ends the request. A miss asks Elasticsearch, with 4 shards, and fills the cache.

This costs about $759 per month and has a p95 near 18 ms in every test. Almost every prefix is answered by the cache, so the index sees only a thin stream of rare prefixes, spread evenly over its shards.

## Why the first try breaks

A common first try is the same App Servers calling Elasticsearch with 12 shards on every keystroke. It looks generous, but even on a normal day the most popular prefixes pile onto one shard. That shard saturates, App Server threads wait behind it, and about 42% of requests fail.

The hidden tests make it worse. "Everyone types the same thing" triples the traffic for 20 s and makes the hot prefixes much hotter (Zipf 1.5), so the hot shard gets even more of the load. "The index rebuilds" adds 200 ms to every Search request for 40 s, which alone is far over the 60 ms limit when every keystroke reaches the index. With the cache in front, both tests touch only the few misses, and the p95 does not move.

## Things to try

- Remove the Cache from the reference and change the shard count from 4 to 12. Watch the busiest shard, not the average.
- Lower the TTL to one minute and compare the hit rate and p95.
- Shrink the Cache and find the size where misses start to reach the index in large numbers.
