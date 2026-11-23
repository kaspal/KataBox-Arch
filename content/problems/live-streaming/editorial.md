# Editorial

## Where the load goes

There are two loads here, and they have different shapes. The video is 95% of the requests: 5,000 × 0.95 = 4,750 segment fetches a second, all reads, over only 3,000 live segments, with the big channels taking most of them (Zipf 1.2). Thousands of viewers ask for the same segment within the same few seconds. Sending every one of those fetches to the origin storage saturates it on a normal evening; a CDN that keeps each segment for a few seconds turns 4,750 fetches a second into a small stream of misses.

The chat is a fan-out problem. Only 250 messages a second are sent, but 200,000 viewers each receive 0.5 messages a second: 200,000 × 0.5 = 100,000 messages a second to deliver. The gateways must hold the connections, and they must also push the messages.

The hidden tests push each side. A raid multiplies everything by four for 20 seconds: 19,000 segment fetches a second, 1,000 messages sent and 400,000 messages a second to deliver. A slow origin adds 400 ms to every fetch that reaches it, more than the whole 300 ms p95 limit.

## The reference design

The segments go through a CloudFront CDN with a TTL of 4 s to an Amazon S3 bucket. The chat viewers connect from the Client, with Reconnect jitter set to 10 s, through an AWS ALB to 2 Socket.IO WebSocket Gateway cards. Each card runs 2 instances with Max connections 200,000, Messages per second 200,000 and New connections per second 20,000. That gives 800,000 connection slots for 200,000 viewers and 800,000 messages a second for a normal 100,000. Sent messages go to 2 Go App Servers and DynamoDB with 16 partitions.

This costs $10,624 per month. CloudFront is almost all of it, $9,403, because it is billed per request at 4,750 segment fetches a second. Then come DynamoDB $412, S3 $365, $140 for each gateway card, the App Servers $140 and the ALB $25.

"A normal evening" passes with no errors and a p95 of about 1 ms: almost every fetch is a CDN hit.

"A raid floods one channel" multiplies the traffic by four from 20 seconds in, for 20 seconds: four times the segment fetches, the messages sent and the messages each viewer receives, 400,000 a second in all. It passes with no errors. The gateways can push 800,000 messages a second, so the raid fills half of them. The CDN absorbs the extra fetches, because the raiders all watch the same segments.

"The origin storage slows down" adds 400 ms to every S3 call from 20 seconds in, for 30 seconds. It passes with no errors: the CDN answers almost every fetch, and only the misses see the slow origin.

There is no cost challenge here. The best cheaper design found cost 98% of the reference. The CDN bill dominates, and every segment fetch needs it.

## Why the first try breaks

A common first try is the same design with only one chat gateway card (2 instances with Max connections 150,000 and Messages per second 100,000 each) and no reconnect jitter. That is 300,000 slots and 200,000 messages a second. It costs $10,484 per month, and "A normal evening" is Accepted: 100,000 messages a second fit into 200,000 a second of capacity.

"A raid floods one channel" fails with a p95 of 19 s. 400,000 messages a second meet 200,000 a second of capacity, so a backlog grows for the whole 20 seconds of the raid, about 4,000,000 messages, and it is still draining long after. The chat arrives late for everyone, not only on the raided channel. The connections are fine; it is the messages that do not fit.

The point is this: live video is read-mostly at the edge, so the CDN carries it, and the origin barely matters. Chat is a fan-out problem, so size the gateways by messages a second, with headroom for the moment a raid multiplies them.

## What the simulator leaves out

Video encoding and the ingest path (the streamer's upload and the transcoding) are not simulated, nor are bitrate ladders. Segment sizes in bytes and CDN bandwidth pricing are left out: the CDN is billed per request only. There are no per-channel chat rooms: every viewer receives messages at the platform's average rate, not only for the channel they watch. Moderation is not simulated either.

## Things to try

- Remove the CloudFront CDN from the reference and wire the Client straight to S3, then run "A normal evening". What happens to the origin?
- Remove the second gateway card from the reference and run "A raid floods one channel". Is one card's 400,000 messages a second enough, with no headroom left?
- Lower the CDN's TTL to 1 s on the reference and run "The origin storage slows down". Do more fetches now see the slow origin?
