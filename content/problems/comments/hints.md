## 1

Each thread is one key. Where does that key live in your data store, and what happens to that place when everybody wants the same thread at once?

## 2

A partitioned store picks the partition by the key, so all the reads of one thread land on one partition. More partitions spread many threads, but they never split one thread. Something that answers a popular key from memory, before the store, takes that load off the partition. Make sure the store can still take the misses if that copy is ever lost.

## 3

Put a Redis Cache of about 4 GB with a TTL of 300 s between Go App Servers and Cassandra, and wire the App Servers to both. With the cache in front, one Cassandra partition is enough for the misses and the comments. 4 Go App Server instances behind an AWS ALB carry 6,000 requests a second with room for a spike.
