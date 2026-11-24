# Weather API Proxy

## The product

You run a weather app. Every time someone opens it, the app asks your backend for the forecast of their city. You don't make forecasts yourself: you buy them from an outside weather provider, which charges per call and lets you make only a few calls a second. A forecast changes slowly, so a copy a few minutes old is fine. What your users do mind is an error screen, so the app should keep showing a forecast even when the provider has a bad day.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 20,000 requests/s |
| p95 latency | 100 ms or less |
| Availability | 99.9% or more |
| Errors | 1% or less |
| Budget | $970 per month |

## The traffic

- **Get a city's forecast** (100%). The request may pass a load balancer, reaches your App Servers, and they read the forecast from a cache (cache-aside). On a miss, the App Server calls the weather provider and puts the answer in the cache. There are 40,000 cities, and a few big ones get most of the requests (a Zipf popularity of 1.0).

The weather provider answers in about 150 ms at the median, with a slow tail that reaches 600 ms at p99. It accepts at most 200 calls a second from you. Calls over the limit are refused (an HTTP 429) and count as failed requests. An App Server keeps its slot for the whole call.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The weather provider is down for 40 s** and **A cache node restarts**. Your design has to pass all three.
