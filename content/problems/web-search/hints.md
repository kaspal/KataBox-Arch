## 1

Two things can take a search engine down: a machine that stops, and a crowd that asks the same question at once. If your search index lives on one card, what answers the searches when that card is gone? And which part of the index answers the most popular query?

## 2

A search index sends each query to the shard that holds it, so one popular query always lands on the same shard. More shards spread many queries, but they never split one. Put a cache in front of the index for the head of the distribution: popular results a few minutes old are fine here. Then keep a second copy of the index, so that losing one node means fewer machines, not no results.

## 3

An AWS ALB in front of 8 Go App Servers. Searches go cache-aside through a Redis Cache with 8 GB of memory and a TTL of 300 s. Behind it, an Elasticsearch card with 6 shards, wired with Replicate to a second Elasticsearch card with 6 shards: searches spread over both, and index writes go to the primary.
