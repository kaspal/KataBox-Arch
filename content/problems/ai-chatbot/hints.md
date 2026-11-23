## 1

The provider counts tokens per minute, per key. How many tokens a minute does your peak send, and how many will the launch send? And when the only provider you call stops answering, where does a message go?

## 2

At 4 times the traffic, one key at its current tier runs out of tokens within a minute, and every call after that is refused until the minute's allowance refills. Add capacity you can count on: more keys, a higher tier, or both. A second provider is also somewhere to send messages when the first one fails, and a client that tries again after a short, random wait turns a refused or failed call into a slower answer instead of an error.

## 3

Put an AWS ALB in front of 2 Go App Servers. Wire them to two Model Inference cards: a Claude API and an Amazon Bedrock, each with the Tokens per minute limit raised to 1,000,000. On the Web client, set the Timeout to 30,000 ms, Retries to 3, the Retry backoff to 200 ms and the Backoff jitter to Full (random wait).
