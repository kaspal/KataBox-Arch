# Help-Centre Answer Bot

## The product

Your company's help centre gets a chatbot. A customer types a question in their own words ("how do I reset my password?", "I forgot my password, what now?") and the bot answers it with a hosted language model, the Claude API. Every question sent to the model costs tokens, and your account has a tokens-per-minute limit. But most questions are not new: many people ask the same thing in slightly different words. Your job is to keep the model for the questions it really has to answer.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 12 requests/s |
| p95 latency | 5,000 ms or less |
| Availability | 99% or more |
| Errors | 1% or less |
| Budget | $80,000 per month |

## The traffic

- **Ask the help centre a question** (95%). The request may pass an edge card, reaches your App Servers, may read a cache (cache-aside), and on a miss the App Server calls the model and puts the answer in the cache. There are about 100,000 distinct ways the questions are written, and some exact wordings are much more common than others (a Zipf popularity of 0.9). About 60% of these questions are near-repeats: a question with the same meaning was asked before.
- **Ask about the topic everyone is talking about** (5%). The same route. On a normal day this is a small topic, but the customers who ask it write it in about 50,000 different ways, and 95% of them mean the same thing as an earlier question.

A question sends about 500 tokens to the model (900 at p95: the question, the instructions and the help articles) and gets about 120 back (250 at p95). Your Model Inference card is the Claude API with its defaults: its speeds, a Requests per minute limit of 4,000 and a Tokens per minute limit of 400,000. Those limits are your account's tier and you can't raise them for this launch (the simulator applies them to every Claude API card (the Inspector shows them, read-only)), and the answers are needed while the customer waits, so the API mode stays Real time. A call over a limit is refused with a 429 and counts as a failed request. The client gives up after 5 s.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **One question goes viral: three times the traffic** and **The evening peak, and a cache node restarts**. Your design has to pass all three.
