# Web Crawler

## The product

You run a crawler that keeps a fresh copy of web pages for a search product. A frontier of URLs waits to be visited. Each URL has to be fetched from the site that hosts it, then stored. Fetching is slow, and the web is not yours: sites answer at their own pace, and you may only ask so often. Pages link to each other all the time, so the frontier keeps handing you URLs you have already visited.

## What you must meet

| Goal | Target |
| --- | --- |
| Crawl rate | 800 URLs/s |
| p95 completion time | 4 s or less |
| Availability | 99.9% or more |
| Budget | $7,500 per month |

## The traffic

- **Crawl a URL** (100%). A Scheduler, the crawl frontier, publishes URLs to a Queue. A Worker takes each one and may check a Cache of seen URLs first. Then it fetches the page from the web and writes it to a store. The web answers in about 250 ms at the median, with a slow tail of a few seconds, and allows 1,000 fetches/s. Calls above that limit fail at once. There are 2 million URLs, and how often each one is rediscovered follows a Zipf 0.9 curve: some come back again and again.

Latency is measured from the moment a URL is published until its work is complete.

## The tests

One test is visible. You can run it as often as you like:

- **Normal crawl**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A site full of duplicate links** and **Slow hosts**. Your design has to pass all three.
