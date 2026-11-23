# Log Search Over a Year

## The product

When something breaks, your engineers search the application logs. The logs land in an Apache Iceberg table on object storage, and the table keeps a year of them: about 100 MB of compressed columns a day, 36.5 GB in all. Engineers query it through Google BigQuery, which reads the table where it lies and bills on demand: $6.25 for every TB it scans. Almost every search asks about the last 24 hours. Now and then someone asks about the whole year.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 2 queries/s |
| p95 latency | 2 s or less |
| Availability | 99.9% or more |
| Errors | 1% or less |
| Budget | $35,000 per month |

The budget is checked against the cost of "A normal working day", as if its peak lasted all month.

## The traffic

- **Search the last 24 hours of logs** (99%). The engineers' Client calls your App Server, which sends the query to a Data Warehouse. The warehouse reads the Lakehouse Table it is wired to with a Query edge. The query filters on the log's timestamp, so it needs one day of the 365 in the table: 0.1 GB.
- **Query the whole year of logs** (1%). The same path. It needs all 36.5 GB, however the table is laid out.

A query's time in the warehouse is its Query overhead (300 ms) + the data it scans ÷ its Scan rate per query (1 GB/s) + 20 ms for each file it opens, 8 files at a time. While it runs, a query holds one of the warehouse's slots (Instances × Concurrency per instance); the others wait in line. The search page waits up to 60 s, so set the Client's Timeout to 60,000 ms. After that the search fails.

## The tests

One test is visible. You can run it as often as you like:

- **A normal working day**: the traffic above, at peak, for 5 minutes.

Two more tests are hidden, and you see only their names: **An audit runs fifteen times the usual year-long queries** and **An incident: everyone searches the last day at once**. During the audit, year-long queries are much more than 5% of the traffic, and they take well over half a minute by themselves, so that test allows a p95 of 45 s. Your design has to pass all three.
