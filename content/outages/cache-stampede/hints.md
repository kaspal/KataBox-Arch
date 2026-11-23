## 1

The database was fine before the flush, and the cache does refill. So why are there so many errors in the first seconds, and why do they come back about 30 seconds later?

## 2

While the first miss for a product is still being answered, every other request for that product also misses and sends the same 200 ms query. A popular product sends hundreds of copies of the same work. Only one of them needs to reach the database.

## 3

On the Redis cache, turn on the stampede guard (coalesce misses). Nothing else needs to change.
