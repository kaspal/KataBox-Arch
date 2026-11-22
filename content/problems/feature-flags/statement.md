# Feature Flags

## The product

You run a feature-flag service. Other teams' services ask it, many times for every page they serve, whether a flag is on: show the new checkout, turn on the beta search, hide a broken button. Every answer has to be fast, because the caller waits for it. Now and then someone flips a flag, and they expect the change to reach every service within seconds, not minutes. And if the database behind the flags has a bad moment, the services asking must not start failing with it.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 60,000 requests/s |
| Reads | 99.9% of requests |
| p95 latency | 30 ms or less |
| Availability | 99.9% or more |
| Budget | $2,500 per month |
| Stale reads | 5% or less |

A read is stale when it returns a value older than the flag's latest flip.

## The traffic

- **Evaluate a flag** (99.9%). The request may pass an edge component such as a Load Balancer, then reaches an App Server, the Flags API. The App Server may check a Cache first, and otherwise reads the data store. There are 20,000 flags of about 1 KB each, and their popularity follows a Zipf 1.1 curve: a small set of flags gets most of the reads.
- **Flip a flag** (0.1%). The App Server writes the new value to the primary database. Flips are spread evenly over all 20,000 flags.

The App Server keeps its slot while it calls the Cache and the database, so a slow database holds its slots for longer.

## The tests

One test is visible. You can run it as often as you like:

- **Normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A flip storm** and **The config database is down for 20 s**. Your design has to pass all three.
