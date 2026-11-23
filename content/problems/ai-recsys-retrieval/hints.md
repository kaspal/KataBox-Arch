## 1

Every miss makes a vector search, and a search holds one of the shard's 16 slots. How many searches a second can one shard finish, and how much of that does a normal day use? Then think about the shopper who opens the same item as thousands of others: which shard does that search go to, and does adding shards change it?

## 2

A search over 5 million vectors in one shard takes about 15 ms, so one shard finishes at most about 1,000 searches a second, fewer once the slow tail counts. A normal day already keeps it about two thirds busy. More shards add slots, but one item's searches always land on the same shard, so a single very popular item can swamp its shard however many there are. The finished list is the same for everyone who opens that item: a cache can answer those requests before they reach the search.

## 3

Put an AWS ALB in front of 4 Go App Servers. Add a Redis Cache with 4 GB of memory and a TTL of 300 s, queried from the App Servers. Search on a Qdrant Vector Database with 2 shards and 5 million vectors. Re-rank on NVIDIA Triton, 1 instance, model type "Classifier".
