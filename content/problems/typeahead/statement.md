# Typeahead

## The product

You run the suggestion box under a search field. As someone types, each keystroke sends the current prefix, and you answer with a short list of likely completions. The answer has to arrive before the next key is pressed, or the list feels stuck. Everyone starts with the same few letters, so a small set of short prefixes makes up most of what you are asked, while a long tail of rarer prefixes shows up now and then.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 6,000 requests/s (one per keystroke) |
| p95 latency | 60 ms or less |
| Availability | 99.9% or more |
| Budget | $3,000 per month |

## The traffic

- **Suggestions for a prefix** (100%). The request may pass an edge component such as a Load Balancer, then reaches an App Server. The App Server may check a Cache first, and otherwise asks the Search index. There are 200,000 prefixes, and their popularity follows a Zipf 1.1 curve: short, common prefixes dominate. The answer for a prefix changes slowly, so it can be cached.

Latency is measured until the suggestions return to the Client.

## The tests

One test is visible. You can run it as often as you like:

- **Normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **Everyone types the same thing** and **The index rebuilds**. Your design has to pass all three.
