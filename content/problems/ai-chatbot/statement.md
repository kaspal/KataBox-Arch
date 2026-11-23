# Support Chatbot

## The product

You run an online shop, and you are adding a support chatbot to its help page. A customer types a question; your backend sends it, with the shop's help articles as context, to a hosted language model, and the answer streams back to the chat window word by word. You don't run the model: you pay a model provider per token, through an API key, and the provider limits how much each key may send per minute. Customers judge the bot first by how quickly the first words appear, and they lose patience with an error message much faster than with a slow answer. Marketing is planning a launch that will bring several times the usual traffic, and model providers do have bad days.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 5 messages/s |
| Time to first token, p95 | 1.5 s or less |
| p95 latency (the whole answer) | 12 s or less |
| Availability | 99% or more |
| Errors | 1% or less |
| Budget | $150,000 per month |

## The traffic

- **Send a chat message** (100%). The request may pass a load balancer, reaches your App Servers, and they call a Model Inference card for a chat answer. A message sends about 800 tokens to the model (the question plus context; 1,600 at p95) and gets about 300 tokens back (600 at p95).

An answer takes several seconds to stream, so the chat window (the Client card) waits up to 30 s for it: set the Client's Timeout to 30,000 ms.

For the launch, your provider has already raised your account to its highest tier: each API key (each Model Inference card) gets 4,000 requests and 1,000,000 tokens a minute. There is no tier above it, and the cards show these limits without letting you change them. A call past a key's limit is refused with an HTTP 429 and counts as a failed message, unless the client tries again. When a provider is short of capacity, it may also cut your limit for a while. You may hold more than one key, and you may also use a second provider that serves a comparable model under its own limits, at the same tier. The bill is per token, at the same price on every provider here, so the number of keys and providers does not change it.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Three more tests are hidden, and you see only their names: **Launch day: 4 times the traffic for 4 minutes**, **The model provider is overloaded for 20 s** and **The model provider cuts your limit for 40 s**. A real launch lasts hours; the test runs 4 minutes, which is long enough to use up a minute's allowance. The launch test allows 5% errors and 95% availability, and the overload and limit-cut tests 2% errors and 98% availability. Your design has to pass all four.
