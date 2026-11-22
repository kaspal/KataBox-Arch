# Editorial

## Where the load goes

At 300 uploads/s of 2 MB each, about 600 MB/s of originals flow into S3. Storing them is cheap for the Upload API; the real cost is the resize. Each job holds a Worker slot for about 200 ms of decoding and resizing, plus about 50 ms to write the thumbnails to S3. 300 jobs a second × 0.25 s ≈ 75 slots busy all the time at peak.

With a queue in between, the Upload API answers fast and the jobs wait their turn. The queue only helps if the Workers have headroom over those 75 slots, and if losing some of them does not stop the work.

## The reference design

A Load Balancer (AWS ALB) sends uploads to the Upload API, 2 Go instances. It writes the original to S3 and publishes a job to an SQS Queue, left at its defaults. Two Celery Worker cards consume that queue: Thumbnailers A and Thumbnailers B, 12 instances each at the default concurrency of 8, so 96 slots per card and 192 in all. This costs about $3,786 per month, S3 and SQS requests included, with a p95 near 0.7 s on a normal day.

"Upload burst" sends 3 times the traffic for 10 seconds, starting at 20 seconds, and allows a p95 of 20 s. 900 jobs a second need about 225 slots, a little more than the 192 there are, so a short backlog builds and drains a few seconds after the burst. The p95 is about 4 s.

"A worker crashes" kills the first Worker card, Thumbnailers A, at 20 seconds for 30 seconds, in a 120-second run. It allows a p95 of 20 s and no lost messages. Thumbnailers B's 96 slots keep up with 300 jobs a second by themselves. The jobs A was holding when it died are not lost: SQS leases each message to a Worker, and when the 30-second visibility timeout runs out without a reply, the message goes back on the queue for B to take. The p95 stays near 0.7 s.

## Why the first try breaks

A common first try is one Worker card, Thumbnailers, with 16 instances: 128 slots, more than the 75 the peak needs, for about $3,226 per month. It passes "Normal day", and the burst too, with a p95 of about 11 s, still under the limit.

The crash is where it fails. That one card is the first Worker card, so the chaos kills all of the thumbnailing for 30 seconds. No message is lost, because the lease brings every job back, but about 9,000 jobs pile up in the queue. When the card returns it has only about 50 spare slots over the incoming load to clear them, which takes well over half a minute. The p95 reaches about 28 s, past the 20 s limit.

The lesson: leases make a crash cost time instead of data. Split the Workers so one crash cannot stop the work, and give each part enough headroom to carry the load alone.

## Things to try

- Delete Thumbnailers B and run "A worker crashes". Then put it back with 6 instances and see whether half a card is enough.
- Lower the queue's visibility timeout to 10 s and run the crash test. A's unfinished jobs come back to B sooner; on the first try it changes little, because no Worker is left to take them.
- Shrink both cards to 8 instances and watch the p95 during "Upload burst".
