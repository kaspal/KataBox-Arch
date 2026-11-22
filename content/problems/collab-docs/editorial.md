# Editorial

## Where the load goes

At 6,000 messages/s, about 5,400 are edits and about 600 are snapshots. The snapshots are easy: under 10 MB/s into PostgreSQL.

The edits shape the design. The sync server an edit arrives at publishes it, and every sync server subscribed to the bus receives it. So each sync card works on every edit in the system, not only on its own editors' edits.

The bill follows the edits too. The Redis Pub/Sub costs $0.50 per million messages. 5,400 edits a second for a month is about 14,200 million messages, roughly $7,100: most of the bill.

## The reference design

A Network Load Balancer, which carries WebSockets, spreads editors over two sync cards, Sync A and Sync B, each with 3 Go instances. Both publish edits to a Redis Pub/Sub, the "edit bus", and both consume from it, so an edit typed on Sync A reaches the editors on Sync B. Snapshots go to a PostgreSQL primary, with a second PostgreSQL on a Replicate edge as a standby copy.

It costs about $8,253 per month, $7,108 of it for the bus. On a normal workday each sync card is about 28% busy and the p95 is near 29 ms.

"500 editors in one doc" triples the edits and makes a few documents far more popular than the rest (Zipf 2.0). That creates no hot server, because the bus delivers every edit to every sync server whatever its document. Tripled edits nearly triple each card's work, and both still have room: the p95 stays near 29 ms. The monthly rate shown during this test rises to about $22,600, because the bus bills each extra message.

"A sync server dies" kills Sync A at 20 seconds and allows 3% errors, 97% availability and a p95 of 2 seconds. The load balancer needs two failed health checks, 2 seconds apart, so for 2 to 4 seconds it keeps sending editors to the dead card. About 1.7% of messages fail and the p95 rises to about 1 second. Then Sync B carries everything: each card is sized to do that alone.

## Why the first try breaks

A common first try has the same load balancer, bus and PostgreSQL, without the replica, and a single sync card of 3 instances. It costs about $7,693 per month and passes the normal workday, with the card about 38% busy and a p95 near 27 ms.

In "500 editors in one doc" that card handles every published edit and every delivered one at three times the rate. Its slots fill, edits queue, and about 3.9% of messages fail. In "A sync server dies" there is no second card to take the editors: from 20 seconds on, nothing answers, and about 67% of messages fail.

## What the simulator leaves out

Real editors merge simultaneous edits to the same spot with operational transforms or CRDTs, apply a document's edits in the same order everywhere, and show who is present and where their cursors are. None of that is simulated. This problem measures delivery, the cost of fan-out and failover. At larger scale you would also route each document to a few chosen sync servers instead of sending every edit to all of them.

## Things to try

- Add a third sync card and run the normal workday. Each card still receives every edit: how much less busy does each get?
- Lower each sync card to 2 instances and run "A sync server dies".
- Merge Sync A and Sync B into one card of 6 instances. Compare the cost, then think about what happens when that card stops.
