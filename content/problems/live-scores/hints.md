## 1

An open connection costs memory on a gateway, and a gateway can fail. If one gateway card disappears, do the others have room for its fans? And what happens when all of those fans try to come back at the same moment?

## 2

Size the gateways for N−1: with any one card gone, the rest must still hold all 5,000,000 fans and keep some message headroom for a burst of updates. Then spread the reconnects out. If every dropped client retries at once, the survivors refuse almost all of them, every second; a random wait before reconnecting turns that storm into a steady stream the gateways can accept.

## 3

Use 4 Socket.IO WebSocket Gateway cards behind an AWS ALB, each with 2 instances, Max connections 1,000,000, Memory 32 GB, Messages per second 200,000 and New connections per second 20,000. Set the Client's Reconnect jitter to 20 s. For the match pages, 4 Go App Servers with a Redis Cache (TTL 2 s) in front of PostgreSQL are plenty.
