# Shopping Cart

## The product

You run the cart service of a large online shop. Shoppers look at their cart, add and remove items, and check out. There are 2,000,000 carts. Most days are calm, but sales and flash deals bring sudden crowds, and the app tier scales out to meet them. A cart change must never be lost: a shopper who adds an item expects to see it there at checkout.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 6,000 rps |
| p95 latency | 150 ms or less |
| Availability | 99.9% or more |
| Error rate | 1% or less |
| Budget | $1,800 per month |
| Lost writes | None |

## The traffic

Requests come in through an optional edge to your app servers. Each request touches one of the 2,000,000 carts.

- **View the cart** (60%). The app reads the cart, from a cache first if you have one, and from the database on a miss.
- **Add or remove an item** (30%). The app writes the change to the database's primary, then refreshes the cart in the cache if you have one.
- **Check out** (10%). The app writes the order to the database's primary.

This problem counts database connections. PostgreSQL accepts at most 1,700 connections (its max_connections, the figure for an RDS db.m5.xlarge). Every app slot that talks to the database directly holds one connection, so an App Server card holds instances × concurrency connections. Connections over the limit are refused, and the requests that needed them fail.

## The tests

One test is visible. You can run it as often as you like:

- **A normal afternoon**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The sale starts** and **A flash deal at noon**. Your design has to pass all three.
