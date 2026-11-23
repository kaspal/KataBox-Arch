## 1

A queue doesn't make the work disappear; it only lets it wait. The deadline holds as long as whatever piles up in the queue is cleared before it is 30 seconds old. So ask two questions: how many deletions can the Workers have in progress at once, and how many can the slowest store finish each second?

## 2

A deletion holds a Worker slot for all three writes, about half a second on a normal day. If the lake table's writes take 3 s longer, each deletion holds its slot seven times as long, and you need seven times the slots for the same traffic. Separately, the Data Warehouse finishes at most "Concurrency per instance" ÷ 0.3 s deletes a second per instance: count how many a surge brings. You don't need room for the whole surge, only enough that the backlog drains well inside the deadline.

## 3

Wire the web client to Go App Servers (2 instances, the default), which publish to an Amazon SQS Queue with "Maximum retries" set to 8, wired with Publish to a Dead-Letter Queue. 8 Celery Workers (concurrency 8, 64 slots) consume it and write an Apache Iceberg Lakehouse Table, a Snowflake Data Warehouse with 2 instances and a DynamoDB NoSQL Store.
