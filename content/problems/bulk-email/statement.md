# Bulk Email Campaigns

## The product

You run a marketing platform. Your customers write a campaign, pick their audience and press send, and your platform delivers the emails, one per recipient. You do not run mail servers yourself: every email goes out through an outside email provider, and that provider sets how fast you may send. A campaign may arrive a little later than planned, and nobody minds. What a customer does mind is a recipient who never gets the email, so none may be lost.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 500 emails/s |
| p95 completion latency | 30 s or less |
| Availability | 99.9% or more |
| Budget | $1,500 per month |
| Lost messages | None |

## The traffic

There is no user-facing API here. A Scheduler fires every second and publishes that second's emails to a queue.

- **Send a campaign email** (100%). A Worker takes the email from the queue and calls the email provider. Latency is measured from the moment the email is published until the provider accepts it.

The provider answers in about 150 ms at the median, with a slow tail that reaches 1.2 s. It also has a rate limit: it accepts at most 800 calls a second from you. Calls over the limit are refused (an HTTP 429) and count as failed, so the email has to be tried again later or it is gone. A Worker keeps its slot for the whole call.

## The tests

One test is visible. You can run it as often as you like:

- **Normal campaign**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The provider cuts your rate limit** and **Two campaigns overlap**. Your design has to pass all three.
