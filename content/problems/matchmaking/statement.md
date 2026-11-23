# Game Matchmaking

## The product

You run matchmaking for an online game. A player presses "Find a match", and your service puts them in a queue. A matcher takes players off the queue, puts a match together and writes it to the database, and then the game can start. Your players are in North America and in Europe. Nobody expects a match in an instant, but a player who waits for minutes, or whose request vanishes, closes the game.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 1,000 requests/s |
| p95 completion latency | 5 s or less |
| Availability | 99.9% or more |
| Errors | 1% or less |
| Budget | $3,500 per month |
| Lost requests | None |

## The traffic

- **Find a match** (100%). The request goes through an App Server, which puts the player on a queue and answers at once. A matcher Worker takes the player off the queue, forms the match (about 200 ms of work) and writes it to a NoSQL database. Latency is measured to completion: from the request until the match is written.

There are no outside services in this problem.

## The tests

One test is visible. You can run it as often as you like:

- **A weekday evening**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **Saturday night** and **A Region goes dark**. Your design has to pass all three.
