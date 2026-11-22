# Editorial

## Where the load goes

Ninety-nine of every hundred requests ask for a file that is the same for everyone and changes only on a deploy. The popularity is skewed: a handful of files, like the home page and the main script, take a large share of the traffic. That makes the work easy to cache. Only the contact form, 1% of the traffic or about 15 requests per second, needs an App Server and a database.

So the real question is not how many servers you need. It is how few requests should reach a server at all.

## The reference design

The Client talks only to a CDN (CloudFront) with a TTL of one day. Behind it, Object Storage (S3) holds the site files. A hit is answered at the edge in a couple of milliseconds. A miss goes to Object Storage once and fills the CDN, so the next visitor gets a hit.

The CDN also forwards the contact form to a Load Balancer, one Go App Server and PostgreSQL. At this volume, one App Server is plenty.

The design costs about $3,417 per month. Almost all of it is CDN requests, priced at $0.75 per million in this model. The p95 is about 7 ms.

Wire the Client to the CDN alone. If you also connect it to the Load Balancer, the traffic splits and half the file requests skip the CDN.

## Why the first try breaks

A common first design is Client, Load Balancer, two Go App Servers that serve the files themselves, and PostgreSQL. It costs about $515 and passes a normal day with a p95 near 17 ms.

The hidden test "The origin slows down" adds 300 ms to every App Server and Object Storage request from 10 s to 50 s. Now every file request holds an app thread for 300 ms. The threads run out, requests queue, and about 47% of them fail. With the CDN in front, the same test changes nothing visible, because hits never reach the origin.

The other hidden test, "A deploy purges the CDN", empties the CDN at 20 s. Misses go to Object Storage while the edge refills, and the p95 rises to about 32 ms for a while, still under the target.

## Things to try

- Shorten the TTL to a few seconds and watch the origin load and the p95 climb.
- Use an App Server as the origin instead of Object Storage, then run the slow origin test again.
- Wire the Client to both the CDN and the Load Balancer, and compare the hit ratio.
