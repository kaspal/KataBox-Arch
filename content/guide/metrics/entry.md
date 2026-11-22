Metrics are numbers measured again and again over time: requests per second, error rate, latency percentiles, CPU, queue depth. Each service exposes its numbers, a collector such as Prometheus gathers them every few seconds, and a tool such as Grafana draws them as graphs and fires alerts when a line crosses a threshold.

They are the first place to look when something feels wrong. A metric tells you *that* latency has climbed and roughly where; logs and traces then tell you *why*. Because they store only numbers, metrics stay cheap even at very high traffic.

Metrics never sit on a request's path, so in the simulator they add no latency and cannot drop a request. They only add their monthly cost, and a problem may require them as part of a production-ready design.
