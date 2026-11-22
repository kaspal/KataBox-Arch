# Editorial

## Where the load goes

At 2,000 requests/s, about 1,700 are chunk uploads and about 300 are folder listings. 1,700 chunks of 128 KB is about 218 MB of uploads every second.

Without a check, every one of those chunks is a write to S3, a write to PostgreSQL and a message on SQS. S3 is billed at about $1 per million requests and SQS at about $0.40 per million. 1,700 a second for a month (2,592,000 s) is about 4.4 billion requests each: roughly $4,400 for S3 and $1,800 for SQS.

With Zipf 1.2, most uploads repeat a chunk you already have. An index of the chunks you have is small: at about 100 bytes an entry, all 2,000,000 fit in about 200 MB.

## The reference design

A Load Balancer (AWS ALB) sends devices to 6 Go instances of the Sync API. For each upload, the API first asks the chunk index, a 1 GB Redis Cache with a TTL of 86,400 s, used cache-aside. A hit means the chunk is already stored: on a normal day about 96% of uploads end there. A miss stores the chunk in S3, writes its record to PostgreSQL and publishes a change notice to an SQS Queue, which 4 Celery Notifiers (concurrency 32, 5 ms a notice) consume. Folder listings read PostgreSQL, which has a replica to share the reads. This costs about $1,706 per month, with S3 at about $159 and SQS at about $104. The p95 is about 26 ms, because fewer than 1 request in 20 goes past the index.

"A new laptop syncs everything" doubles the chunk uploads and makes them far less repetitive (Zipf 0.7), with a p95 limit of 1 s. Many more chunks miss the index and take the full path: the p95 rises to about 79 ms, and the bill at that rate to about $2,586. "The object store slows down" adds 150 ms to every S3 call for 30 seconds, from 15 seconds in, also with a 1 s limit. The few uploads that reach S3 take 150 ms longer, but they are under 5% of the traffic, so the p95 stays at about 26 ms.

## Why the first try breaks

A common first try is the same design without the chunk index: every upload is stored in S3, written to PostgreSQL and announced on SQS. Its p95 on a normal day is about 171 ms, but it costs about $7,364 per month, far over the $4,000 budget. S3 alone is about $4,464 and SQS about $1,825, both billed per request. It fails every test on cost; when the new laptop syncs, the bill reaches about $13,689.

It is also slow. Every upload holds a Sync API slot while S3 answers. The 6 instances have 384 slots, and on a normal day about 40% of them, some 150, are busy. When S3 adds 150 ms, 1,700 uploads a second hold about 255 more (1,700 × 0.15 s), more than the slots left. Requests queue at the API, and the p95 reaches about 1.6 s, past the 1 s limit.

Content-addressed deduplication, hashing the chunk and skipping it when you already have it, saves money and time at once.

## What the simulator leaves out

A real service names each chunk by a hash of its contents, so the same bytes always get the same name, whoever sends them. The simulator uses the request key as the chunk's identity. Conflicts, when two devices edit the same file at once, are not simulated, and neither is delta encoding, where a device sends only the bytes that changed instead of whole chunks.

## Things to try

- Lower the chunk index's TTL to 60 s and run "Normal day". Watch the S3 and SQS costs climb as fewer uploads hit.
- In the first try, double the Sync API instances and run "The object store slows down". See whether the extra slots are enough, and what the bill says.
