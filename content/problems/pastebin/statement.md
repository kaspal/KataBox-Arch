# Pastebin

## The product

People paste a block of text, anything from a one-line command to a 1 MB log file, and get a link they can share. Anyone with the link can read the paste. Most visits are reads, and a handful of pastes, such as a link posted in a busy chat, get read far more often than the rest. Writes are fewer, but each one can be large.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 2,000 requests/s |
| Reads | 85% of requests |
| p95 latency | 150 ms or less |
| Availability | 99.9% or more |
| Budget | $8,000 per month |

## The traffic

- **Read a paste** (85%). The request may pass an edge component such as a Load Balancer, then reaches an App Server. The App Server may check a Cache first, and otherwise reads the data store. There are 5 million pastes, and their popularity follows a Zipf 1.0 curve: some pastes are read much more than others.
- **Create a paste (up to 1 MB)** (15%). The App Server writes the paste body to a store. Treat every create as a full 1 MB body.

You choose which store holds the pastes. Think about how each kind of store copes with large payloads.

## The tests

One test is visible, and you can run it as often as you like:

- **Normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A burst of large pastes** and **Cold cache after a restart**. Your design has to pass all three.
