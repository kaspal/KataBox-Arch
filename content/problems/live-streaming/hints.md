## 1

Almost all the traffic is viewers fetching the same few segments of the same few channels. Does every one of those fetches have to reach the storage that holds the video? And the chat: what happens to the number of messages to deliver when four times the people are chatting and watching?

## 2

Live video is read-mostly: thousands of viewers ask for the same segment within a few seconds. A CDN in front of the object storage answers them from its own copy, so the origin sees only the misses, and a slow origin barely shows. The chat is a fan-out problem: every message sent is delivered to many viewers. Size the WebSocket gateways for messages a second, not only for connections, and keep enough headroom for a raid that multiplies the chat by four.

## 3

Put a CloudFront CDN with a TTL of 4 s in front of an Amazon S3 bucket for the segments. For the chat, wire the Client through an AWS ALB to 2 Socket.IO WebSocket Gateway cards, each with 2 instances, Max connections 200,000, Messages per second 200,000 and New connections per second 20,000: 800,000 messages a second for a raid of 400,000. Set the Client's Reconnect jitter to 10 s. Sent messages go to 2 Go App Servers and DynamoDB with 16 partitions.
