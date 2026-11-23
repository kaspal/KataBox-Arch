# Editorial

## Where the load goes

Every question makes three calls in a row: the embedding, the vector search and the chat model. The chat model takes the most time and the most money, but it scales the usual way: more GPUs, more batch slots. The step that is easy to forget is the vector search, because a single search looks cheap.

It is cheap in time, not in slots. In the simulator a search costs the card's service time of 2 ms, times 1 + log2 of the vectors per shard in millions, times √(neighbours ÷ 10). With 1,000 million vectors in one shard and 100 neighbours, that is 2 × (1 + log2 1,000) × √10 ≈ 2 × 11.0 × 3.16 ≈ 69 ms. A shard runs 16 searches at a time, so it finishes at most about 16 ÷ 0.069 ≈ 230 searches a second, and fewer once the slow tail is counted. A normal day sends 150. One shard is already about three quarters busy.

Splitting the index into shards barely shortens one search: with 2 shards each one holds 500 million vectors, but every shard answers and the slowest sets the time, so a search still takes about 70 ms. What shards add is slots: 2 shards run 32 searches at a time. A second node wired with Replicate adds slots too, and it is the only thing that keeps searches answered when a node is lost.

The hidden tests check both. "Questions double for 30 s" sends 300 questions a second from second 15 to second 45. "A vector search node is lost on launch day" runs the whole test at 1.5 times the peak (225 questions a second) and kills the first Vector Database card 20 seconds in, for 30 seconds. While it is down, whatever survives carries all 225 searches a second.

## The reference design

An AWS ALB sends the questions to 4 Go App Servers with a concurrency of 512 per instance. They hold their slot for the whole answer, about 5 s at p95, so they need many slots, not much CPU. The embedding runs on NVIDIA Triton, 2 instances with the model type "Embeddings" and a prompt speed of 50,000 tokens/s. The search runs on a Qdrant Vector Database with 2 shards and 1,000 million vectors, wired with Replicate to a second Qdrant with the same settings, and the App Servers query both. The answer comes from vLLM on GPU, 12 instances with 128 batch slots per GPU. The web client's timeout is 15,000 ms, so a long answer isn't cut off.

It costs about $11,621 per month: $8,808 for the 12 GPUs of the chat model, $1,468 for the 2 embedding GPUs, $520 for each Qdrant (2 shards each), $280 for the App Servers and $25 for the ALB. The chat model is three quarters of the bill; the whole vector search is under a tenth.

On a normal day, each Qdrant is about 20% busy, the chat GPUs about 27% and the App Servers about 21%. The time to first token is about 0.7 s at p95, the whole answer about 4,957 ms at p95, and there are no errors.

When the questions double, the two Qdrant cards have 64 search slots between them and stay far from full. The time to first token stays at about 0.7 s at p95, the whole answer takes about 5,924 ms at p95 (the GPUs share their speed among more answers), and 0.007% of the questions fail.

When a node is lost on launch day, about 1.66% of the questions fail, all in the 2 seconds or so before the health checks notice the dead node. The test allows 3% errors and 97% availability. After that, every search goes to the surviving Qdrant, whose 2 shards run 32 searches at a time: 225 searches a second keep it a little over half busy. The time to first token stays at about 0.7 s at p95 and the whole answer at about 5,366 ms.

Both halves are needed. One Qdrant with 2 shards and no replica gets through the surge but fails about 49.3% of the questions when it is lost. Two Qdrant cards with 1 shard each get through the surge too, but when one is lost the survivor has 16 slots for 225 searches a second: it overflows, and about 7.6% of the questions fail.

For the cost challenge, the proof search found nothing below 90% of the reference's cost ($10,487): the GPUs set the bill, and they have little to give. This problem has no cost challenge.

## Why the first try breaks

A common first try is the same design with one Qdrant of 1 shard and no replica. It costs about $10,841 and passes "A normal day": its one shard is about 78% busy, and the p95 is about 4,957 ms with no errors.

When the questions double, 300 searches a second meet a shard that finishes about 230 at best. Its queue fills, and questions that find it full are refused: about 23.0% of the questions fail, almost all during the 30 seconds of the surge. The bigger and more expensive chat model never sees them.

When the node is lost on launch day, there is nothing else to ask. At 1.5 times the peak the lone shard is already over its limit and fails a few questions before the kill; then, for 30 seconds, every question fails. About 57.1% of the questions fail in all.

The point is this: in a retrieval pipeline every question passes through the vector search, so it has to be sized like any other server. Shards and replicas both add search slots, but only a replica keeps the index answering when a node is lost, and the survivor must have the slots to carry the full load alone.

## What the simulator leaves out

It doesn't judge answers: whether the 100 passages found are the right ones (recall) and whether the answer is correct are not simulated, and nothing here says how many neighbours or which model gives good answers. The search time is a modelling choice (a formula over vectors per shard, neighbours and shards), not a measurement of Qdrant; vector dimensions, memory, index build time and index updates are not simulated. The simulator draws one prompt size per question and charges it to both model calls, so the embedding card's prompt speed is set high to stand for a short question. The time to first token is measured at each model call, so its p95 mixes the embedding calls (short) with the chat calls. The streaming of tokens over the network, GPU memory and the KV cache are not simulated. The GPU price is a g5.xlarge-class figure, and the Vector Database price is a teaching figure still marked as needing a source.

## Things to try

- Set the Qdrant cards to 1 shard each and run "A vector search node is lost on launch day". Watch the surviving card's queue.
- Remove the replica and give the one Qdrant 4 shards. Which hidden test still fails, and why don't more shards help it?
- Lower the chat GPUs to 8 and run "Questions double for 30 s". Compare the time to first token with the whole answer's p95.
