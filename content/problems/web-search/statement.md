# Web Search

## The product

You run a web search engine. People type a query and expect a page of results in a blink. Behind the search box, a crawler keeps finding pages and sending them in to be indexed. The queries are not spread evenly: a small set of popular searches makes up a large share of them, and when something happens in the news, a huge number of people search for the same thing at once. Results a few minutes old are fine. A search box that returns errors is not.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 8,000 rps |
| p95 latency | 200 ms or less |
| Availability | 99.9% or more |
| Error rate | 0.1% or less |
| Budget | $3,200 per month |

## The traffic

Requests reach your App Servers, through an edge if you want one.

- **Search the web** (95%). An App Server runs the query on a search index, through a cache if you add one. There are 1,000,000 different queries, and a few popular ones get a large share of them (Zipf 1.0).
- **Index a crawled page** (5%). An App Server writes the page into the search index. Writes go to the index's primary. There are 10,000,000 different pages.

There are no outside services in this problem.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A search node dies** and **Everyone searches the same thing**. Your design has to pass all three.
