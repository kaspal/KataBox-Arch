# Chat

## The product

You run a chat service. People keep the app open all evening and send short messages to friends and groups. Each phone holds a long-lived WebSocket connection to one of your gateway servers. A message arrives at the sender's gateway, but the person it is for may be connected to a different one. It still has to reach them quickly. When a conversation stalls in the middle of a sentence, people notice at once.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 4,000 messages/s |
| p95 delivery latency | 150 ms or less |
| Availability | 99.9% or more |
| Budget | $12,000 per month |

In this problem, pub/sub requests make up most of the bill. Count them when you plan.

## The traffic

- **Send a message** (100%). The message travels over a WebSocket through an edge component to a gateway, which is an App Server. The gateway publishes the message to a broker and is done with it. The gateway that holds the recipient's connection consumes it and delivers it. Latency is measured from the moment the message is sent until it is delivered, not only until the first gateway accepts it.

Every message takes this path, so there is no read and write split to lean on here. What matters is how messages move between gateways, and what happens when a gateway is not there.

## The tests

One test is visible. You can run it as often as you like:

- **Normal evening**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A gateway dies** and **Evening peak**. Your design has to pass all three.
