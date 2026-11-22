## 1

Where does 95% of the traffic go, and does every single click really need to reach the database?

## 2

Put a cache in front of the database (cache-aside) so popular links rarely touch it. Then ask yourself what happens to creates and reads when your only database stops.

## 3

Add a Redis Cache of a few GB (8 GB holds about 8 million 1 KB entries) and a PostgreSQL replica joined to the primary by a Replicate edge, with a short failover time. Give the app tier plenty of headroom above the peak, not just enough for a normal day.
