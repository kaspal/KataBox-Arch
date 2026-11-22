# Editorial

## Where the load goes

At peak, the 50 customer keys send about 2,970 requests/s between them, close to 59 each. The misbehaving key adds about 30 more. That is easy work for two App Servers and one PostgreSQL database. The danger is not the total; it is that a single key can multiply its rate without warning. In "One key floods the API" the bad key sends 300 times its usual rate, about 9,000 extra requests/s. Everything behind the edge is shared, so if those calls get in, they take the threads that real customers need.

## The reference design

A Rate Limiter (Envoy) sits at the edge with a limit of 200 requests/s per key and a burst of 400. Behind it are 2 Go App Servers and one PostgreSQL database. The whole design costs about $520 per month.

Because the Rate Limiter keeps one token bucket per key, the flood drains only the bad key's bucket. Its extra calls are shed at the edge, which the judge counts as a success, and the App Servers never see them. The other 50 buckets stay full.

On "Every customer's launch day (×2 for 15 s)", each customer peaks near 119 requests/s. That is well inside the 200 limit, so nobody real is turned away.

## Why the first try breaks

The usual first try puts a Load Balancer in front of 2 Go App Servers and PostgreSQL. It passes a normal day. During the flood, the Load Balancer forwards every call, the App Servers saturate, and about 22% of everyone's requests fail. The honest customers pay for one key's behaviour.

The opposite mistake is a tight limit. A Rate Limiter at 100 requests/s per key looks safe on a normal day, since 59 is below 100. On launch day each customer needs about 119, so the limiter starts shedding legitimate calls, and each of those counts as an error. Leave headroom above a real customer's busiest moment, not their average.

## Things to try

- Lower the per-key limit to 100 and run launch day. Then try 150.
- Set the burst to 0 and see whether short spikes from real customers start to fail.
- Replace the Rate Limiter with a Load Balancer and add App Servers until the flood passes. Compare the cost.