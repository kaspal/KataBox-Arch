## 1

The upload itself is easy: the App Server stores the file and queues a message. The hard part happens later, in the Workers. What happens to a message whose write to PostgreSQL fails, and how long does it keep trying? And how much work is waiting in the queue when the files get 25 times bigger?

## 2

A database failover takes about 30 seconds, and the queue's default 3 retries give up after 7. Raise "Maximum retries" until the waits add up to more than a failover, and give the database a replica that can take over. Wire a Dead-Letter Queue for whatever still fails, but treat every file in it as a file not loaded. Then size the Workers for the slots giant files hold, not for a normal day.

## 3

Wire the partner server to 2 Go App Servers, which write to Amazon S3, read PostgreSQL and publish to an Amazon SQS Queue with "Maximum retries" set to 8 (1 + 2 + 4 + 8 + 16 + 32 + 32 + 32 = 127 s of waiting). Wire the queue with Publish to a Dead-Letter Queue. 16 Celery Workers (concurrency 8) consume it, read S3 and write PostgreSQL. Wire PostgreSQL with Replicate to a second PostgreSQL, keeping the 30 s failover time.
