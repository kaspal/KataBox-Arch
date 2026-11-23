# Editorial

## Where the load goes

At 50 requests a second, about 39 are normal files, 1 is a giant file and 10 are status checks. The upload is cheap: the App Server writes the file to S3, publishes a message and answers. The work is in the Workers, which hold a slot while they parse the file (100 ms), read it from S3 (50 ms) and write its rows to PostgreSQL (about 25 ms for 256 KB). With hops and slow tails, a normal file holds a slot for about 0.2 s, and a giant one for 25 times that, about 5 s. A normal day keeps about 39 × 0.2 + 1 × 5 ≈ 13 slots busy.

The hidden tests change both halves. "A partner sends a month of giant files" multiplies the giant files by 20, to 20 a second for the whole minute: about 39 × 0.2 + 20 × 5 ≈ 108 busy slots, eight times a normal day. "The database's primary dies" kills the PostgreSQL primary 20 seconds into a 2-minute run, and it never comes back. Every write fails until a replica is promoted, about 30 seconds later, and every file that arrives in that time has to wait.

## The reference design

The partner server calls 2 Go App Servers. They write each file to Amazon S3, publish a message to an Amazon SQS Queue and read PostgreSQL for status checks. The queue has "Maximum retries" set to 8: the waits add up to 1 + 2 + 4 + 8 + 16 + 32 + 32 + 32 = 127 seconds, far longer than a failover. A Dead-Letter Queue is wired from the queue with Publish. 16 Celery Workers with a concurrency of 8 (128 slots) consume the queue, read the file from S3 and write PostgreSQL. PostgreSQL is wired with Replicate to a second PostgreSQL, with the default failover time of 30 s. This costs about $2,652 per month: $1,120 for the Workers, $566 for S3, $126 for SQS, $140 for the App Servers and $350 for each PostgreSQL. A normal day has a p95 of about 489 ms, with no errors and no lost files.

During the giant files, the Workers are about 87% busy and the p95 is about 7.2 s, mostly the giant files' own work. The month costs more at that rate ($3,006), since S3 and SQS charge per request.

"The database's primary dies" allows a p95 of 45 s, 2% errors and 98% availability. For about 2 seconds, until the health checks notice, status reads still go to the dead primary and some fail; then the replica serves them. Writes fail until the replica is promoted, about 30 seconds after the crash. Each failed file goes back to the queue and is tried again: the first ones to fail, at 20 s, get through on their retry at 51 s, about 31 s late. The p95 is about 31.9 s, with 0.149% errors and no lost files. The Dead-Letter Queue stays empty: remove it and run the test again, and still nothing is lost.

There is no cost challenge for this problem: the search found a design at 90% of the reference's cost, not under the 80% a challenge needs.

## Why the first try breaks

A common first try is the same path with 3 Celery Workers (24 slots), the queue's default of 3 retries, no Dead-Letter Queue and one PostgreSQL. It costs about $1,392 per month and passes "A normal day" with the same p95 of about 489 ms; its Workers are 57% busy.

With the giant files, 24 slots face about 108 slots of work. The Workers are 99% busy, the queue grows all minute, and the p95 climbs past a minute.

When the primary dies, there is nothing to promote. Every status check fails from then on, for 16.9% errors. Every file fails its write, is retried after 1, 2 and 4 seconds, and is then lost. The Workers stay 93% busy with writes that cannot succeed, the queue backs up behind them, and by the end 1,794 files are lost and the p95 is over a minute. Even with a replica, 3 retries are not enough: 7 seconds of waiting runs out long before a 30-second failover ends, and about 380 files are lost. A Dead-Letter Queue doesn't rescue it either: the same 380 files wait in the Dead-Letter Queue instead, and the test fails on them, because a dead-lettered file is a file not loaded.

The point is this: a queue does not make work safe by itself. Retries have to outlast the failure you expect, the database needs a copy that can take over, and the Workers need room for the biggest files, not the average one.

## What the simulator leaves out

The judge counts the files that end up in the Dead-Letter Queue, but replaying them is not simulated: in real life someone has to find each one and send it again.

Parsing and loading are one Worker step here: a real loader would stream the file, write it in batches and make each batch idempotent, so that a retry does not insert the same rows twice. Duplicate rows, partial loads and malformed files are not simulated. A giant file is modelled as 25 times the work at every step, not as a file of a particular size. The rows' content is never checked.

## Things to try

- Keep 8 retries but remove the replica, and run "The database's primary dies". Then lower "Maximum retries" to 5 (31 s of waiting) and then 4, with the replica back.
- Cut the Workers to 12 and run the giant files again. Then try an autoscaling group.
- Set the PostgreSQL failover time to 10 s and see how the failover p95 changes.
