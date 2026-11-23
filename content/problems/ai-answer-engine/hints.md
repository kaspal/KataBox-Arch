## 1

Count what reaches the chat model. At 200 questions a second, how many requests a minute is that, and how close is it to the 16,000 limit? Then think about a test with two or three times the traffic that lasts longer than a minute. Which questions does the model really have to answer?

## 2

Popular questions arrive word for word, and an answer may be reused for 10 minutes. A cache keyed on the exact question, in front of the whole pipeline, answers every repeat inside that window without a search, an embedding or a model call. The hotter the questions get, the more it answers. Matching by meaning only ever catches the quarter of questions that are near-repeats, however hot the topic is.

## 3

Use an AWS ALB in front of 8 Go App Servers with a Concurrency per instance of 512 (each holds its slot for the whole answer). Read a Redis Cache first: Mode "Exact keys", its default 2 GB, and a TTL of 600 s. On a miss, query an Elasticsearch Search Index with its defaults, embed on NVIDIA Triton with Model type "Embeddings" and a Prompt speed of 50,000 tokens/s, search a Qdrant Vector Database with 2 shards and 100 million vectors, and answer with the Claude API at the account's limits (16,000 requests and 30,000,000 tokens per minute).
