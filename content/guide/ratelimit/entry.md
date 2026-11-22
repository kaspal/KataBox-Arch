A rate limiter decides, for each request, whether its sender is still within their allowance. The common design is a **token bucket** per caller: tokens drip in at a steady rate up to a maximum, each request takes one, and a request that finds the bucket empty is turned away at once instead of reaching your servers.

The rate sets what a caller may send over time, and the bucket size, the **burst**, sets how much they may send at once after a quiet spell. Because each caller has their own bucket, one misbehaving key drains only its own allowance, and everyone else is served as usual.

In the simulator the limiter keeps one bucket per API key. Shedding traffic the problem marks as abusive counts as a success; shedding a real user's request counts as an error. So the limit must sit above real customers' peaks but well below a flood.
