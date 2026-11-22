# Flash-Sale Ticketing

## The product

You sell tickets for big events. Most days are calm: people look at event pages and now and then buy a seat. Then a popular concert goes on sale at a set time, and a crowd that has been waiting with the page open all arrives together. Buying a ticket means charging the buyer through an outside payment provider, which is slow and has its own rate limit. The buyer does not need the charge to finish before seeing an answer, only to know the order was taken.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic before the sale | 2,000 requests/s |
| Browsing | 90% of requests |
| p95 latency | 100 ms or less |
| Availability | 99.9% or more |
| Budget | $8,000 per month |

## The traffic

- **Browse the event page** (90%). The request may pass an edge component such as a CDN, then is served by an origin: an App Server or Object Storage. There are 200 event pages, and every page can be cached.
- **Buy a ticket** (10%). The request may pass the edge, then reaches the Checkout API on an App Server, which puts the order on a Queue. The buyer gets an answer as soon as the order is queued. Payment Workers then charge the payment provider, which takes about 300 ms at the median and accepts up to 2,000 charges a second, and write the ticket to the database.

## The tests

One test is visible. You can run it as often as you like:

- **Before the sale**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The sale opens** and **The payment provider slows down**. Your design has to pass all three.
