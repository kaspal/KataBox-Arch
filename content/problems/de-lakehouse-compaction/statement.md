# Interactive Queries on a Streaming Lake Table

## The product

Your product's analysts explore user behaviour in SQL notebooks: they write a query about today's events, look at the answer, and write the next one. They stay in the flow only if an answer comes back in a few seconds. The events reach them through a streaming pipeline: collectors send batches of product events to your API, a stream processor writes them into a lake table every few seconds, and a SQL warehouse reads that table in place. Writing every few seconds keeps the data fresh, but it leaves the table full of small files, and a query has to open every file it reads.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 10 requests/s |
| p95 latency | 8,000 ms or less |
| Availability | 99.9% or more |
| Errors | 1% or less |
| Lost events | 0 |
| Budget | $71,700 per month |

The p95 is taken over every request. Most requests are queries, so in practice it is the analysts' wait.

## The traffic

- **Send a batch of product events** (16%). A batch of about 128 KB may pass a load balancer and reaches your App Servers, which publish it to a stream. A stream processor consumes it and writes it into the lake table. Its latency is measured until the write is done.
- **Run an interactive query over today** (83.5%). An App Server sends the query to the warehouse, which reads the lake table. The table keeps 30 days of events, about 2 GB a day in the columns a query reads, so about 60 GB in all. A query filters on today: one day of the 30. One query in five repeats an earlier one exactly.
- **Run an interactive query over a sale day** (0.5%). The same kind of query, looking back at the last sale day: that day holds 20 GB of events, ten times a normal day, and the table about 78 GB.

The stream processor commits a file every few seconds for each of its writer tasks, so new data lands in files of about 4 MB. On a sale day the pipeline runs ten times the writer tasks to keep up, so the files stay about 4 MB and there are ten times as many. Opening a file in object storage takes about 20 ms, and a query opens up to 8 files at a time. A query's other costs are the warehouse's query overhead and the data it reads at its scan rate.

The notebooks wait up to 60 s for an answer: set the Client's timeout to 60,000 ms.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A sale day: ten times the events, and analysts look at today** and **Month-end review: four times the queries**. Your design has to pass all three.
