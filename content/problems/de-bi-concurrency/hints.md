## 1

Count the warehouse's slots. 12 queries a second that take about 0.4 s each keep about 5 queries running at once, and one cluster runs 8. What happens when four times as many arrive? And what does a query that holds a slot for a whole minute do to the analysts queuing behind it?

## 2

Clusters multiply the slots: each one adds 8. Size them for the 9 a.m. rush, not for a normal hour. Exploration queries hold their slots for about 60 s, so on a shared warehouse a burst of them fills every slot and the analysts wait in the queue. Give the notebooks a warehouse of their own: it auto-suspends, so it costs almost nothing while nobody explores.

## 3

Set the Client's Timeout to 120,000 ms. Wire the App Server (2 Go instances) to a Snowflake Data Warehouse with 3 instances, and the Microservice (2 instances) to a second Snowflake Data Warehouse with 5 instances. Keep Concurrency per instance at 8 and Auto-suspend after at 60 s on both.
