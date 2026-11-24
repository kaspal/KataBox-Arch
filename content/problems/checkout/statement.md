# Checkout & Inventory

## The product

You run an online shop. Most visitors browse: they open product pages, compare, and open a few more. Some of them buy. A purchase stores the order, reserves the stock so the item is not sold twice, and charges the card through an outside payment provider. You do not run the payment provider, and on a bad day it gets slow. Whatever it does, the shop must keep answering, and an order a shopper placed must never quietly disappear.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 4,000 requests/s |
| Reads | 80% of requests |
| p95 latency | 800 ms or less |
| Availability | 99.9% or more |
| Budget | $6,200 per month |
| Lost messages | None |

Latency is measured until the shopper gets an answer.

## The traffic

Shoppers reach the Shop API, an App Server, usually through a Load Balancer.

- **Browse products** (80%). The App Server reads the product, from a Cache first if your design has one, otherwise from the database. There are 50,000 products, and their popularity follows a Zipf 1.1 curve: a small set of products gets most of the views.
- **Place an order** (20%). The App Server writes the order and reserves the stock in the primary database. Then the card is charged through the payment provider, which answers in about 300 ms at the median, with a slow tail that reaches 1.5 s. The charge can be handed to a queue, where a Worker picks it up and calls the provider. If the App Server has no queue to publish to, it calls the provider itself, keeps its slot for the whole call, and the shopper waits for it.

If you cache products, a product page can show a stock level up to one cache lifetime old while orders change it. That is fine here: stale reads are not judged, because the order itself reserves against the database.

## The tests

One test is visible. You can run it as often as you like:

- **Normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The payment provider slows down** and **Double-click storm (orders ×2)**. Your design has to pass all three.
