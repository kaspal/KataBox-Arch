# Ask the Docs (RAG)

## The product

You run the help assistant of a large developer platform. A user types a question, and the assistant answers it from the platform's own documentation, forum threads and support tickets: one billion short passages in all. Each question goes through three steps. An embedding model turns the question into a vector. A vector search finds the 100 passages nearest to it. A chat model reads the question and those passages and writes the answer, streamed word by word. Users judge the assistant by how soon the first words appear, so the time to first token matters more than the time to the last one.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 150 questions/s |
| Time to first token, p95 | 2,000 ms or less |
| p95 latency (the whole answer) | 8,000 ms or less |
| Availability | 99.9% or more |
| Errors | 1% or less |
| Budget | $20,000 per month |

## The traffic

- **Ask a question** (100%). The request may pass a load balancer and reaches your App Servers. The App Server calls three things in order and keeps its slot until the answer is done: a Model Inference card with the model type "Embeddings", a Vector Database, and a Model Inference card with the model type "Chat (text generation)". The prompt (the question plus the passages found) is about 1,200 tokens on average, 2,000 at p95. The answer is about 150 tokens on average, 300 at p95.

The index holds one billion vectors: set "Vectors (millions)" on your Vector Database cards to 1,000. Every search asks for the 100 nearest neighbours. In the simulator, a search takes longer the more vectors each shard holds and the more neighbours it asks for, and each shard runs 16 searches at a time. With 1,000 million vectors in one shard, a search takes about 70 ms.

The time to first token counts from the moment the question arrives until the chat model sends its first token, so it includes the embedding, the vector search and any wait in a queue on the way. Clients wait up to 15 s for an answer.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **Questions double for 30 s** and **A vector search node is lost on launch day**. Your design has to pass all three.
