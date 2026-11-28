## 1

At 9 a.m. the traffic grows tenfold, but the dashboard doesn't: it is still the same 400 tiles. How many of those loads really need the warehouse to scan anything, and what holds a warehouse slot while they wait?

## 2

The Result cache already answers repeats, but every query still goes to the warehouse, and 40% of them scan 0.5 GB, which one cluster can do only twice a second. A cache in front, keyed by tile, turns the rush into a few hundred queries at most, whatever the traffic. When the cache starts empty, many people ask for the same missing tile at the same moment: make them wait for one query instead of each starting their own.

## 3

Put an AWS ALB in front of 2 Go App Servers. Wire them to a Redis Cache with a TTL of 600 s and "Coalesce misses (stampede guard)" on, and to one Snowflake Data Warehouse with 5 clusters (Instances 5) and its other defaults (Result cache on): the cold start at 9 a.m. needs that much scanning for a few seconds.
