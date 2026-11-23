## 1

On a normal afternoon a handful of app instances is enough. On a big day the autoscaling group adds more. What else grows every time it adds one?

## 2

Each app slot that talks to PostgreSQL directly holds a connection of its own, so the database sees instances × concurrency connections, and it refuses everything past 1,700. A connection pooler in front of the database keeps a fixed pool of connections however many app instances there are, and in transaction mode it lends a connection only for the length of one transaction.

## 3

Put a PgBouncer Connection Pooler between the App Servers and PostgreSQL, with Mode left at "Per transaction" (the default) and Database connections at about 800. Keep the Spring Boot App Servers (Concurrency per instance 200) in an AWS Auto Scaling Group with Minimum instances 6, Maximum instances 24 and Target utilisation 60%. A Redis Cache (8 GB, TTL 600 s) answers most cart views, and PostgreSQL with 2 Shards takes the writes.
