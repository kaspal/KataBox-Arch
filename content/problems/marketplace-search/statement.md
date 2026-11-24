# Marketplace Search

## The product

You run the search box of a marketplace. Buyers type what they want and expect a page of matching listings at once. Sellers edit their listings all day, changing prices, photos and stock, and a buyer should find the new version soon after it is saved. Listings live in a relational database, which is the source of truth, but a relational database is a poor search engine, so searches are served from a search index kept in step with it. A few queries are typed far more often than the rest.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 5,000 requests/s |
| Reads | 95% of requests |
| p95 latency (updates measured to completion) | 200 ms or less |
| Availability | 99.9% or more |
| Budget | $2,400 per month |
| Lost messages | None |

## The traffic

Buyers and sellers reach the Search API, an App Server, usually through a Load Balancer.

- **Search listings** (95%). The App Server may check a Cache first, and otherwise asks the search engine. There are 200,000 distinct queries, each answer about 8 KB, and their popularity follows a Zipf 1.1 curve: a small set of queries makes up most of the searches.
- **Update a listing** (5%). The App Server writes the listing to the primary database. The database then publishes the committed change to a stream (change data capture), a Worker takes it from the stream and writes it into the search index. Latency is measured to completion: until the search index has the change, not just until the App Server answers.

No outside providers are involved.

## The tests

One test is visible. You can run it as often as you like:

- **Normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A reindex at peak** and **Everyone searches for the same thing**. Your design has to pass all three.
