# Model Router

## The product

You run the help assistant of a large software product. Users type questions, and a language model answers them. Most questions are easy ("how do I reset my password?", "where is the export button?"), and a small model answers them well enough. About one in five is hard: a long error log, or a question that needs several manuals at once. Those need a large model. Today every question goes to the large model, through a hosted API, and finance has noticed the bill.

A router in front of your services already labels each question easy or hard and sends it to the right service. You decide which model each service calls.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 10 questions/s |
| Time to first token, p95 | 2 s or less |
| p95 latency (the whole answer) | 30 s or less |
| Availability | 99.9% or more |
| Errors | 1% or less |
| Budget | $320,000 per month |

## The traffic

- **Ask an easy question** (80%). The request may pass an API Gateway, reaches the Answers service (App Server cards), and it calls a Model Inference card for a chat answer. An easy question sends about 1,200 tokens to the model (the question plus help articles; 2,400 at p95) and gets about 100 tokens back (220 at p95).
- **Ask a hard question** (20%). The same, but through the Research service (Microservice cards). A hard question sends about 3,000 tokens (6,000 at p95) and gets about 1,200 back (2,400 at p95).

The router's labels come with the traffic: easy questions always reach App Server cards, and hard ones Microservice cards. Your wiring decides which model each of them calls.

Two models are available:

- **The large model**: a Claude API card. It is billed per token and writes about 80 tokens a second (the card's default). Your account's tier allows 4,000 requests and 1,500,000 tokens a minute, and the provider will not raise it this quarter: the simulator applies them to every Claude API card (the Inspector shows them, read-only). A call past the limit is refused with an HTTP 429.
- **The small model**: an open-weights model you run yourself on a vLLM on GPU card. Each GPU instance is billed by the month, whatever it serves, and works on several answers at once (its Batch slots per GPU). The small model is fast: set its Output speed to 150 tokens a second.

A hard answer takes many seconds to stream, so the help window (the Client card) waits up to 60 s for it: set the Client's Timeout to 60,000 ms.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **Twice as many hard questions** and **A newsletter brings three times the easy questions**. A real newsletter brings its wave for hours; the test runs 3 minutes, which is long enough to use up a minute's allowance of tokens. Your design has to pass all three.
