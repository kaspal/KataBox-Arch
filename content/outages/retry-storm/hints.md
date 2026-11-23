## 1

The slowdown lasted ten seconds, but the errors didn't stop when it did. What was still hitting the database after 25 s?

## 2

Every timed-out call came back three more times, at once: four times the load, arriving exactly when the database had no room. Retries need to wait, spread out, and stop when too many are failing.

## 3

On the Client, keep 3 retries but set a backoff of 200 ms, full jitter, and a retry budget of 10%. Nothing else needs to change.
