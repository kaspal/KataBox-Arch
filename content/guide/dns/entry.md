DNS is the internet's phone book. Before a browser can send a request to example.com, it asks a **resolver** for the address behind that name. The answer comes with a **TTL**, and resolvers and clients keep it for that long, so most requests never trigger a fresh lookup.

Because answers are cached and the service is spread across many servers, DNS is almost never what limits a system. It matters more for where traffic goes: managed DNS can send users to the nearest region, or away from one that has failed, though only as fast as cached answers expire.

In the simulator, a DNS card is a fast hop on the request's path with practically unlimited capacity. It adds a small lookup time and a per-query price. The TTL is a design note: it does not change the run.

**How the simulator bills this.** Clients and resolvers keep each answer for its TTL, so only about 1 request in 100 reaches the DNS service as a query, and only those are billed. That makes DNS nearly free in every design. Route 53 and Cloudflare DNS are both billed at $0.40 per million queries (list price, us-east-1, read 2026-11-22).
