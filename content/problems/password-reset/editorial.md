# Editorial

## Where the load goes

At 400 requests a second, 280 are reset requests and 120 are redeems. Each reset request ends in one call to the email provider, so the provider sees 280 calls a second, well under its limit of 600. A caller holds a slot for each call: by the median alone, 280 × 0.15 s keeps about 42 calls in flight, and the slow tail, up to 0.8 s, adds more. None of this is heavy work.

The hidden tests ask other questions. While the provider is down for 30 seconds, about 280 × 30 = 8,400 reset emails come in with nowhere to go yet. And 10% of redeems land on 20 tokens being opened at once: 12 a second on a normal day, 48 a second when there are four times as many redeems. Two redeems of the same token that overlap can both read "unused".

## The reference design

A Client goes through an AWS ALB to 2 Go App Servers. They keep the tokens in PostgreSQL with Conditional writes turned on, so the redeem is one statement: `UPDATE … SET used = true WHERE token = ? AND used = false`. The loser of a race is refused cleanly, with "this link was already used". For a reset request, the App Server publishes the email to a RabbitMQ queue with Maximum retries set to 10, and 4 Celery Mailer instances with a concurrency of 32 consume it and call the provider. This costs about $775 per month: PostgreSQL $260, the Mailers $280, the App Servers $140, RabbitMQ $70 and the ALB $25. On a normal day the p95 completion is about 608 ms, with no errors.

"The email provider is down for 30 s" takes the provider away from 20 seconds in, in a run of 120 seconds, and allows a p95 of 60 seconds. The refused emails wait in the queue and are retried after 1, 2, 4, 8, 16, 32, 32, 32, 32 and 32 seconds: 191 seconds of waits in all, far longer than the outage. They go out once the provider is back: the p95 is about 29.5 s, with no errors and no lost resets.

"Mail scanners open every link twice" sends four times the redeems and allows no conflicts, where a conflict is a token used twice. Every race on a token has one winner, and the other redeem is refused: the p95 is about 498 ms, with no conflicts.

There is no cost challenge for this problem: the reference is already lean, and no cheaper design found was cheap enough to make one.

## Why the first try breaks

A common first try uses the same edge and 2 Go App Servers that send the email themselves, inline, with no queue (a Celery worker card sits on the canvas, unwired), and PostgreSQL without conditional writes. It costs about $565 and passes "A normal day" with a p95 of about 452 ms, faster than the reference because there is no queue hop. But even on that normal day, 7 tokens were used twice. The test does not judge it, but the bug is already there.

When the provider goes down, an inline send has nowhere to keep the email. The App Server's call fails, and so does the request: 18.5% of the run's requests end in an error, and 3,532 reset emails are counted as lost: they failed with no queue to keep them and nothing will ever retry them. When mail scanners open every link, the App Server reads the token, sees "unused" and writes, and a second redeem does the same in between: 93 conflicts, and the test fails.

There are two lessons here. A reset email is work that may wait, and a queue with retries turns an outage into a delay. A one-time token needs a check and a set in one statement, not "read, then write".

## What the simulator leaves out

Tokens do not expire here, and they are not hashed before they are stored. There is no per-user rate limit on reset requests. Email deliverability and spam filtering are not simulated. The mail scanner itself is not modelled either: it shows up only as more redeems of the same tokens at once.

## Things to try

- Set the RabbitMQ queue's Maximum retries back to 3 (1 + 2 + 4 = 7 seconds of waits) and run the outage again. How many resets survive a 30-second outage?
- Turn off Conditional writes on PostgreSQL in the reference and run "A normal day". Look for tokens used twice, even though the test passes.
- Cut the Mailers to 1 instance and run the outage again. Watch how long the backlog takes to drain once the provider is back.
