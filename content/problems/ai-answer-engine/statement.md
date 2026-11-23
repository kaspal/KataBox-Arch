# Web Answer Engine

## The product

You run a web answer engine. Someone types a question, and instead of ten links they get a short written answer with its sources. Behind each answer is a pipeline: a keyword search over the web index, an embedding of the question, a search for the closest passages by meaning, and a hosted chat model that writes the answer from what was found. The engine is popular, so the chat model's per-minute limits and its token bill are the two things that can stop it. Answers are the same for everyone who asks the same question, and when a story breaks, a lot of people ask the same question at once.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 200 requests/s |
| p95 latency | 10,000 ms or less |
| Availability | 99% or more |
| Errors | 1% or less |
| Budget | $5,000,000 per month |

## The traffic

- **Ask the answer engine a question** (100%). The request may pass edge cards (a CDN, a load balancer) and reaches your App Servers. The App Server may read a cache first (cache-aside). On a miss it looks the question up in the web index (a Search Index card), embeds it (a Model Inference card with Model type "Embeddings"), asks a Vector Database for the 20 closest passages, and sends the question with what it found to the chat model (a Model Inference card with Model type "Chat (text generation)"). The answer goes into the cache on its way back.

There are about 20 million distinct questions, and some are typed far more often than others (a Zipf popularity of 0.95): people pick them from the search box's suggestions, so a popular question arrives word for word. About a quarter of all questions mean the same as an earlier one in other words. An answer with its sources is about 4 KB. The vector index holds 100 million passages.

A question sends about 1,500 tokens to the chat model (2,500 at p95: the question, the instructions and the passages found) and gets about 250 back (500 at p95). The chat model is the Claude API on your account's tier: a Requests per minute limit of 16,000 and a Tokens per minute limit of 30,000,000. You can't raise that tier this quarter, and someone is waiting for every answer, so the API mode stays Real time. A call over a limit is refused with a 429 and counts as a failed request.

Two more rules:

- **Freshness.** News changes answers, so an answer may be served from a cache (a Cache card or a CDN) for at most 10 minutes: a TTL of 600 s or less.
- **Patience.** An answer takes seconds to write, so the Web client's Timeout is 20,000 ms. Keep it.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A story breaks: everyone asks the same few questions** and **The evening peak lasts four minutes**. Your design has to pass all three.
