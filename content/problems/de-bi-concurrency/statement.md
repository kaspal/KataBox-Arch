# Analysts at 9 a.m.

## The product

Your company has about 300 analysts, and they all query the same Snowflake data warehouse through a BI tool. They don't open a shared dashboard: each one writes their own ad-hoc questions ("revenue by region for this campaign", "churn for customers who signed up in March"), so almost no two queries are the same. Everyone starts at 9 a.m., and the warehouse has to keep up.

The data science team works on the same warehouse from notebooks. Their exploration queries are different: each one scans the full history. On a normal day they are rare, but when the team starts a new investigation, many arrive at once. Analysts must not wait because a notebook is busy.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 12 queries/s |
| p95 latency | 2 s or less |
| Availability | 99.9% or more |
| Errors | 1% or less |
| Budget | $11,500 per month, judged on a normal working hour |

## The traffic

- **Run an ad-hoc query from the BI tool** (99.98%). The query goes to the BI tool's server (an App Server), which runs it on a Data Warehouse. Each query scans 0.1 GB. Only 5% repeat an earlier query, so the warehouse's Result cache answers few of them.
- **Run a heavy exploration query from a notebook** (0.02%, about one every 7 minutes). The query goes to the notebook gateway (a Microservice), which runs it on a Data Warehouse. Each query scans 60 GB.

The analysts and the notebooks wait up to 120 s for an answer: set the Client's Timeout to 120,000 ms.

## The warehouse

The company runs Snowflake at size X-Small. On the Data Warehouse card:

- **Instances** are the warehouse's clusters. Each cluster runs **Concurrency per instance** = 8 queries at once; more queries wait in the warehouse's queue, and when the queue is full, the warehouse refuses them.
- A query takes the **Query overhead** (300 ms) plus its scan at the **Scan rate per query** (1 GB/s): about 0.4 s for an ad-hoc query and about 60 s for an exploration query. The clusters share their scan rate among the queries running at once: a warehouse scans at most clusters × 1 GB/s in total, and a single query never faster than 1 GB/s.
- Concurrency, scan rate and overhead come with the size, so keep them at 8, 1 GB/s and 300 ms. To run more queries at once, or scan more GB a second, add clusters.
- Each cluster costs $1,460 a month if it runs all the time. The warehouse is billed only while it works: after **Auto-suspend after** (60 s) with no query, it stops billing until the next query arrives.

## The tests

The simulator scales time down: a rush that lasts half an hour in real life lasts 40 s here, and an afternoon of exploration lasts 2 minutes.

One test is visible. You can run it as often as you like:

- **A normal working hour**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **9 a.m.: four times the queries for 40 s** and **The data science team starts exploring the full history**. Your design has to pass all three.
