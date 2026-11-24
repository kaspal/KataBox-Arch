# Editorial

## Where the load goes

At 1,000 transfers/s, the ledger takes 1,000 commits a second. PostgreSQL spends about 5 ms on each, so about 5 of its 100 connections are busy: one primary has plenty of room. Every transfer also flows through the stream and ends as a DynamoDB write, which uses three times the capacity of a read and is billed.

A stream keeps one message per partition in flight at a time. A change takes a projector about 2 ms plus a DynamoDB write of about 3 ms, so 1,000 changes a second keep about 5 partitions busy, and twice that during a burst.

Speed is not the hard part. The hard part is that an answered transfer must never be lost.

## The reference design

A Load Balancer (AWS ALB) sends transfers to the Ledger API, 2 Go instances. Each transfer is committed to a PostgreSQL ledger, and a second PostgreSQL is wired to it with a Replicate edge. Both are wired to a Kafka stream of 32 partitions by change data capture edges. 6 Celery Balance projectors, with a service time of 2 ms, consume the stream and update the balances in DynamoDB. This costs about $2,891 per month: DynamoDB about $1,633, nearly all of it the price of 2.6 billion writes a month at $0.625 per million, and Kafka $153. The completion p95 is about 50 ms.

"A retry storm" doubles the traffic for 15 seconds from 20 seconds, and allows a p95 of 15 s. The extra changes wait their turn in the stream and at the projectors, and the p95 rises to about 230 ms.

"The ledger's primary fails mid-transfer" kills the PostgreSQL primary at 20 seconds, for good, in a run of six minutes. It allows 10% errors and 90% availability, and no lost transfers. Transfers fail until the replica is promoted, after PostgreSQL's default failover time of 30 seconds: 30 of 360 seconds, about 8% of the run. Those customers see an error and can try again. A failed write is never committed, so it publishes nothing, and the promoted replica keeps publishing through its own CDC edge. Nothing is lost.

## Why the first try breaks

A common first try is the same design with one PostgreSQL ledger and no replica, for about $2,631 per month. It passes "Normal day" and "A retry storm" with the same numbers as the reference.

When the primary dies there is nothing to promote, so every transfer fails from 20 seconds to the end of the run: about 94% errors. Nothing committed is lost, but the payments app is down until someone rebuilds the database.

The lesson: an error the customer sees can be retried, but a transfer that was acknowledged and then lost cannot. The ledger must be replicated, and the balances are a projection you can always rebuild from its change log.

## What the simulator leaves out

Exactly-once delivery and idempotency are not simulated: a retried transfer is simply a new one, with no key to catch the duplicate. Double-entry bookkeeping is not modelled either. Replication here is asynchronous, with the replica 50 ms behind, and still the simulator loses no committed transfer at failover. Real asynchronous replication can: the last transfers committed before the crash may never reach the replica. That is why real ledgers replicate synchronously, or to a quorum, and pay for it with slower commits.

## Things to try

- Delete the CDC edge from the replica to the stream and run "The ledger's primary fails mid-transfer". Changes committed after the promotion have nowhere to go.
- Set the stream back to 6 partitions and run "A retry storm".
- Lower the ledger's failover time to 10 s and compare the error rate.
