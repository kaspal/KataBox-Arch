## 1

The provider counts tokens per minute, per key, and your tier is as high as it goes. How many tokens a minute does your peak send, and how many will the launch send? And when the only provider you call stops answering, or starts refusing calls, where does a message go?

## 2

At 4 times the traffic, one key at 1,000,000 tokens a minute runs out of tokens after about two minutes, and from then on many calls are refused until the minute's allowance refills. There is no higher tier, so add capacity you can count on: more keys. A second provider is also somewhere to send messages when the first one fails or cuts your limit, and a client that tries again after a short, random wait turns a refused or failed call into a slower answer instead of an error. Health checks notice a provider that is down; they don't notice one that answers 429.

## 3

Put an AWS ALB in front of 2 Go App Servers. Wire them to two Model Inference cards: a Claude API and an Amazon Bedrock. On the Web client, set the Timeout to 30,000 ms, Retries to 3, the Retry backoff to 200 ms and the Backoff jitter to Full (random wait).
