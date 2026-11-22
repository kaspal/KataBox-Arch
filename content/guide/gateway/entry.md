An API gateway is the single entry point for the APIs of many services. Every call passes through it, and on the way it does the jobs every API needs: checking who the caller is, routing the call to the right service, applying quotas, and logging. Each service can then focus on its own work.

That convenience has a cost. The gateway does real work on every request, so it adds a few milliseconds, and it must be sized like any other tier. If it is too small, it becomes the bottleneck for everything behind it.

In the simulator a gateway is a hop with its own capacity, instances times concurrency, and a processing time per request. Quotas per caller are modelled by the Rate Limiter card, so wire one in when the problem is about abuse.
