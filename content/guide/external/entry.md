A third-party API is a service someone else runs that your system calls: a payment processor, an SMS gateway, an email sender. You don't size it or operate it. What you control is **how you call it**: from which component, inside or outside the user's request, and how fast.

In the simulator, an external card stands for a provider the problem declares. The problem sets its latency, its rate limit and any outages. Drawing the card lets you route to it explicitly and give different callers their own path, for example the app for card payments and a worker behind a queue for emails. Cards bound to the same provider **share its rate limit**, because the limit belongs to the account, not to the card. Without a card, the provider is still there, just implicit.

The key design move is keeping slow or unreliable providers off the user's path. A call made while the user waits fails when the provider fails. A call made by a worker from a queue can wait and retry.

**How the simulator bills this.** The generic API and Stripe cost nothing per call (Stripe really charges a share of each payment, which isn't modelled). Twilio bills $8,300 per million SMS, $0.0083 each, and SendGrid $900 per million emails, about $0.0009 each. These are list prices from Twilio's US SMS pricing and SendGrid's Pro plan (read 2026-11-23).
