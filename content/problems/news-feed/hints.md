## 1

When someone opens their feed, where should it already be waiting, so the request has nothing left to assemble?

## 2

Use fan-out on write: when a post is published, copy it into each follower's feed ahead of time. Do that copying in Workers, off the request path, and keep the finished feeds in a cache so reads rarely reach the database.

## 3

Publish posts to a Stream with plenty of partitions, and let a few fan-out Workers consume it and write into a large Redis feed Cache (16 GB is comfortable). Read feeds cache-aside from that Cache, give PostgreSQL a replica, and leave the App Servers headroom well above a normal day.
