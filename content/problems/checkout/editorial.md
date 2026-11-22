# Editorial

## Where the load goes

At 4,000 requests/s, about 3,200 are product views and about 800 are orders. With Zipf 1.1 over 50,000 products, a cache answers most product views.

Orders are where the time goes. Storing the order takes milliseconds; the charge takes about 300 ms at the median and up to 1.5 s in the tail. Whoever makes that call holds a slot for all of it. By the median alone, 800 orders a second × 0.3 s keeps about 240 slots busy. When the provider adds 2.5 s, each charge takes about 2.8 s, and the same 800 orders a second need about 2,240 slots.

## The reference design

A Load Balancer (AWS ALB) sends shoppers to 4 Go instances of the Shop API, at the default concurrency of 64: 256 slots. Product views go cache-aside through a Redis Cache with a TTL of 60 s to PostgreSQL, a primary and a replica. An order is written to the primary, then published to an SQS Queue with max_retries set to 8 and a Dead-letter Queue behind it. 12 Celery payment workers, each with a concurrency of 64, consume it: 768 slots. The shopper's answer leaves as soon as the order is stored and queued, so the p95 is about 32 ms. This costs about $2,767 per month.

"The payment provider slows down" adds 2.5 s to every payment call from 15 seconds, for 30 seconds. The workers' 768 slots are fewer than the 2,240 the slow charges need, so a backlog builds in the queue and drains once the provider recovers. The App Servers never wait on the provider, so the p95 stays at about 32 ms. "Double-click storm (orders ×2)" doubles the order traffic for the whole run, to about 1,600 orders a second. Each is still a quick write and a publish: the p95 is about 35 ms. The bill rises to about $3,612 with the extra traffic.

## Why the first try breaks

A common first try is the same shop with no queue: 6 Go instances, 384 slots, and each one calls the payment provider inline. It costs about $1,181 per month and passes "Normal day" with a p95 of about 500 ms, because every order waits for its charge. The App Servers are already about 89% busy.

Both hidden tests push past that. When the provider slows down, the charges need about 2,240 slots and there are 384. When orders double, 1,600 charges a second need about 480 slots at the median alone, more than the 384 there are. In both tests about 40% of all requests are dropped, product views included: they wait for the same App Server slots, and every slot is stuck on a payment.

The queue is not free. SQS costs about $886 per month here and the workers $840: together more than the first try's whole bill. What it buys is isolation: a slow provider fills a queue instead of the shop.

## What the simulator leaves out

A double click in the simulator really creates two orders; the test measures the extra load, not deduplication. A real checkout sends an idempotency key so a repeat is charged once. The stock reservation's correctness under concurrent orders is not simulated either: nothing checks for overselling.

## Things to try

- Delete the queue from the reference and run "The payment provider slows down". Watch product views fail with the orders.
- Cut the payment workers to 4 instances and run the same test. The shopper's p95 barely moves; look at the queue's depth instead.
