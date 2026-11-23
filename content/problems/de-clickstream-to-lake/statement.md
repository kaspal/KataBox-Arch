# Clickstream into a Lakehouse

## The product

You run a product-analytics service. Your customers put a small script on their websites, and every browser sends you its clicks in batches of about 20. You keep every click in one lake table on object storage, and your customers look at charts of their traffic: each time a chart refreshes, it runs one query over the last day of clicks. The table holds a year of clicks, about 730 GB of compressed columnar files (about 2 GB a day). Clicks may reach the table a few seconds late, but none may be lost, and a chart must not hang.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 200 requests/s |
| p95 latency | 4 s or less |
| Availability | 99.9% or more |
| Errors | 1% or less |
| Lost click batches | 0 |
| Budget | $30,000 per month |

The p95 counts every request: a click batch until it is written to the table, and a chart refresh until it gets its answer.

## The traffic

- **Send a batch of click events** (90%). The request may pass a load balancer and reaches your App Servers, which publish it to a Stream. A Stream Processor consumes it and writes it to the Lakehouse Table. The batch counts as done when it is in the table.
- **Refresh a dashboard chart** (10%). The request may pass a load balancer and reaches your App Servers, which send the query to a Data Warehouse. The warehouse reads the Lakehouse Table in place (wire it to the table with a Query edge) and scans the last day of clicks. The table changes every second, so the same chart never gets a cached answer.

How long a scan takes depends on the table:

- **Partitioning.** Without partitions, a query reads the whole year. By day, it reads only the day it needs.
- **Files.** The Stream Processor commits small files, about 2 MB each. Without compaction, the table keeps them. With compaction, they are rewritten into files of the table's target file size. Each file costs the query time to open, whatever its size.

A dashboard gives up after 5 s (the Client's default timeout), and that refresh counts as failed. An App Server keeps its slot while it waits for the warehouse.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A launch day brings 5× the traffic** and **The stream processor is down for 20 s**. Your design has to pass all three.
