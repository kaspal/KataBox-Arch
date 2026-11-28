## 1

Count the GB the warehouse has to scan each second. 12 queries a second at 0.1 GB each, minus the few the Result cache answers, are about 1.1 GB/s, and one cluster scans 1 GB/s, shared among the queries running on it. What happens when four times as many arrive? And what does a query that scans 60 GB do to the analysts queuing behind it?

## 2

Clusters multiply the scan rate: each one adds 1 GB/s and 8 slots. Size them for the 9 a.m. rush, not for a normal hour. An exploration query scans 60 GB, so on a shared warehouse even one of them holds the whole scan rate for seconds, and a burst of them leaves the analysts waiting in the queue. Give the notebooks a warehouse of their own, with enough clusters to keep up with the exploration: it auto-suspends, so it costs almost nothing while nobody explores.

## 3

Set the Client's Timeout to 120,000 ms. Wire the App Server (2 Go instances) to a Snowflake Data Warehouse with 5 instances, and the Microservice (2 instances) to a second Snowflake Data Warehouse with 32 instances. Keep Concurrency per instance at 8 and Auto-suspend after at 60 s on both.
