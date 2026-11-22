## 1

Two people are typing in the same document, but the load balancer put them on different sync servers. How does an edit that lands on one server reach the person connected to the other?

## 2

Let every sync server publish its edits to one shared bus and subscribe to it, so each server hears every edit no matter where it came from. Then make sure no single card holds all the sync servers, and that the one left standing can take the whole load, including a meeting that types three times as much.

## 3

Put a Network Load Balancer in front of two Go App Server cards of 3 instances each. Wire both cards to publish to a Redis Pub/Sub and to consume from it, and write snapshots to PostgreSQL. The bus bills per message, so check its share of the monthly cost before you add more of anything.
