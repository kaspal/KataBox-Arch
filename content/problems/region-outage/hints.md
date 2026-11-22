## 1

Two Availability Zones protect you from a data centre failing. What protects you when every zone in a region fails at once, or when the region is fine but cut off from your customers?

## 2

Run the service in more than one Region, each with its own Load Balancer, App Servers and a copy of the database, and let DNS spread customers over them. Reads can stay in their own Region; writes still have to reach the primary, so check what a cross-region write costs against the p95 target.

## 3

Put Route 53 in front of three Regions. In each one, place an AWS ALB, a Go App Server with 2 instances and a PostgreSQL member: the primary in one Region and a replica in each of the other two, wired with Replicate edges. Each App Server talks to the database in its own Region. A write from another Region crosses twice, about 150 ms in all, which still fits under 250 ms. When the primary's Region is lost, a replica is promoted.
