# An Assistant for the US and the EU

## The product

You run a writing assistant with users in the United States and in the European Union. A user sends a message, the assistant answers it word by word, and the conversation is kept so the user can come back to it. You serve an open-weights model yourself, with vLLM, on GPUs you rent by the month.

Your EU customers' contracts say where their conversations may be stored: **the conversation history of an EU user must be stored only in EU Regions**, and that holds on a bad day too, when a database fails over. Your legal team has cleared the model itself to answer EU messages from GPUs in either continent; the rule is about where the history is kept.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 30 messages/s |
| Time to first token, p95 | 1 s or less |
| p95 latency (the whole answer) | 15 s or less |
| Availability | 99% or more |
| Errors | 1% or less |
| EU history stored outside the EU | never (0 residency violations) |
| Budget | $12,000 per month |

## The traffic

- **A US user sends a message** (60%).
- **An EU user sends a message** (40%). Its data must stay in EU Regions.

Both go the same way. The request may pass a DNS card and a load balancer and reaches your App Servers. They save the user's message to the conversation history (a write to a database), call a Model Inference card for a chat answer, and save the answer (a second write). A message sends about 800 tokens to the model (the question and the recent conversation; 2,000 at p95) and gets back a typical answer of about 250 tokens (600 at p95).

An answer takes several seconds to stream, so the chat window (the Client card) waits up to 30 s for it: set the Client's Timeout to 30,000 ms.

## Regions and residency

Draw a Region frame for each cloud region you use, and give the Region a **Residency tag** in its inspector: `eu` for an EU region, `us` for a US one. Each time a database, cache or queue stores or serves an EU user's message, the test checks the Region it sits in: outside a Region tagged `eu`, that is a residency violation. A card in no Region at all counts as outside. Model Inference cards are not checked: the rule covers stored history only.

Nothing routes an EU message by itself: a DNS card spreads users over the Regions behind it, US and EU alike, and an App Server takes turns between the Model Inference cards it is wired to. Your design must make sure that wherever a message is served, the history it writes ends up in the EU. US users' history may be stored anywhere.

A call from one Region to another takes about 70 ms each way in the simulator.

## Your GPUs and your database

Use **vLLM on GPU** Model Inference cards and keep the card's speeds as they are: the first token comes after 200 ms plus the time to read the prompt at 5,000 tokens a second, and then one answer alone is written at 80 tokens a second. A GPU writes up to **Batch slots per GPU** answers at once, and with a full batch each answer is written at half speed. Your model and its conversations' memory fit at most **64 answers at once on one GPU**: keep Batch slots per GPU at 64 or less. A vLLM card's **Instances** are its GPUs; each GPU costs $734 a month, and a card can fail as a whole.

The history is kept in PostgreSQL. Your platform team runs it with automatic failover tuned to promote a replica 10 s after the primary is lost: set the SQL Database cards' **Failover time** to 10 s. Replicas are SQL Database cards wired to the primary with Replicate edges.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The Region with the history database goes dark** and **An EU-only audit day: the database primary fails, then a GPU pool**. In the audit test every message comes from an EU user, at the same total rate. Each hidden test runs 5 minutes and allows 8% errors and 92% availability while things fail over, but no residency violations at all. Your design has to pass all three.
