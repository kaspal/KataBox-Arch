# Live Streaming with Chat

## The product

You run a live streaming site. Streamers go live, and viewers watch in their browser. The player does not hold one long video download: every few seconds it fetches the next short segment of the stream. Next to the video sits the channel's chat, where viewers send messages and every viewer in the chat sees them arrive over an open WebSocket connection. A few big channels draw most of the audience, and when one streamer sends their viewers to another channel (a raid), that channel's chat floods in an instant.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 5,000 rps and 200,000 open chat connections |
| p95 latency | 300 ms or less |
| Error rate | 1% or less |
| Availability | 99.9% or more |
| Connected chat viewers | 95% or more in every second |
| Budget | $15,000 per month |

## The traffic

There are three request types:

- **Fetch the next video segment** (95%, 4,750 rps). The player fetches a segment from object storage, through an optional edge in front of it. There are 3,000 live segments at any moment, and the big channels get most of the fetches (Zipf 1.2).
- **Send a chat message** (5%, 250 rps). The message goes through an optional edge to an App Server, which writes it to a database.
- **Viewers in the chat**. 200,000 viewers hold an open WebSocket connection, through a load balancer to a WebSocket gateway. They are all connected from the start. Each viewer receives 0.5 chat messages a second on average, so the platform pushes 100,000 messages a second in all. For a viewer, latency is the time to deliver a message. A connection attempt that is refused counts as an error.

There are no outside services in this problem.

## The tests

One test is visible. You can run it as often as you like:

- **A normal evening**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A raid floods one channel** and **The origin storage slows down**. Your design has to pass all three.
