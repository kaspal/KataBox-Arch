# RAG for Many Tenants

## The product

You run a question-answering service that businesses buy for their own documents. Each business is a tenant: its employees ask questions, and the service answers them from that tenant's manuals, policies and tickets. One shared platform serves everyone. An embedding model turns the question into a vector, a vector search finds the 50 passages of that tenant's documents nearest to it, and a chat model writes the answer from them.

There are about 400 small tenants, one big tenant (a retail chain) and one mid-size tenant whose developers call the service from scripts. The big tenant is about to move all its teams onto the service, which will bring about four times its traffic today. The mid-size tenant's scripts have sent floods before, and nothing has stopped them yet.

Every tenant's contract says the same thing: one API key may send up to 100 questions a second. Above that, the service is allowed to refuse questions with an HTTP 429.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 100 questions/s |
| Time to first token, p95 | 2,000 ms or less |
| p95 latency (the whole answer) | 10,000 ms or less |
| Availability | 99.9% or more |
| Errors | 1% or less |
| Budget | $15,000 per month |

## The traffic

- **A question from the big tenant** (20%). One API key: about 20 questions a second at peak.
- **A question from a small tenant** (70%). 400 tenants, each with its own API key, share this traffic evenly: about 70 questions a second in all, well under one a second each.
- **A question from the tenant whose script floods** (10%). One API key: about 10 questions a second on a normal day.

Every question takes the same path. It may pass edge cards (a Load Balancer, a Rate Limiter) and reaches your App Servers. The App Server calls three things in order and keeps its slot until the answer is done: a Model Inference card with the model type "Embeddings", a Vector Database, and a Model Inference card with the model type "Chat (text generation)". The prompt (the question plus the passages found) is about 1,500 tokens, 3,000 at p95. The answer is about 200 tokens, 400 at p95.

All the tenants' documents add up to 500 million vectors: set "Vectors (millions)" on your Vector Database cards to 500. In the simulator, a search takes longer the more vectors each shard holds and the more neighbours it asks for, and each shard runs 16 searches at a time. With 500 million vectors in one shard and 50 neighbours, a search takes about 45 ms.

In the simulator, every tenant's questions are spread over the same cards: you can't reserve a card for one tenant. What you can do per tenant is limit it. A Rate Limiter keeps a token bucket for each API key: its Limit is questions a second per key, and its Burst is how many a key may send at once. A question it refuses gets a 429. For the big tenant and the small ones that is an error; for the tenant whose script floods, a refusal above its contract is what should happen, so it counts as a success.

The time to first token counts from the moment the question arrives until a model sends its first token. Clients wait up to 15 s for an answer: set the web client's Timeout to 15,000 ms.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The big tenant moves all its teams over** and **A tenant's script floods the service**. Your design has to pass all three.
