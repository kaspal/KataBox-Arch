## 1

In the first seconds of a sale, what is almost everyone doing, and which part of your design has to answer all of them?

## 2

Serve the event pages from the edge so the crowd never reaches your servers, and let a deep queue level the purchases: take every order quickly, then charge at whatever pace the payment provider allows.

## 3

Put a CDN with a short TTL (a minute is fine) in front of Object Storage holding the event pages, and let it forward checkout to a Load Balancer and a few Checkout API servers. Give the Queue in front of the Payment Workers a very large max depth so it never refuses an order.
