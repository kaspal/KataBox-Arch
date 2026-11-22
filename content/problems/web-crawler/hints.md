## 1

How often does your crawler fetch a page it already has, and what does each of those fetches cost?

## 2

De-duplicate before the expensive step. A quick lookup that says "already seen" is far cheaper than a trip to the web, and it saves a Worker thread and part of the web's rate limit.

## 3

Put a Redis Cache of seen URLs between the Worker and the web, checked before every fetch, with a long TTL such as a day. Give the Workers enough threads in total to keep going when pages take seconds instead of milliseconds.
