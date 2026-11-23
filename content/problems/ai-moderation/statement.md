# Post Moderation

## The product

You run a social app. Before a new post goes live, a moderation classifier reads its text and labels it as fine or as breaking the rules. A post is never published unchecked: if the check fails, the user sees an error and has to try again. You host the classifier yourself on NVIDIA Triton, on your own GPUs. Most days the traffic is steady, but when a post goes viral, a flood of replies and reposts arrives within seconds.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 1,500 posts/s |
| p95 latency | 200 ms or less |
| Availability | 99.9% or more |
| Error rate | 1% or less |
| Budget | $5,000 per month |

## The traffic

- **Publish a post** (100%). The request may pass a load balancer, reaches your App Servers, and they call the classifier before they accept the post. The classifier takes about 40 tokens of text (up to about 90) and returns one label. On Triton it needs about 13 ms for a typical post, and each GPU instance works on up to 64 posts at once (its Batch slots per GPU). An App Server keeps its slot for the whole call.

The model step is required: a post that cannot be checked is not published, and the request fails.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A post goes viral** and **A model server is lost**. Your design has to pass all three.
