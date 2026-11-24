# Editorial

## Where the load goes

At 4,000 requests/s, about 3,960 are segment plays and about 40 are uploads. The plays are cheap one by one, but they are many and very repetitive: with Zipf 1.1 over 500,000 segments, the same popular segments are requested again and again. Segments never change once written, so they are ideal for a CDN with a long TTL. The uploads are the opposite: few, but each needs about 400 ms of transcoding. That work must not sit on the path of anything a viewer is waiting for.

## The reference design

The Client talks to a CDN (CloudFront) with a TTL of a day. Segment misses go to Object Storage (S3), which holds every rendition. The CDN forwards uploads to a Load Balancer in front of 2 Go Upload API servers. Each upload is written to S3 and published to an SQS Queue, and 8 Transcoder Workers drain the Queue and write the renditions back.

This costs about $12,515 per month, most of it CDN requests. The p95 is about 7 ms, because nearly every play ends at the edge. It handles the season premiere, which sends 10 times the traffic for 20 seconds with even hotter segments (Zipf 1.4): hotter segments only raise the CDN hit rate. It also handles the upload rush, when uploads run at 6 times their normal rate: the Queue lengthens a little, and viewers never notice.

## Why the first try breaks

A common first try is a Load Balancer in front of 3 Go video servers that serve the segments themselves and also run the upload path. On a normal evening it passes with a p95 near 17 ms. At the premiere, 40,000 requests a second land on 3 servers with no edge to absorb them. They run out of slots, work queues up, and about 24% of requests fail.

## Things to try

- Lower the CDN TTL to a few seconds and watch the hit rate and the origin load on the visible test.
- Replace S3 behind the CDN with the 3 Go servers. Does the premiere still pass?
- Cut the Transcoder Workers to 2 and run the upload rush. Who waits, and does anyone fail?
