# Editorial

## Where the load goes

At 4,000 messages/s, every message passes through compute twice: once at the sender's gateway, which publishes it, and once at the recipient's gateway, which delivers it. Between the two sits the broker. A Pub/Sub hands a copy of each message to every subscribed gateway, and the pub/sub requests grow with the traffic: they are most of the bill. The work per message is small. The hard part is keeping delivery going when a gateway fails and when the evening gets busy.

## The reference design

A Network Load Balancer that carries WebSockets spreads connections over two gateway cards, Gateway A and Gateway B, each with 2 Go instances. Both publish to a Redis Pub/Sub "message bus", and both consume from it, so whichever gateway holds the recipient can deliver.

This costs about $5,590 per month and has a p95 near 28 ms on a normal evening. When Gateway A dies, the messages it was handling fail, and for a short while the Load Balancer keeps sending to it. Health checks notice in 2 to 4 seconds, and connections move to Gateway B. About 1.7% of messages fail and some deliveries wait about a second; then Gateway B carries everything. The design also handles the evening peak.

## Why the first try breaks

A common first try is one gateway card with 2 instances that publishes to and consumes from the bus. On a normal evening it is fine, with a p95 near 26 ms, even a little faster than the reference.

The hidden tests find the gaps. "A gateway dies" kills the gateway card at 20 seconds. It allows up to 2% errors and a p95 of 2 seconds, because a short gap is expected. With only one card there is nowhere to move the connections, and about 67% of messages fail. "Evening peak" triples the traffic for 25 seconds. Two instances cannot keep up: their threads fill, messages queue, then fail, and about 5% are lost.

## Things to try

- Remove Gateway B and run "Normal evening". Compare the cost and p95, then ask what the second card is buying you.
- Add a third gateway card and kill one again: compare the errors while health checks catch up.
- Lower each gateway to 1 instance and see how close a normal evening gets to the limit.
