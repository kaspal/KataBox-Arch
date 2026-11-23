# Right-to-Erasure Deletes

## The product

Your app keeps data about its users in three places: an Apache Iceberg table in the data lake (every event they ever made), a Snowflake warehouse (the tables analysts query) and a DynamoDB table (their profile). Privacy law gives each user the right to have that data erased. The law allows up to a month; your company promises 24 hours, and it has to keep that promise for every request, not on average.

A user clicks "Delete my data". The App Server publishes a deletion request to a queue and answers at once. A Worker takes the request, looks up where the user's data is, and deletes it from each store in turn: the lake table, the warehouse and the NoSQL store. A request is done only when the last store has deleted it.

**Time is scaled down.** A test lasts a minute or two, so the 24-hour deadline is played here as **30 seconds** from the click until the last store is done. The rule that carries over is the same at any scale: when deletions arrive faster than you finish them, the backlog grows, and once it is older than the deadline, every request behind it is late.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 20 deletion requests/s |
| Deadline | every request done within 30 s (p99), from the click until the last store has deleted it |
| Availability | 99.9% or more |
| Errors | 0.1% or less |
| Requests lost | none |
| Budget | $6,500 per month |

## The traffic

- **Delete my data** (100%). The web client calls your App Servers, which publish to a queue. A Worker consumes the request and makes three writes, one after another: to a Lakehouse Table, to a Data Warehouse and to a NoSQL Store. Each request is measured until the third write is done.

What each step costs here:
- The Worker spends its default service time of 100 ms finding the user's records, and holds its slot while it waits for each write.
- A Lakehouse Table write takes about 50 ms. A Lakehouse Table takes a thousand writes at once, so writes never wait for room there.
- A Data Warehouse write (a row-level DELETE) takes about 300 ms, and a Data Warehouse runs "Concurrency per instance" queries at once on each instance (8 by default). Snowflake bills an instance for the time it is running, and a warehouse that never gets 60 s of rest never suspends.
- A DynamoDB write takes a few milliseconds.

When a Worker's step fails, the queue tries the request again after 1, 2, 4, 8, 16 and then 32 seconds, up to its "Maximum retries" (3 by default). A request that fails every retry goes to a Dead-Letter Queue if one is wired to the queue; otherwise it is lost. A lost deletion is a promise broken: that user's data stays.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak, for a minute.

Two more tests are hidden, and you see only their names: **A privacy story goes viral** and **The lake table's rewrites slow down**. Your design has to pass all three.
