# Editorial

## Where the load goes

At 800 URLs/s, almost all the time goes into one step: the fetch. It takes about 250 ms at the median, and a Worker keeps its thread for the whole wait. That alone means about 200 threads busy at any moment, more when the slow tail shows up. The web allows 1,000 fetches/s, so there is little headroom. With Zipf 0.9 over 2 million URLs, a fair share of what the frontier sends are repeats. Each repeat you fetch holds a thread and spends rate limit on a page you already stored. A lookup of a millisecond can replace a fetch of 250.

## The reference design

A Scheduler (Airflow) publishes URLs to an SQS Queue. 6 Crawler Workers with 64 threads each take them from the Queue. Before fetching, each Worker checks a Redis Cache of seen URLs (4 GB, TTL one day). A hit means "already seen, skip it", and the work ends there. A miss records the URL, fetches the page and writes it to Cassandra with 8 partitions.

This costs about $5,026 per month. Its p95 is about 0.4 s on a normal crawl. With duplicates, most URLs are skipped and p95 drops to about 0.06 s. With slow hosts it reaches about 1.7 s.

## Why the first try breaks

A common first try is the same design without the seen-URLs Cache. It passes "Normal crawl" with a p95 near 1.3 s. It fetches everything, but there are enough threads to cope.

"A site full of duplicate links" doubles the URL rate and makes repeats far more common (Zipf 1.6). Without the check, the Workers fetch every repeat. Their threads fill, the Queue grows, and p95 reaches about 38 s. "Slow hosts" adds 1.5 s to every fetch for 30 s, and allows a p95 of up to 8 s. Each fetch now holds a thread several times longer, the backlog builds, and p95 reaches about 23 s. The reference survives both because it fetches far fewer pages, so its threads stay free.

## Things to try

- Remove the Cache from the reference and compare how many fetches reach the web on the normal crawl.
- Lower the TTL to one minute and watch the hit rate fall.
- Remove Workers one at a time and find the smallest count that still keeps p95 under the limit when fetches slow down.
