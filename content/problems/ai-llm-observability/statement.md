# LLM Usage Analytics

## The product

You run a writing assistant: people chat with it, and your backend sends each message to a hosted language model and returns the answer. Finance and product want to know what every conversation costs and how fast the model answers, team by team. So every chat message also records a trace: which model it called, the tokens in and out, how long it took and what it cost. The traces go to a Data Warehouse, where the analytics dashboards read them. Finance bills each team from these traces, so none may go missing: a message whose trace can't be recorded counts as failed. The dashboards, though, may be a few minutes behind. The person in the chat can't wait for them.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 5 messages/s |
| p95 latency (the whole answer) | 12 s or less |
| Availability | 99% or more |
| Errors | 1% or less |
| Budget | $110,000 per month |

## The traffic

- **Send a chat message** (100%). The request may pass a load balancer and reaches your App Servers, which call a Model Inference card for a chat answer. A message sends about 800 tokens to the model (1,600 at p95) and gets about 300 tokens back (600 at p95). Then its trace goes to the Data Warehouse in one of two ways: the App Server writes it there itself, or it publishes the trace to a Stream, and the Stream's consumer (a Stream Processor or a worker) writes it to the warehouse. The p95 is measured on the chat answer: from the message to the moment the App Server answers.

An answer takes several seconds to arrive in full, so the chat window (the Client card) waits up to 30 s for it: set the Client's Timeout to 30,000 ms.

Your model account allows 4,000 requests and 2,000,000 tokens a minute: the simulator applies them to every Claude API card (the Inspector shows them, read-only). A warehouse is built for big scans, not for small writes: writing one trace holds a warehouse slot for about 300 ms. A Snowflake warehouse has 8 slots per instance and is billed only while it runs.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **Launch day: 4 times the traffic for 2 minutes** and **The warehouse slows down: every write takes 8 s longer for 40 s**. A real launch lasts hours; the test runs 2 minutes. Your design has to pass all three.
