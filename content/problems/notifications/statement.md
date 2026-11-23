# Notification System

## The product

You run a service that sends notifications on behalf of other apps: push alerts to phones, emails and text messages. An app calls your Notify API and expects a quick answer; the sending happens behind it. You do not run the push, email or SMS networks yourself. Outside providers do, and they are sometimes slow and now and then down. Whatever happens to them, a notification someone asked for must never quietly disappear.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 1,500 notifications/s |
| p95 completion latency | 3 s or less |
| Availability | 99.9% or more |
| Budget | $9,000 per month |
| Lost messages | None |

## The traffic

Every notification takes the same path. The Notify API, an App Server, publishes it to a queue and is done. A Worker takes it from the queue and sends it through the matching external provider. Latency is measured until the provider accepts the notification.

- **Push notification** (60%). The push provider answers in about 40 ms at the median.
- **Email** (30%). The email provider answers in about 150 ms at the median.
- **SMS** (10%). The SMS provider answers in about 300 ms at the median.

All three providers have slow tails: now and then a call takes many times its median. A Worker keeps its slot for the whole call.

## The tests

One test is visible. You can run it as often as you like:

- **Normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The email provider goes down** and **A marketing blast**. Your design has to pass all three.
