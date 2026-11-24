# Collaborative Docs

## The product

You run a document editor where several people can work on the same page at once. While a document is open, the editor keeps a WebSocket to one of your sync servers, and every keystroke goes out as a small edit. Two people in the same document may be connected to different sync servers, yet each has to see the other's typing almost as it happens. Every so often the editor also saves a full snapshot of the document, so that nothing is lost if everyone closes it. When the other person's cursor freezes or their words arrive seconds late, writing together stops working.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 6,000 messages/s |
| p95 completion latency | 150 ms or less |
| Availability | 99.9% or more |
| Budget | $1,400 per month |

## The traffic

Both message types arrive over a WebSocket through an edge component to a sync server, which is an App Server.

- **Type an edit** (90%). The sync server publishes the edit to a broker. Every sync server that holds editors of that document has to receive it, so it can pass it on to them. An edit counts as done when it has reached the sync servers, not when the first one accepts it, and its latency is measured to that point.
- **Save a snapshot** (10%). The sync server writes a copy of the document, about 16 KB, to a database.

There are about 100,000 documents open at peak. Plan the broker's cost as carefully as its speed: a Pub/Sub card bills for each message it carries, and edits are most of the traffic.

## The tests

One test is visible. You can run it as often as you like:

- **Normal workday**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **500 editors in one doc** and **A sync server dies**. Your design has to pass all three.
