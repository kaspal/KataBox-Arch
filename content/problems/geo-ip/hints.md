## 1

The whole IP table is small, and nobody writes to it while the service runs. What does that make cheap that would be hard with data people change? And what happens to your callers if the one place that holds it goes away?

## 2

Put a full copy of the table in memory in two Regions, each with its own Load Balancer and App Servers, and let DNS with health checks send callers to the Regions that answer. When one Region is lost, the other gets everyone, so size each Region to carry the whole peak, and the busiest hour on top of it, on its own.

## 3

Put Route 53 with a TTL of 30 s in front of two Regions, us-east and eu-west. In each one, place an AWS ALB, Go App Servers with 6 instances and a Redis Cache with 2 GB of memory and a TTL of 0, so the data never expires. Each App Server reads from the Cache in its own Region. 6 instances keep each Region's apps under a third busy on a normal day, so either Region can take all the traffic when the other goes dark.
