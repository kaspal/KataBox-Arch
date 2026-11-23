## 1

The chat model is the expensive part, but it isn't the only thing every question waits on. How many searches a second can one shard of your Vector Database run, and how close to that is a normal day? What happens to a question when the only vector node it can ask is gone?

## 2

A shard runs 16 searches at a time, and a search over a billion vectors takes about 70 ms, so one shard tops out a little over 200 searches a second. More shards add search slots. A second node wired with Replicate adds slots too, and keeps answering when the first one is lost. But when one node is lost, the survivor carries every search alone, so it needs enough slots for that.

## 3

Put an AWS ALB in front of 4 Go App Servers with a concurrency of 512 per instance (each one holds its slot for the whole answer). Embed on NVIDIA Triton, 2 instances, model type "Embeddings", prompt speed 50,000 tokens/s. Search on a Qdrant Vector Database with 2 shards and 1,000 million vectors, wired with Replicate to a second Qdrant with the same settings, and query both from the App Servers. Answer with vLLM on GPU, 12 instances with 128 batch slots per GPU. Set the web client's timeout to 15,000 ms.
