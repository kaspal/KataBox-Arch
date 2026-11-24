# Maps and Arrival Times

## The product

You run a maps app. People pan around the map, which loads it tile by tile, and they ask for a route from where they are to where they are going, with an arrival time. The route comes from your own road graph. The arrival time is better with live traffic, which you buy from an outside traffic feed. A route with an arrival time based on typical speeds is still useful; a route that never comes back is not.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 4,000 rps |
| p95 latency | 250 ms or less |
| Availability | 99.9% or more |
| Errors | 0.5% or less |
| Budget | $11,000 per month |

## The traffic

- **Load a map tile** (70%). The tile is read from object storage, through a CDN if there is one. There are 2,000,000 tiles, and a few of them (the busy cities) are asked for far more than the rest. A tile can be cached.
- **Plan a route with an arrival time** (30%). An App Server walks the road graph in a graph database: 6 hops, with 3 roads leaving each junction, about 1,100 relationships per route. Then it asks the live traffic feed to adjust the arrival time.

The traffic feed answers in about 40 ms at the median, with a slow tail up to 200 ms. The call to it is optional: if it fails or times out, the route still comes back, with an arrival time from typical speeds.

## The tests

One test is visible. You can run it as often as you like:

- **A normal afternoon**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **Rush hour** and **The traffic feed goes down**. Your design has to pass all three.
