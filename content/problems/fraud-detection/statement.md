# Payment Fraud Scoring

## The product

You run the fraud check for a payment network. Partner servers, the checkout systems of shops and banks, ask you to score each card payment before they approve it, and they also send you a stream of card events. A fraud model looks at the card's recent features and returns a score. The check sits in the middle of checkout, so it has to be fast: a shopper waiting at the till does not care why the answer is late. If the model cannot answer in time, a simple set of rules decides instead.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 3,000 requests/s |
| p95 latency | 60 ms or less |
| Availability | 99.9% or more |
| Error rate | 0.5% or less |
| Budget | $3,900 per month |

## The traffic

Every request comes from a partner server and may pass through a load balancer on its way to your API.

- **Score a payment** (60%). The API reads the card's features from a cache, then asks the fraud model for a score. The model is a classifier: it takes about 20 tokens of features (up to about 40) and needs about 20 ms per score. The model step is optional: if the call fails or takes too long, the payment is checked by rules and the request still succeeds.
- **Record a card event** (40%). The API publishes the event to a stream. A stream processor reads it, aggregates it and updates that card's features in the cache, so the next score sees them.

There are about 5 million cards, and each request is about one of them.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The model server slows down** and **Black Friday**. Your design has to pass all three.
