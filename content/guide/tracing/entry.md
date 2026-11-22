Distributed tracing follows a single request as it moves through a system. The first service gives the request a **trace id**, and every service it calls records a **span**: what it did, when it started and how long it took. The tracing backend joins the spans into one timeline, so you can see the whole journey of the request.

Its strength is answering "where did the time go?" when a request crosses many services. Metrics show that the tail latency is high; a trace shows that the time was spent waiting on one slow call three hops down.

Storing every trace is costly, so most systems keep only a **sample**. Tracing never sits on a request's path, so in the simulator it adds no latency and cannot drop a request. It only adds its monthly cost, and a problem may require it as part of a production-ready design.
