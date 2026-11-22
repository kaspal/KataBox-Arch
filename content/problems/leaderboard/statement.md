# Game Leaderboard

## The product

A mobile game has 50 leaderboards, one per mode and region. The game screen shows the top 100 players of a board, and players open it often to see where they stand. When a match ends, the game submits the player's score.

A few boards are hot: the main mode in the biggest regions draws most of the views. The rest are quiet.

## What you must meet

| Requirement | Target |
| --- | --- |
| Peak traffic | 8,000 requests/s |
| Reads | 80% of requests |
| p95 latency | 50 ms or less |
| Availability | 99.9% or more |
| Budget | $1,500/month or less |

## The traffic

- **Read the top 100** (80%). Returns the 100 best scores of one board, a 32 KB result. Requests spread over the 50 boards with a strong skew toward a few of them. An App Server reads it from the store: a Cache used as the store, or a database.
- **Submit a score** (20%). An App Server writes the new score to that same store.

There is no separate source of truth. Whatever you choose as the store holds the leaderboards.

## The tests

Your design is judged by one visible test and two hidden tests.

- **Normal day** (visible). A steady day at peak traffic.
- **The tournament ends** (hidden).
- **The store restarts** (hidden).

Errors must stay under 0.1%.
