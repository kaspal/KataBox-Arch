## 1

During the spike the Cassandra card is only about half busy on average, yet requests are dropped. Where is the load actually going?

## 2

Almost half of all page loads ask for one product, and a key always lands on the same partition. That one partition is full while the other three idle. More partitions won't help: the hot key still lives on one of them. The hot product needs to be served from somewhere that isn't a partition.

## 3

Put a Redis cache (4 GB, TTL 300 s) in front of Cassandra: connect the App Servers to the cache with a query edge and keep their edge to Cassandra for the misses. Nothing else needs to change.
