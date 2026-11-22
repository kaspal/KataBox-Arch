# Editorial

## Where the load goes

Before the sale, 2,000 requests/s split into about 1,800 page views and 200 purchases. The trouble starts when the sale opens: traffic jumps 25 times for 10 seconds, to about 50,000 requests a second. Some 45,000 of those are people reloading the event page, and there are only 200 pages, so a CDN can answer nearly all of them. The other 5,000 a second are orders, more than the payment provider accepts. They cannot all be charged right away, and they do not need to be. They need to be stored and charged in order.

## The reference design

The Client talks to a CDN with a TTL of 60 seconds. Page misses go to Object Storage (S3). The CDN forwards checkout to an Application Load Balancer in front of 4 Go Checkout API servers. Each server puts the order on an SQS Queue with a max depth of 1,000,000 and answers the buyer. Payment Workers, 8 instances with 64 slots each, take orders from the Queue, charge the provider and write the ticket to PostgreSQL.

This costs about $5,429 per month. The p95 stays near 25 ms through the opening, because the buyer's answer only waits for the Queue. When the payment provider slows down, adding 3 seconds to every charge for 30 seconds, the Workers drain more slowly and the Queue gets longer. No buyer waits, and no order is lost.

## Why the first try breaks

A common first try is a Load Balancer in front of 4 Go shop servers that serve the event pages themselves and also take checkout, feeding a RabbitMQ Queue with a max depth of 5,000 and the same Payment Workers. Before the sale it passes with a p95 near 18 ms. When the sale opens, every page view lands on those 4 servers. They run out of slots, requests queue behind each other, and about 17% of requests fail. The page views drown the purchases they share servers with.

## Things to try

- Keep the CDN but shrink the Queue max depth to 5,000. Run the opening and count lost orders.
- Remove the CDN and add shop servers until the opening passes. Compare the cost.
- Cut the Payment Workers to 2 instances and run the slow provider. How long does the backlog take to clear?
