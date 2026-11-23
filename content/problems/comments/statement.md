# Comment Threads

## The product

You run the comments under the articles of a busy news site. Every article has a thread, and every reader who scrolls down loads it. Most readers only read; a few post a comment. There are 100,000 threads, but on most evenings readers spread over many of them. Some evenings one story takes off, and everybody wants the same thread at once. Readers will put up with a comment showing up a moment late. They won't put up with a thread that doesn't load.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 6,000 requests/s |
| p95 latency | 60 ms or less |
| Availability | 99.9% or more |
| Errors | 1% or less |
| Budget | $1,200 per month |

## The traffic

- **Load a thread's comments** (96%). The request may pass a load balancer, reaches your App Servers, and they read the thread from a cache if there is one (cache-aside), and from the data store on a miss. A thread is read as one key.
- **Post a comment** (4%). The request may pass a load balancer, reaches your App Servers, and they write the comment to the data store. If there is a cache, the App Server then refreshes that thread's entry in it.

Both kinds of request pick a thread out of 100,000, and a few popular threads get most of them (a Zipf popularity of 0.9). There are no outside services in this problem.

## The tests

One test is visible. You can run it as often as you like:

- **A normal evening**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **One thread goes viral** and **The cache is flushed on a busy evening**. Your design has to pass all three.
