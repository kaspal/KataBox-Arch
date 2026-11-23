## 1

During the split both sides of the database could take writes. Which side was allowed to reserve stock, and what decided it?

## 2

A Redis lock goes to whoever can reach Redis; it doesn't know there is a split. Only a lock that needs a majority of its members can refuse the side that was cut off. And the database should refuse writes on a primary that promoted itself, too.

## 3

Replace the Redis lock with an etcd Coordination Service: three cards with 1 instance each, one in AZ-a, one in AZ-b and one in a new AZ-c, joined by Replicate edges from the one in AZ-a, and wire both apps to the one in AZ-a. Turn on Fencing tokens on both PostgreSQL cards. Leave Failover in a split on auto.
