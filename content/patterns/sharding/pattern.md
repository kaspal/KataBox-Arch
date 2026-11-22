Each record has a shard key, such as a user id. A function of that key, usually a hash, picks which shard owns the record, and every read or write for that key goes there. Each shard is a full database with its own servers, so adding shards adds capacity.

The choice of key decides everything. A good key spreads requests evenly and keeps the data one request needs on one shard. A poor one sends a celebrity's traffic, or today's date, to a single shard that saturates while the rest sit idle.

Work that spans many keys gets harder: a query across all users has to ask every shard and merge the answers. Many NoSQL stores shard for you by partition key; a SQL database usually needs the app or a routing layer to know where each key lives.
