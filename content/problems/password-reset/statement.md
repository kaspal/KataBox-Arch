# Password Reset

## The product

You run the "Forgot your password?" flow for a large web service. A user types their email address, your service saves a one-time reset token and sends them an email with a link. The user opens the link and sets a new password. The email may take a little while, and nobody minds. What users do mind is an email that never comes. A link must also work exactly once: a token that sets a password twice is a security bug.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 400 requests/s |
| p95 latency | 1,000 ms or less |
| Availability | 99.9% or more |
| Errors | 1% or less |
| Budget | $1,300 per month |
| Lost messages | None |

## The traffic

- **Ask for a reset link** (70%). The request may pass a load balancer, reaches your App Servers, and they write a reset token to the data store. Then the email goes out through the email provider. If the App Server hands it to a queue, a Worker takes it from the queue and calls the provider. With no queue, the App Server calls the provider itself, and the answer waits for it. Latency is measured from the request until the provider accepts the email.
- **Open the link and set a new password** (30%). The request may pass a load balancer, reaches your App Servers, and they read the token from the data store, then write to it: they mark the token used and save the new password. Most redeems are for different tokens, but 10% of them hit one of 20 tokens that are being opened at the same moment.

The email provider answers in about 150 ms at the median, with a slow tail that reaches 800 ms at the p99. It accepts at most 600 calls a second from you.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The email provider is down for 30 s** and **Mail scanners open every link twice**. Your design has to pass all three.
