# URL Shortener

## The product

You run a service that turns long web addresses into short ones. Someone pastes a long link and gets back a short code to share. Almost all of the traffic comes from the other side: people following those short links and expecting to land on the original page at once. Each redirect is tiny, but there are a lot of them, and a few popular links get far more clicks than the rest.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 12,000 requests/s |
| Reads | 95% of requests |
| p95 latency | 80 ms or less |
| Availability | 99.9% or more |
| Budget | $2,100 per month |

## The traffic

- **Follow a short link** (95%). The request may pass an edge component such as a Load Balancer, then reaches an App Server. The App Server may check a Cache first, and otherwise reads the data store. There are 10 million links, and their popularity follows a Zipf 1.1 curve: a small set of links gets most of the clicks.
- **Create a short link** (5%). The App Server writes the new link to the primary database.

## The tests

Two tests are visible. You can run them as often as you like:

- **Normal day**: the traffic above, at peak.
- **One link everyone clicks**: the same volume, with clicks crowding even more onto the most popular links.

Two more tests are hidden, and you see only their names: **A link goes viral** and **The primary database dies**. Your design has to pass all four.
