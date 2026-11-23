A vector database stores **embeddings**: lists of numbers that a model produces for a piece of text, an image or a product, placed so that similar things sit close together. A query is an embedding too, and the database returns the `top_k` stored items nearest to it. That is how semantic search, "more like this" recommendations and retrieval for AI answers work.

Comparing a query with every vector would be too slow at millions of items, so these databases build an approximate index, usually **HNSW**, a graph that the search walks towards the closest points. It answers in milliseconds but may miss a true neighbour now and then. The size of each embedding (`dims`) sets how much memory the index needs; the simulator keeps it as a design note only.

Large collections are split into **shards**. A query goes to every shard, and the answer waits for the slowest one, so search time depends on how many vectors each shard holds and on `top_k`. Replicas are separate cards joined by Replicate edges and add read capacity; upserts go to the primary.

**How the simulator bills this.** Self-hosted flavours (pgvector, Qdrant, Milvus) cost $260 per node a month, an RDS-class machine. Pinecone has no node fee: it bills $16 per million read units and $4 per million write units (teaching prices; source to be confirmed).
