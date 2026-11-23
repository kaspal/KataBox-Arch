# Summarise a Document Backlog

## The product

Your company keeps a large archive of documents, and the search page wants a short summary next to each one. Every night a scheduled job picks up the documents that arrived during the day and asks a hosted language model to summarise each of them. Nobody reads the summaries until the next morning, so no person waits on any single call. What matters is that the whole backlog is done by the deadline, without errors and without lost documents, and that the token bill stays sensible.

**Scaled time.** A real backlog is summarised overnight, over hours. The simulator runs minutes, so the problem is scaled down: the backlog lands in the first 30 seconds of a two-minute run, and "by morning" becomes "within 45 seconds of being queued". The per-minute limits of the model provider are kept at their real size, so the backlog against the limit is what you have to get right.

## What you must meet

| Goal | Target |
| --- | --- |
| Backlog | 15 documents/s for 30 s (450 documents) |
| Deadline (p95, from queued to summarised) | 45 s or less |
| Availability | 99.9% or more |
| Errors | 0.1% or less |
| Lost documents | none |
| Dead-lettered documents | none |
| Budget | $38,000 per month |

The monthly cost is what the run's rate would cost if it went on for a whole month, the same rule as every other problem. Token prices are the registry's teaching prices.

## The traffic

- **Summarise a document** (100%). A Scheduler publishes each document to a Queue. A Worker takes it from the Queue and calls a Model Inference card (a chat model). The time measured is from the moment the document is queued until its summary is done.

A document is about 600 tokens long (1,200 at p95), and its summary about 100 tokens (200 at p95). The model provider's account allows 4,000 requests and 400,000 tokens a minute in real time (the card's "Requests per minute limit" and "Tokens per minute limit"); those are your account's limits, and you can't raise them for this job: the simulator applies them to every Claude API card (the Inspector shows them, read-only). A call past a limit is refused with an HTTP 429. A Worker keeps its slot for the whole model call.

When a call fails, the Queue tries the document again after a backoff, up to its "Maximum retries". A document that fails every retry goes to a Dead-Letter Queue if one is wired to the Queue; otherwise it is lost. A document in the Dead-Letter Queue was never summarised, so the judge counts it: none is allowed.

## The tests

One test is visible. You can run it as often as you like:

- **Tonight's backlog**: 450 documents queued in the first 30 seconds.

Two more tests are hidden, and you see only their names: **Three times the backlog** and **The model provider slows down**. Your design has to pass all three.
