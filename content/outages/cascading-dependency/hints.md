## 1

Checkout can render without recommendations, and the route already allows it. So why does checkout fail when the recommendations service hangs? Watch how long each checkout holds its App Server slot.

## 2

A fallback runs only when the call ends, and a call to a hung service never ends on its own: checkout has no call timeout, so it waits until the client gives up. Give the call a limit. Then think about the slow case: a timeout still costs every request its full wait unless something stops calling a dependency that keeps failing.

## 3

On the checkout App Server, set a call timeout of 200 ms and a circuit breaker that opens at 20% failed calls for 30 s. Nothing else needs to change.
