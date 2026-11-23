## 1

Every tenant shares the same vector search and the same GPUs. What happens to a small tenant's question when one tenant sends five hundred questions a second? And what can you limit per tenant, when you can't give each tenant its own cards?

## 2

A Rate Limiter keeps a bucket per API key, so one Limit caps each tenant separately. It can't be lower than the big tenant's traffic after it moves its teams over (about 80 a second), or you refuse a paying tenant. The contract says 100. Then size the shared cards for the worst a limit still lets in: the normal traffic of everyone else plus one tenant at 100 a second, about 190 questions a second. The GPUs slow every answer as they fill, so leave them room.

## 3

Put a Rate Limiter, Redis token bucket, with a Limit of 100 and a Burst of 100, between the web client and an AWS ALB. Behind it, 4 Go App Servers with a concurrency of 512 per instance. Embed on NVIDIA Triton, 1 instance, model type "Embeddings", prompt speed 50,000 tokens/s. Search on one Qdrant Vector Database with 1 shard and 500 million vectors. Answer with vLLM on GPU, 10 instances with 128 batch slots per GPU. Set the web client's timeout to 15,000 ms.
