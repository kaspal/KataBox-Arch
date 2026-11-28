# Editorial

## Where the load goes

At 8,000 rps, 7,600 requests a second are searches and 400 are index writes. The searches spread over 1,000,000 different queries, but not evenly: with Zipf 1.0 the single most popular query is about 7% of them, over 500 a second. A search index sends each query to the shard that holds it, so that query always lands on the same shard, however many shards there are.

The hidden tests push on two different weak spots. When everyone searches the same thing, the traffic triples to 24,000 rps, 22,800 searches a second, and the skew steepens to Zipf 1.5: the top query alone becomes well over a third of all searches, thousands a second on one shard. When a search node dies, the index's primary is gone for good from 20 seconds in. Whatever was only on that card is no longer there to answer.

## The reference design

An AWS ALB sends requests to 8 Go App Servers. Searches go cache-aside through a Redis Cache with 8 GB of memory and a TTL of 300 s, in front of an Elasticsearch card, "search-a", with 6 shards. "search-a" is wired with Replicate to a second Elasticsearch card, "search-b", also with 6 shards. Searches spread over both cards; index writes go to the primary. This costs about $2,183 per month: $750 for each Elasticsearch card, $560 for the App Servers, $98 for Redis and $25 for the ALB. In "A normal day" the p95 is about 38 ms, with no errors.

The cache is not optional at this load. Tuning showed why: without a cache, even 10 shards and 8 App Servers failed a normal day. The most popular query lives on one shard, that shard saturates, and the App Servers fill up waiting on it. With the cache, the popular queries are answered from Redis and never reach Elasticsearch.

"A search node dies" kills the search cluster's primary at 20 seconds, for good. This test allows 3% errors and 97% availability. The reference ends at about 2.5% errors. The errors are the index writes refused until the replica is promoted, and the searches that were in flight on the primary when it died. The Web client's 2 Retries win some of them back; without them the reference sits at about 2.7%, too close to the limit. The searches keep working on the replica.

"Everyone searches the same thing" triples the traffic from 15 seconds in, for 30 seconds, with a popularity of Zipf 1.5, so one query takes a large share. The cache answers the trending query: the p95 is about 26 ms, with about 0.04% errors. The price is freshness: a cached result can be a few minutes old, which is fine for search.

There is no cost challenge here. The best cheaper design found cost 91% of the reference, not cheap enough to make one. The second Elasticsearch card is a large part of the bill, and it is the price of the lesson.

## Why the first try breaks

A common first try is the same design without the replica: one Elasticsearch card with 6 shards. It costs about $1,433 per month and is Accepted on "A normal day", with a p95 of about 39 ms. It passes "Everyone searches the same thing" too, because the cache takes the trending query.

"A search node dies" breaks it. From 20 seconds on there is no index left: every index write fails, and so does every search that needs the index. The run ends at about 13% errors, far over the 3% the test allows.

The point is this: two separate habits. Replicate the index, because losing a node must not be an outage. And cache the head of the query distribution, because sharding spreads many queries but can't split one popular query.

## What the simulator leaves out

Ranking, crawling and building the index are not simulated. Elasticsearch replica shards inside one card are not simulated either: the replicas prop does not change the result, so here a second card is the replica. A real search engine sends a query to every shard and merges the answers (scatter-gather); here a query goes to one shard. Rules for how fresh results must be are left out.

## Things to try

- Remove the Redis Cache from the reference and run "A normal day". Then raise Elasticsearch to 10 shards and run it again: does the popular query's shard still fill up?
- Remove "search-b" and run "A search node dies". Compare the errors before and after 20 seconds in Results.
- Lower the cache's TTL to 30 s and run "Everyone searches the same thing". Watch how much more of the trending query reaches Elasticsearch.
