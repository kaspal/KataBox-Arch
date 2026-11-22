## 1

Can one database primary absorb tens of thousands of small writes a second, with room to spare when the city gets busier?

## 2

Partition the location writes across a NoSQL Store, so each partition takes only a slice of the drivers. Then think about what a very popular key does: it always lands on the same partition.

## 3

Use Cassandra with enough partitions that the peak, and a good deal more, fits comfortably; each partition handles about 10,000 writes a second. Keep the nearby-drivers index in a Redis Cache, and give the App Servers headroom above the peak.
