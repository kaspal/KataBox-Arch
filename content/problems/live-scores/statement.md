# Live Sports Scores

## The product

You run a live scores app. On a match day millions of fans keep the app open, and each one holds a WebSocket connection to your platform. When a team scores, the update is pushed down that connection, and the fan sees it within a second. Fans also open match pages: the line-ups, the stats and the score so far. A fan who is disconnected sees nothing, so almost everyone must stay connected all the time, even when part of your platform fails.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 2,000 page loads/s and 5,000,000 open connections |
| p95 latency | 1 s or less |
| Error rate | 1% or less |
| Availability | 99.9% or more |
| Connected fans | 95% or more in every second |
| Budget | $1,800 per month |

## The traffic

There are two request types:

- **Fans following a match**. 5,000,000 fans hold an open WebSocket connection, through a load balancer to a WebSocket gateway. They are all connected from the start. Each fan receives a score update on average every 50 seconds, so the platform pushes 100,000 messages a second in all. For a fan, latency is the time to deliver an update. A connection attempt that is refused counts as an error.
- **Open a match page** (100% of the page traffic, 2,000 rps). The request goes through an optional edge to an App Server, then to an optional cache and to a database. There are 2,000 matches, and a few big ones get most of the views (Zipf 1.0).

There are no outside services in this problem.

## The tests

One test is visible. You can run it as often as you like:

- **A busy match day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A goal in the last minute** and **A gateway dies at half-time**. Your design has to pass all three.
