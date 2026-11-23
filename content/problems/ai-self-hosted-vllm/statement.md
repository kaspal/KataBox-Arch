# Chat on Your Own GPUs

## The product

You build the internal assistant of a large insurance company. Staff ask it about policies, claims and procedures, and the answer streams back word by word. The documents it reads must not leave the company's own cloud account, so you don't call a hosted model provider: you run an open-weights model yourself, with vLLM, on GPUs you rent by the month. Staff judge the assistant first by how quickly the first words appear. Twice a year, at renewal time, traffic doubles for hours, and GPU servers do fail.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 20 messages/s |
| Time to first token, p95 | 1 s or less |
| p95 latency (the whole answer) | 15 s or less |
| Availability | 99% or more |
| Errors | 1% or less |
| Budget | $8,000 per month |

## The traffic

- **Send a chat message** (100%). The request may pass a load balancer, reaches your App Servers, and they call a Model Inference card for a chat answer. A message sends about 1,000 tokens to the model (the question plus the documents; 2,000 at p95) and gets back a typical answer of about 250 tokens (600 at p95).

An answer takes several seconds to stream, so the chat window (the Client card) waits up to 30 s for it: set the Client's Timeout to 30,000 ms.

## Your GPUs

Use **vLLM on GPU** Model Inference cards and keep the card's speeds as they are: the first token comes after 200 ms plus the time to read the prompt at 5,000 tokens a second, and then one answer alone is written at 80 tokens a second.

vLLM batches: one GPU writes up to **Batch slots per GPU** answers at the same time, but every answer in the batch slows the others down. With a full batch, each answer is written at half speed. A message that finds every slot busy waits, and that wait is part of its time to first token. Your model and its conversations' memory fit at most **64 answers at once on one GPU**: keep Batch slots per GPU at 64 or less.

A vLLM card is a pool of GPU servers that is deployed together (one node pool, one rollout); its **Instances** are its GPUs, each holding its own copy of the model. A pool can fail as a whole, for example after a bad driver update. Each GPU costs $734 a month.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **Twice the traffic for 2 minutes** and **A GPU card is lost**. A real renewal peak lasts hours; the test runs 2 minutes at double traffic, long enough for any queue to show. The GPU test allows 2% errors and 98% availability. Your design has to pass all three.
