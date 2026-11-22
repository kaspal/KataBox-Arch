## 1

How much data is there in all 50 leaderboards together, and how many times per second is it read?

## 2

A small, hot data set read thousands of times a second belongs in memory. Sorted sets keep scores in order as they arrive, so reading the top 100 is cheap.

## 3

Use Redis as the store, with no TTL, and add a replica through a Replicate edge with a short failover. Put enough Go App Servers behind the Load Balancer to carry twice the normal traffic.
