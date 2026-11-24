# Chat Provider Failover

## The product

You run the support assistant inside a shopping app. A customer types a question, your backend sends it to a hosted language model with some context about their order, and the answer streams back into the chat. You buy the model from an outside provider, and providers have bad days: sometimes the API is down, and sometimes it accepts calls and then never answers. A customer stuck on a spinner for a minute gives up and calls the support line, so the assistant should keep answering even when one provider fails. You also have an account with a second provider.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 20 messages/s |
| p95 latency (whole answer) | 6,000 ms or less |
| p95 time to first token | 1,500 ms or less |
| Availability | 99% or more |
| Errors | 1% or less |
| Budget | $190,000 per month |

## The traffic

- **Send a chat message** (100%). The message may pass a load balancer and reaches your App Servers. Each App Server calls a chat model (a Model Inference card with the Model type "Chat (text generation)") and waits for the whole answer. A prompt is about 600 tokens (up to about 1,500), and an answer is about 60 tokens (up to about 150).

The chat app waits up to 30 s for an answer: set the Web client's Timeout to 30,000 ms.

Your two provider accounts have these limits per minute. The simulator applies them to every card of that provider (the Inspector shows them, read-only):

| Card | Requests per minute limit | Tokens per minute limit |
| --- | --- | --- |
| Claude API | 4,000 | 2,000,000 |
| OpenAI API | 1,500 | 1,000,000 |

A call over a limit is refused at once (an HTTP 429). When an App Server is wired to more than one chat model, it sends its calls to them in turn. Health checks notice a card that stops answering after about 4 s, and from then on the App Servers skip it; a card that answers slowly still looks healthy.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A model provider is down for 60 s** and **A model provider hangs for 60 s**. In both, the provider that fails can be either of your two. Your design has to pass all three.
