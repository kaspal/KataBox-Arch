The breaker sits inside the caller, wrapped around calls to one dependency. While it is closed, calls pass through and it counts the failures and timeouts. When they pass a threshold, it opens, and from then on calls fail at once, or return a fallback, without touching the dependency.

After a cool-down, the breaker goes half-open and lets a few trial calls through. If they succeed, it closes and traffic returns to normal; if they fail, it opens again and waits once more.

A breaker works best with short timeouts and a useful fallback: a cached value, a default, or a clear message that the feature is unavailable. Without a fallback it still helps, because a fast error frees the caller's threads for the requests that can succeed.
