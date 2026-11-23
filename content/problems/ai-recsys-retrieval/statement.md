# Recommendations: Retrieve, Then Re-rank

## The product

You run the "you may also like" strip on the item pages of a large online shop. The catalogue holds 5 million items, and each one has an embedding vector in a vector index. When a shopper opens an item page, your service finds the 50 items whose vectors are nearest to that item's vector (retrieval), then sends those 50 candidates to a small classifier model that scores them for this shop, and returns them in the new order (re-ranking). The finished list for an item is about 2 KB, and it is the same for every shopper who opens that item.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 600 requests/s |
| p95 latency | 150 ms or less |
| Availability | 99.9% or more |
| Errors | 1% or less |
| Budget | $3,000 per month |

## The traffic

- **Show "you may also like" for an item** (100%). The request may pass a load balancer and reaches your App Servers. The App Server may first look the item up in a Cache (cache-aside: a hit returns the stored list and ends the request; a miss goes on, and the finished list fills the cache). On a miss it calls a Vector Database, then a Model Inference card with the model type "Classifier". The classifier's input (the item and its 50 candidates) is about 50 tokens on average, 100 at p95, and it answers with one token.

The request's key is the item being viewed. Item popularity is a long tail: a few items are viewed a lot, but most of the 5 million are viewed rarely, so most requests are for items nobody else asked for in the last few minutes.

Set "Vectors (millions)" on your Vector Database cards to 5. Every search asks for the 50 nearest neighbours. In the simulator, a search takes longer the more vectors each shard holds and the more neighbours it asks for; each shard runs 16 searches at a time, and a given item's searches always go to the same shard. With 5 million vectors in one shard, a search takes about 15 ms.

Caches start warm.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A sale triples traffic** and **One item goes viral**. Your design has to pass all three.
