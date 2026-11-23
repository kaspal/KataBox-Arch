## 1

You get 20,000 requests a second and the provider takes 200. Almost every answer has to come from somewhere other than the provider. And when the provider stops answering, or your copy of the data disappears, what does a user get?

## 2

A forecast that expired a minute ago is still far better than an error. Let the cache keep expired copies for a while and serve them when the provider call fails, and let requests for the same missing city wait on one call instead of each making their own. A cache that restarts empty sends every request to the provider at once, so keep a second copy that can take over and refill it.

## 3

Put an AWS ALB in front of 8 Go App Servers with a call timeout of 200 ms, so a request waiting on a dead provider gives up quickly. Read from a Redis Cache (2 GB, TTL 600 s) with "Serve stale on error for" set to 3,600 s, the stampede guard on and a failover time of 5 s. Wire it with Replicate to a second Redis Cache with the same settings.
