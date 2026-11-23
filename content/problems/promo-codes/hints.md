## 1

Almost every request only reads a code, and the same few codes are read over and over. One request in twenty changes a count, and many of those hit the same code at the same moment. These are three different worries: the reads, the race on the count, and what happens when the one database stops.

## 2

Codes change rarely compared with how often they are checked, so a cache in front of the database takes most of the lookups off it and frees the App Servers sooner. A count shown at checkout may be a little behind; what must be exact is the redeem. Read-then-write lets two shoppers take the last use; a write that checks the count and takes one off in the same step lets only one of them win. And a single primary with nothing to fail over to takes every redeem down with it.

## 3

Put an AWS ALB in front of 2 Go App Servers. Add a Redis Cache (0.5 GB is plenty, TTL 60 s) for the lookups. On the PostgreSQL card, turn on "Conditional writes (check and set in one step)" and set its Failover time to 10 s, then wire a Replicate edge to a second PostgreSQL card with the same settings.
