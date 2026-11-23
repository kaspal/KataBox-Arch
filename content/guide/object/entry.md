**Object storage** keeps files, called objects, in buckets, each under a key such as `pastes/abc123`. You write an object whole and read it back whole. There are no tables and no queries: you fetch by key, or list the keys under a prefix. In return you get practically unlimited space, very high durability, and nothing to run yourself.

It is the right home for anything large: images, videos, uploads, backups, long text bodies. The usual pattern keeps the bytes in object storage and a small record in a database that points to them. A CDN can serve the objects straight from the bucket, so popular files rarely touch your servers at all.

In the simulator an Object Storage card serves a very large number of requests at once, and its service time does not grow with the object's size, unlike a SQL or NoSQL card. Each request is slower than a cache or database read, though, and it is billed per request, so a cache or CDN in front still pays off for hot objects.

**How the simulator bills this.** Amazon S3 and Google Cloud Storage bill each request: a read (GET) costs $0.40 per million and a write (PUT) $5.00 per million, so writes usually drive the bill. Storage space and data transfer are not billed (list price, us-east-1, read 2026-11-22).
