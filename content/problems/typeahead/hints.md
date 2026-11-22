## 1

Out of 200,000 prefixes, how many different ones do people actually type most of the time, and where in the index do they land?

## 2

Cache the answers for popular prefixes in front of the index. Adding shards spreads different prefixes, but one hot prefix always lands on the same shard.

## 3

Put a Redis Cache with a TTL of about an hour between the App Servers and the Search index, and keep the index behind it with a few shards. A handful of App Servers is enough once most keystrokes end at the cache.
