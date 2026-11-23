# Editorial

## Where the load goes

At 20,000 auctions a second, the work is waiting on the bidders: an App Server holds a slot for each call. By the median alone, 20,000 × 0.03 s keeps about 600 slots busy, and the bidders' tail, up to 90 ms at the p99, adds more. The profile read from Redis is quick and adds little.

The hidden tests take that room away. When a big bidder slows down, every bidder call takes 80 ms longer, about 110 ms at the median. If the auction waits for it, 20,000 × 0.11 s is about 2,200 slots busy at once, and every auction is slower than the 100 ms target. At prime time, 40,000 auctions a second arrive, so even at the normal 30 ms you need about 1,200 slots for the median alone.

## The reference design

An AWS ALB spreads the auctions over 8 Go App Servers, each with a Concurrency per instance of 256 and a Call timeout of 70 ms. They read profiles from a Redis Cache with 16 GB of memory. This costs about $873 per month: $560 for the App Servers, $288 for Redis and $25 for the ALB. In a normal hour the p95 is about 76 ms and the p99 about 82 ms, with no errors, and the App Servers are about 39% busy.

"A big bidder slows down" adds 80 ms to the bidders from 20 seconds in, for 30 seconds. The p95 rises to about 86 ms and the p99 to about 93 ms, with no errors: bids later than 70 ms are dropped, the auction closes on time with the bids it has, and the slot is free again. Because the bidder step is optional, a dropped bid is not an error.

"Prime time" doubles the traffic from 15 seconds in, for 30 seconds. 2,048 slots cover it: the p95 stays at about 76 ms, with no errors.

For the cost challenge, a proof design comes in at 67% of the reference's cost, under the 70% target. It keeps the same shape and the same Call timeout, with fewer App Server instances and a much smaller Redis.

## Why the first try breaks

A common first try is 4 Go App Servers at a Concurrency per instance of 256, with no Call timeout, and the same Redis. It costs about $593 and passes "A normal hour": a p95 of about 76 ms and a p99 of about 102 ms, with the App Servers about 80% busy.

When a big bidder slows down, every auction waits for it. Each one holds an App Server slot 80 ms longer, about 2,200 slots are needed and only 1,024 exist, so the slots run out, requests queue and then fail: about 28% errors, with a p95 of about 1.1 s. Prime time fails too, with about 24% errors and a p95 of about 393 ms: at 80% busy in a normal hour, there is no headroom for twice the traffic.

The point is this: in an auction, or any fan-out with a deadline, the slowest participant sets your latency unless you set a deadline. A call timeout turns a slow bidder into a lost bid, not a slow auction, and it frees the slot for the next one.

## What the simulator leaves out

The bidders are one combined call here, not separate bidders with their own bids. Auction revenue and win rates are not simulated, so a dropped bid is simply gone. The exchange's own timeout, second-price logic and budget pacing are not simulated either.

## Things to try

- Remove the Call timeout from the reference and run "A big bidder slows down" again. Watch how busy the App Servers get.
- Set the Call timeout to 30 ms, then to 95 ms, and compare the p95 and p99 in a normal hour.
- Lower the App Servers to 5 instances and run "Prime time" again. How busy are they at the peak?
