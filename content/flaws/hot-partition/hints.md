## 1

The store is mostly idle, yet requests queue. Look at how the load is split, not how much there is.

## 2

Every read of the same profile goes to the same partition. What takes reads of one key away from the store?

## 3

Put a Redis cache between the API and DynamoDB (cache-aside).
