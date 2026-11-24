# Session Store

## The product

You run the web tier of a busy site where most people stay signed in. Every page they load has to know who they are, so the app looks up their session before it renders anything. When someone signs in, the app creates that session and saves it. If the session cannot be found, the user is thrown back to the sign-in page, which feels like the site broke. Sessions have to be fast to read and still be there when a machine fails.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 20,000 requests/s |
| Reads | 90% of requests |
| p95 latency | 50 ms or less |
| Availability | 99.9% or more |
| Budget | $1,400 per month |

## The traffic

Both request types take the same path: an edge component, which must be a Load Balancer, then an App Server, then the session store. The session store is a Cache card used as the place sessions live, not as a cache in front of something else: every request needs it, and there is no database behind it. There are 2 million active sessions, about 2 KB each.

- **Load a signed-in page** (90%). The App Server reads the user's session from the session store.
- **Sign in** (10%). The App Server writes a new session to the session store's primary.

## The tests

One test is visible. You can run it as often as you like:

- **Normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **An app server dies** and **The session store's primary fails**. Your design has to pass all three.
