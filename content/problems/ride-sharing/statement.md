# Ride Sharing

## The product

You run a ride-sharing app. Every driver on shift sends their position every few seconds, so the service always knows where the cars are. When a rider asks for a car, the service looks up the drivers near them and picks one. Riders are what people see, but drivers are what the servers feel: location pings outnumber ride requests by far, and each one is a write. Some drivers are on the road much more than others, so their pings are more frequent.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 15,000 requests/s |
| p95 latency | 60 ms or less |
| Availability | 99.9% or more |
| Budget | $25,000 per month |

## The traffic

- **A driver reports their location** (98%). The request passes an edge component such as a Load Balancer, then reaches an App Server, which writes the new position to the data store. There are 50,000 drivers, and their activity follows a Zipf 0.8 curve.
- **A rider asks for a car** (2%). The request passes the edge, then the App Server reads a nearby-drivers index from a Cache, which here acts as the store for that index. There are 2,000 areas to look up.

Every ping is a write. Nothing about a location is worth caching for later: the next ping replaces it.

## The tests

One test is visible. You can run it as often as you like:

- **Normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **City-wide surge** and **Everyone at the stadium**. Your design has to pass all three.
