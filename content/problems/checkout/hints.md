## 1

Placing an order has two halves: storing it, and charging the card. Which half does the shopper really need to wait for, and what happens to your App Servers while the payment provider takes its time?

## 2

Store the order and reserve the stock, then hand the charge to a queue and answer the shopper. Workers behind the queue call the payment provider, so a slow provider fills the queue instead of the App Servers' slots, and browsing keeps working. Keep the cache in front of product reads too.

## 3

Wire the Shop API to an SQS Queue with max retries raised to about 8 and a Dead-letter Queue behind it, and let Celery workers with a high concurrency, around 64, consume it: charging is waiting on the network. Count worker slots as orders a second times the seconds a charge takes, and give room for the slow tail. With the charge off the request path, a few Go instances are enough.
