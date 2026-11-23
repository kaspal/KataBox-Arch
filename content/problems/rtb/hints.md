## 1

An auction waits for its bidders, and the slowest one decides when the wait ends. What happens to your latency, and to your App Servers, on a day when one bidder gets slow?

## 2

A bid that arrives after the auction has closed is worth nothing, so there is no point in waiting for it. Give the bidder call a deadline: a late bidder then becomes a lost bid, not a slow auction, and the App Server slot is free again on time. Then make sure you have enough slots for a traffic peak, not just for a normal hour.

## 3

Put an AWS ALB in front of Go App Servers that read profiles from a Redis Cache. Set the App Servers' Call timeout to about 70 ms, well inside the 100 ms p95 target but above the bidders' usual 30 ms. 8 instances at a Concurrency per instance of 256 leave room for twice the normal traffic.
