A limiter sits in front of the service and keeps a counter per caller, usually per API key or user. The common form is a token bucket: each caller's bucket refills at a steady rate up to a maximum, every request takes a token, and a request that finds the bucket empty is refused with a "too many requests" reply.

The refill rate sets the long-run limit and the bucket size sets how big a burst is allowed, so normal users with bursty clicks pass while a script hammering the API is cut off. Refusing early, before any real work, is what makes the limit cheap.

With several limiter nodes the counters must be shared, often in a fast in-memory store, or each node enforces its own slice of the limit. Tell callers how long to wait before retrying, so well-behaved clients back off instead of retrying at once.
