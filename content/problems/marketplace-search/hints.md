## 1

A query always goes to the same shard of the search index. What happens to that shard when a handful of queries makes up most of the traffic?

## 2

Put a small query cache in front of the search engine, so the popular queries are answered from memory and the index sees mostly the long tail. For updates, let the database's change data capture feed a stream and have workers write each change into the index; then check how many changes the stream can move a second.

## 3

Wire a Redis Cache with a short TTL, around 30 s, between the Search API and Elasticsearch: 200,000 answers of 8 KB fit in the default memory. Wire PostgreSQL to a Kafka stream for the changes and let a couple of Celery Indexers consume it. A stream works one message per partition at a time, and each change takes about 30 ms to index, so raise the partitions well above the default 6, to around 24.
