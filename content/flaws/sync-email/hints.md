## 1

Checkout fails while the database is fine. What else does checkout wait for?

## 2

Does the customer need the email to be sent before the order is confirmed?

## 3

Add a queue the API publishes to, and a worker that consumes it and sends the email.
