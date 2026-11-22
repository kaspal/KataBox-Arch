**Serverless** compute runs your code only when a request or an event arrives. You upload a function or a container, and the provider starts instances as traffic grows and removes them when it stops. You never size a fleet, and you pay for the requests you serve rather than for machines left running.

The catch is the start. When a request needs an instance that isn't running yet, it waits for one to boot: a **cold start**, from a fraction of a second to a few seconds. There is also a ceiling on how many requests can run at once. Above it, the provider throttles new requests instead of queueing them.

In the simulator a Serverless card has no instances to set. Its capacity is the concurrency limit, and each request holds its slot until its own calls have answered. A request that takes concurrency to a new high for the run pays the cold start; instances stay warm after that. Requests beyond the limit are throttled almost at once, and the card is billed per request only.
