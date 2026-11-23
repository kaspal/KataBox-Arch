## 1

Most requests are map tiles, and the same few tiles are asked for again and again. Then ask yourself: the traffic feed only makes the arrival time a little better. What should a route do when the feed stops answering?

## 2

Put a CDN in front of the tiles so most of them never reach object storage. For the routes, an optional call is only optional if your servers stop waiting for it: a short call timeout caps how long a slow call holds a server slot, and a circuit breaker stops calling a feed that keeps failing, so routes come back at once with typical-speed arrival times.

## 3

Wire the Client to a CloudFront CDN with a TTL of 86,400 s (one day), in front of Amazon S3. Send routes through an AWS ALB to Go App Servers with 5 instances, the Call timeout set to 150 ms and the Circuit breaker at errors set to 50%, and wire them to a Neo4j Graph Database with 3 instances.
