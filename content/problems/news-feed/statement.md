# News Feed

## The product

You run the home feed of a social app. People open it many times a day and expect to see the latest posts from the accounts they follow, at once. Most of the work is reading: a feed is opened far more often than anyone posts. When someone does post, everyone who follows them should find it in their feed soon after. Most accounts have a handful of followers, but a few have a great many.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 10,000 requests/s |
| Reads | 97% of requests |
| p95 latency | 100 ms or less |
| Availability | 99.9% or more |
| Budget | $3,000 per month |

## The traffic

- **Open your feed** (97%). The request may pass an edge component such as a Load Balancer, then reaches an App Server. The App Server may check a Cache first, and otherwise reads the data store. There are 1 million users, and how often each feed is opened follows a Zipf 1.0 curve. Each feed page is about 16 KB.
- **Publish a post** (3%). The App Server writes the post to the primary database and publishes it to a broker, and the author is done. A Worker picks it up and writes it into the feed of each follower, either in a Cache or in the database. A post has 10 followers on average, but a few accounts have about 50 times more.

## The tests

One test is visible. You can run it as often as you like:

- **Normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **Everyone opens the app at once** and **Breaking news**. Your design has to pass all three.
