# Rate Limiter

## The product

You run a public API with 50 paying customers. Each customer calls it with their own API key, and each call does a small piece of work against your database. Most customers behave well. Sometimes one key does not: a broken retry loop, a script left running, or someone testing how much you can take. Your job is to keep the API fast for everyone else when that happens, without turning away the customers who are simply busy.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 3,000 requests/s |
| p95 latency | 100 ms or less |
| Availability | 99.9% or more |
| Budget | $1,500 per month |

## The traffic

- **API call** (99%). Calls from the 50 customer keys, about 59 requests/s per key at peak. Each call passes at least one edge component, then an App Server, which reads and writes the database.
- **Calls from one misbehaving key** (1%). Calls from a single extra key, marked abusive. They take the same route. Blocking them counts as a success; blocking a real customer's call counts as an error.

At least one edge component is required in front of the App Servers.

## The tests

One test is visible, and you can run it as often as you like:

- **Normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **One key floods the API** and **Every customer's launch day (×2 for 15 s)**. Your design has to pass all three.
