# Real-Time Ad Auction

## The product

You run an ad exchange's auction service. Each time a web page or an app has a free ad slot, an ad exchange sends you a bid request and waits for your answer. You look up what you know about the user, ask the bidders what they will pay, and send back the result. The exchange gives you only a few milliseconds: an answer that comes too late is worth nothing, and the ad slot goes to someone else. A missing bid now and then is fine. A slow auction is not.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 20,000 rps |
| p95 latency | 100 ms or less |
| p99 latency | 150 ms or less |
| Error rate | 0.1% or less |
| Availability | 99.9% or more |
| Budget | $1,200 per month |

## The traffic

The requests come from the ad exchanges' servers, not from people.

- **Run an ad auction** (100%). An App Server reads the user's profile from the profile store (a cache), then calls the bidders and closes the auction with the bids it has. The bidder call is optional: if it fails or takes too long, the auction still closes, just without those bids. There are 5 million user profiles, and some users are seen much more often than others.

The bidders are an outside provider. One call stands for all of them together, so the slowest bidder sets how long it takes: about 30 ms at the median and 90 ms at the p99. An App Server keeps its slot for the whole call.

## The tests

One test is visible. You can run it as often as you like:

- **A normal hour**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A big bidder slows down** and **Prime time**. Your design has to pass all three.
