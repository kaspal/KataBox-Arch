## 1

Every transfer lands on one database before anything else happens. What does the app answer while that database is gone, and is there anything that could take its place?

## 2

Give the ledger a copy that can be promoted when the primary dies. Then think about the balances: they come from the ledger's change stream, so whichever copy is the primary has to be able to publish to that stream.

## 3

Wire a second PostgreSQL to the ledger with a Replicate edge, and give both of them a change data capture edge to the same Kafka stream, so the promoted replica keeps publishing. Raise the stream well above its default 6 partitions, to around 32, and let a few Celery projectors with a short service time write the balances into DynamoDB.
