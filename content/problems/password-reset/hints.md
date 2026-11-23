## 1

Two things can go wrong here that have nothing to do with speed. The email provider can stop answering for a while, and the same link can be opened twice at the same moment. Where is a reset email kept while the provider is away? And what stops two opens of one link from both finding the token unused?

## 2

A reset email is work that may wait. If the App Server hands it to a queue that retries with growing waits, an outage becomes a delay instead of a lost email, as long as the retries last longer than the outage. For the token, "read it, then write it" leaves a gap: two redeems can both read "unused" before either writes. The check and the write have to happen in one step, so the second one is refused.

## 3

Wire the App Servers to a RabbitMQ queue with Maximum retries at 10 (the waits run 1, 2, 4, 8, 16, 32, 32… seconds, more than three minutes in all), and have 4 Celery Mailer instances with a concurrency of 32 consume it and call the provider. On the PostgreSQL card, turn on "Conditional writes (check and set in one step)". 2 Go App Servers behind an AWS ALB are enough for the rest.
