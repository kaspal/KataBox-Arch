# Editorial

## Where the load goes

Every question, whichever tenant sends it, makes the same three calls: the embedding, the vector search and the chat model. The cards behind them are shared by all the tenants, so their capacity is shared too. Nothing in a queue knows whose question it is.

The vector search costs the card's service time of 2 ms, times 1 + log2 of the vectors per shard in millions, times √(neighbours ÷ 10). With 500 million vectors in one shard and 50 neighbours, that is 2 × (1 + log2 500) × √5 ≈ 2 × 10.0 × 2.24 ≈ 45 ms. A shard runs 16 searches at a time, so it finishes at most about 16 ÷ 0.045 ≈ 360 searches a second. A normal day sends 100.

The chat model is the bigger shared resource. On vLLM an answer waits 200 ms, reads the prompt at 5,000 tokens a second (about 0.3 s for 1,500 tokens), then writes 200 tokens at 80 tokens a second (about 2.5 s) when the GPU is idle. As a GPU's batch slots fill, every answer on it slows down, up to half speed when it is full. So a tenant that sends more questions doesn't only take slots: it makes everyone else's answers slower.

The two hidden tests pull in opposite directions. In "The big tenant moves all its teams over", the big tenant sends 4 times its traffic for the whole test: 80 questions a second instead of 20, about 160 in all. Those are paying customers, so none may be refused. In "A tenant's script floods the service", the mid-size tenant sends 50 times its traffic for the whole test: 500 questions a second, about 590 in all.

A limit per tenant has to sit between the two: above 80 a second, so the big tenant gets through, and the contract says 100. But a limit of 100 still lets the flooding tenant send 100 a second. The shared cards must carry the normal traffic of everyone else plus one tenant at its limit: 20 + 70 + 100 = 190 questions a second. That number, not the 100 of a normal day, sets the size of the GPUs.

## The reference design

The web client (timeout 15,000 ms) sends every question through a Rate Limiter, Redis token bucket, with a Limit of 100 and a Burst of 100, and then an AWS ALB. The limiter keeps a bucket for each API key, so each of the 402 tenants gets its own 100 a second. Behind the ALB, 4 Go App Servers with a concurrency of 512 per instance hold their slot for the whole answer. The embedding runs on NVIDIA Triton, 1 instance with the model type "Embeddings" and a prompt speed of 50,000 tokens/s. The search runs on one Qdrant Vector Database with 1 shard and 500 million vectors. The answer comes from vLLM on GPU, 10 instances with 128 batch slots per GPU.

It costs about $8,669 per month: $7,340 for the 10 chat GPUs, $734 for the embedding GPU, $280 for the App Servers, $260 for Qdrant, $30 for the limiter and $25 for the ALB.

On a normal day the chat GPUs are about 28% busy, Qdrant about 33% and the App Servers about 18%. The whole answer takes about 6,541 ms at p95, the time to first token stays under 0.9 s at p95, and 0.017% of the questions fail.

When the big tenant moves its teams over, its 80 questions a second stay under its limit of 100, so nothing is refused. The GPUs are about 50% busy and Qdrant about 54%. The whole answer takes about 7,366 ms at p95, and no question fails.

When the script floods, the limiter lets 100 of its 500 questions a second through and refuses the other 400 with a 429, which counts as a success for that tenant. The shared cards see about 190 questions a second: the GPUs are about 66% busy, Qdrant about 64% and the App Servers about 43%. The whole answer takes about 8,295 ms at p95, and 0.017% of the questions fail.

Each part of the design is needed, and the variants show it:

- With the same design and only 8 chat GPUs, it passes the onboarding (p95 about 8,461 ms) but not the flood: the 100 a second the limit still lets in fill the GPUs, every answer slows down, and the p95 reaches about 12,573 ms.
- With a Limit of 60, the flood passes, but the onboarding fails: about 20 of the big tenant's 80 questions a second are refused, about 11.4% of all the questions.
- With a Limit of 150, the onboarding passes, but the flood lets in 150 a second and the p95 reaches about 12,825 ms.
- With no limiter at all, money can't buy a pass within the budget: 24 chat GPUs and 2 shards ($19,175) still fail about 41.0% of the questions in the flood, and it takes 30 GPUs, 3 shards and 8 App Server instances to pass it, at about $24,119.

For the cost challenge, the proof search found nothing below 90% of the reference's cost ($7,795): the chat GPUs are most of the bill, and the flood needs them. This problem has no cost challenge.

## Why the first try breaks

A common first try has no Rate Limiter and 8 chat GPUs, sized for the onboarding with some room to spare. It costs about $7,171. It passes "A normal day" (p95 about 6,805 ms, 0.017% errors) and "The big tenant moves all its teams over" (p95 about 8,461 ms, 0.094% errors, the GPUs about 71% busy).

When the script floods, all 590 questions a second reach the shared cards. In the first seconds Qdrant gets 590 searches a second against the 360 or so its shard can finish: its queue fills, and about 21,000 searches are dropped over the test. The questions that get past it fill the GPUs, which run about 97% busy, so every answer slows down until answers take longer than the clients' 15 seconds. The App Servers, holding a slot for each of those slow answers, are about 92% busy. About 84.2% of all the questions fail.

The other tenants did nothing wrong, and they fail too. Their questions are the same as the script's (the same path, the same prompt and answer sizes), and they wait in the same queues on the same cards, so they fail about as often. Run the same test with the flooding tenant's traffic taken out and this design has no errors at all.

The point is this: in a shared service, a tenant's traffic is everyone's latency. When you can't give each tenant its own machines, give each one a quota, set it from what you promise your biggest tenant, and size the shared capacity for the normal traffic plus one tenant at that quota.

## What the simulator leaves out

It doesn't judge answers: whether the 50 passages found are the right ones and whether the answer is correct are not simulated. It doesn't keep tenants' data apart either: in a real service each search must only see its own tenant's documents (a filter or a separate index per tenant), and a leak between tenants is a security failure the simulator can't see. Every request type is spread over every matching card, so the real isolation moves (a separate index or shard for the big tenant, separate GPU pools for tenant tiers, a fair or priority queue per tenant in the model server) can't be built here; a quota per API key is the isolation the simulator offers. One Limit applies to every key, while real plans often have tiers. The limiter counts questions, not tokens. The search time is a modelling choice (a formula over vectors per shard, neighbours and shards), not a measurement of Qdrant; vector dimensions, memory and index updates are not simulated. The simulator draws one prompt size per question and charges it to both model calls, so the embedding card's prompt speed is set high to stand for a short question. The time to first token is measured at each model call, so its p95 mixes the embedding calls with the chat calls. Streaming over the network, GPU memory and the KV cache are not simulated. The GPU price is a g5.xlarge-class figure, and the Vector Database price is a teaching figure still marked as needing a source.

## Things to try

- Remove the Rate Limiter and run "A tenant's script floods the service". Watch Qdrant's drops in the first seconds, then the GPUs' utilisation.
- Set the Limit to 60, then to 150, and run both hidden tests each time. Which one fails, and why?
- Keep the limiter and lower the chat GPUs to 8. The onboarding still passes; work out why the flood doesn't, from 190 questions a second and the slowdown as the GPUs fill.
