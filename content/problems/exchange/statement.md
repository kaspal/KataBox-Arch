# Order-Matching Exchange

## The product

You run the core of a stock exchange. Brokers' gateways send you orders to buy and sell shares, and your exchange matches each order against the order book of its symbol: the list of open orders waiting for a price. Traders care about speed, and they care even more about the slow orders: an order that sits for a few hundred milliseconds while the price moves costs someone money. Every order must reach its book; none may be lost.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 10,000 requests/s |
| p95 completion latency | 30 ms or less |
| p99 completion latency | 50 ms or less |
| Availability | 99.9% or more |
| Budget | $2,800 per month |
| Lost messages | None |

## The traffic

Brokers' gateways reach an Order gateway, an App Server, usually through a Load Balancer.

- **Place an order** (100%). The Order gateway publishes the order to a stream and answers at once. A Worker, the matcher, takes the order from the stream and matches it in the order book, which lives in memory in a Cache. Latency is measured until the order has reached its book, not until the gateway answers.

There are 5,000 symbols, and their trading follows a Zipf 1.1 curve: a few popular symbols get a large share of the orders. No outside providers are involved.

This is the first problem with a p99 limit as well as a p95. The p99 is the time that 99 orders in 100 beat, so only 1 order in 100 may be slower than it. A design with a few badly stuck orders can pass the p95 and still fail the p99.

If the matchers stop, the orders waiting in the stream still count: when a test ends, any order not yet matched counts at the age it has reached by then.

## The tests

One test is visible. You can run it as often as you like:

- **Normal session**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The market opens** and **A matcher dies**. Your design has to pass all three.
