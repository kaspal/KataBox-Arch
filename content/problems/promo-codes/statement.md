# Promo Codes

## The product

You run the promo-code service for an online shop. At checkout, a shopper types a code and the shop asks your service whether it is valid and how many uses it has left. Some codes can be used only a set number of times: "the first 500 orders get 20% off". When a shopper places the order, the shop redeems the code, and your service takes one use off it. A code that is used more times than it allows costs the shop real money, so a limited code must never be over-redeemed. Checkout must also keep working when the database has a bad day.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 2,000 rps |
| p95 latency | 100 ms or less |
| Availability | 99.9% or more |
| Error rate | 1% or less |
| Budget | $1,300 per month |

## The traffic

Shoppers' requests reach your App Servers, through an edge if you want one.

- **Check a code at checkout** (95%). An App Server looks up the code, through a cache if you add one, in the data store. There are 5,000 codes, and a few popular ones get most of the checks.
- **Redeem a code with limited uses** (5%). An App Server reads the code's remaining uses from the data store, then writes the new count. 30% of redeems are on one code that everyone is using right now. If two redeems both read "1 use left" before either writes, both succeed, and the code is used once more than it allows. The simulator counts that as a conflict.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **An influencer shares a code** and **The database primary fails**. Your design has to pass all three.
