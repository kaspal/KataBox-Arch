# Geo-IP Lookup

## The product

You run a lookup service that tells other apps where an IP address is: the country, the region and the city. Ad servers, fraud checks, shops that pick a currency and sites that pick a language all call it, often on every page they serve. The answer comes from a table of about 1 million IP ranges. Nobody writes to it while the service runs; a new copy arrives once a week. Callers wait on your answer before they can do anything else, so it must be fast, and it must keep answering when part of the cloud goes away.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 30,000 requests/s |
| p95 latency | 40 ms or less |
| Errors | 0.1% or less |
| Availability | 99.9% or more |
| Budget | $1,800 per month |

## The traffic

Callers reach the service through an optional edge, such as DNS and a Load Balancer, then an App Server, then a Cache. The Cache is not a shortcut in front of a database here: it holds the whole IP table and is the only place the data lives. There are 1 million IP ranges, and their popularity follows a Zipf 0.8 curve: some addresses are looked up far more often than others.

- **Look up an IP address** (100%). The App Server reads the IP's range from the Cache and returns its location.

You can place cards in Regions. Callers sit outside every Region.

No outside providers are involved.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The busiest hour of the week** and **A Region goes dark**. Your design has to pass all three.
