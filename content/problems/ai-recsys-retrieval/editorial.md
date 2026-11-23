# Editorial

## Where the load goes

A request that misses the cache makes two calls in a row: the vector search that retrieves 50 candidates, and the classifier that re-ranks them. The classifier is the most expensive card, but it is fast: on NVIDIA Triton a call takes about 5 ms to start, 10 ms to read 50 tokens and a little more for its one-token answer, and one GPU has 64 batch slots. The step that runs out first is the vector search.

In the simulator a search costs the card's service time of 2 ms, times 1 + log2 of the vectors per shard in millions, times √(neighbours ÷ 10), times 1 + 0.15 ln(shards). With 5 million vectors in one shard and 50 neighbours, that is 2 × (1 + log2 5) × √5 ≈ 2 × 3.3 × 2.2 ≈ 15 ms. A shard runs 16 searches at a time, so it finishes at most about 16 ÷ 0.015 ≈ 1,070 searches a second, and fewer once the slow tail is counted. A normal day sends up to 600, and a cache barely changes that: the popularity of items is a long tail, so fewer than one request in ten finds its list in the cache.

Two things can break the search, and the hidden tests check one each:

- **"A sale triples traffic"** runs the whole test at 1,800 requests a second. That is more searches than one shard can run, whatever the cache does. The fix is more slots: shards. With 2 shards each holds 2.5 million vectors, a search takes about 2 × 2.3 × 2.2 × 1.1 ≈ 11.5 ms, and 32 slots finish about 2,800 searches a second.
- **"One item goes viral"** makes popularity far steeper for the whole test (one item gets about 61% of all views, the next few most of the rest) and multiplies traffic by 6 from second 15 to second 45: 3,600 requests a second, about 2,200 of them for the same item. A given item's searches always go to the same shard, so that one shard gets 2,200 searches a second. No shard can finish that many: adding shards spreads the other items, not this one. The fix is to stop sending those requests to the search at all. Every shopper who opens the viral item gets the same list, so a cache answers them after the first one.

## The reference design

An AWS ALB sends the requests to 4 Go App Servers. They first ask a Redis Cache with 4 GB of memory and a TTL of 300 s; 4 GB holds about 2 million lists of 2 KB. On a miss they search a Qdrant Vector Database with 2 shards and 5 million vectors, then re-rank on NVIDIA Triton, 1 instance with the model type "Classifier".

It costs about $1,631 per month: $734 for the Triton GPU, $520 for the Qdrant card (2 shards at $260), $280 for the App Servers, $72 for the Redis Cache and $25 for the ALB.

On a normal day the Qdrant card is about 23% busy, the App Servers about 9% and the GPU about 14%. The p95 is about 62 ms and there are no errors.

When a sale triples traffic, the shards carry three times the normal day's searches and stay well under full: the p95 stays at about 62 ms, with no errors.

When an item goes viral, the cache answers almost every request for the popular items, so the burst hardly reaches the search. The p95 drops to about 18 ms (most answers come straight from Redis), with no errors.

Each half covers one test, and each alone fails the other:

- Shards without a cache: one Qdrant with 4 shards and no Redis ($2,079) passes the sale, but when the item goes viral its shard overflows and about 10.2% of the requests fail. Even 16 shards ($5,199, over the budget) still fail about 4.4%: the viral item's shard is still one shard.
- A cache without shards: one Qdrant with 1 shard plus the Redis Cache ($1,371) passes the viral test, but the sale's long tail misses the cache and about 41.5% of the requests fail.

For the cost challenge, the proof search found nothing below 88% of the reference's cost ($1,428): the one GPU is nearly half the bill and can't be split, and the search and cache are already small. This problem has no cost challenge.

## Why the first try breaks

A common first try is one Qdrant with 1 shard, no cache, and the same App Servers and Triton GPU. It costs about $1,299 and passes "A normal day": its one shard is about 66% busy, and the p95 is about 70 ms with no errors.

When a sale triples traffic, 1,800 searches a second meet a shard that finishes about 1,000 at best. Its queue fills, and requests that find it full are refused: about 49.8% of the requests fail. The classifier GPU, the most expensive card, never sees them.

When an item goes viral, the same lone shard gets the whole burst, and about 64.1% of the requests fail.

The point is this: in a retrieve-then-re-rank pipeline the retrieval is a server like any other, and it fails in two different ways. Too much traffic overall needs more shards. Too much traffic for one key needs the answer cached, because a key always lands on the same shard.

## What the simulator leaves out

It doesn't judge recommendations: whether the 50 neighbours are the right ones (recall), whether the classifier orders them well, and how fresh the cached lists are compared with a newly re-ranked list are not simulated, and nothing here says how many candidates or which model gives good recommendations. Personalised lists (one per shopper) are not modelled: every shopper gets the same list for an item, which is what lets the cache help. The search time is a modelling choice (a formula over vectors per shard, neighbours and shards), not a measurement of Qdrant; vector dimensions, memory, index builds and index updates are not simulated, and the catalogue doesn't grow during a test. Real vector databases usually search every shard for every query; here each item's searches go to one shard, which is what makes a viral item a hot shard. The classifier is one call per request: it scores the 50 candidates as one batch of about 50 tokens. GPU memory is not simulated. The GPU price is a g5.xlarge-class figure, and the Vector Database price is a teaching figure still marked as needing a source.

## Things to try

- Remove the cache and give the Qdrant card 4 shards. Run "One item goes viral" and watch which shard fills.
- Keep the cache and set the Qdrant card back to 1 shard. Run "A sale triples traffic": how much does the cache help when most items are rarely viewed?
- Set the Cache's TTL to 10 s and run "One item goes viral". Does the viral item still stay off the search?
