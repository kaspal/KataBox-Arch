# LLM Gateway for Many Teams

## The product

You run the platform team at a company where ten product teams build features on a large language model. Nobody holds their own provider key: every team calls the model through one shared LiteLLM gateway, with its own API key, and the gateway forwards the calls to one company account with the Claude API. That account has a hard limit of 4,000 requests and 400,000 tokens a minute. When the account goes over it, the provider refuses calls with an HTTP 429, and it doesn't care which team spent the tokens: everybody's calls are refused.

Each team has agreed to a share of 55,000 tokens a minute. One of the teams sometimes runs a batch script against the gateway, and nothing has stopped it yet.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 8 requests/s |
| p95 latency (the whole answer) | 5,000 ms or less |
| p95 time to first token | 1,500 ms or less |
| Availability | 99.9% or more |
| Errors | 1% or less |
| Budget | $80,000 per month |

## The traffic

- **A team's chat completion** (90%). Nine teams, each with its own API key, share this traffic evenly.
- **Calls from the team that floods** (10%). The tenth team, with one API key. On a normal day it sends as much as any other team.

A typical request sends about 400 tokens of prompt (1,000 at p95) and gets about 60 tokens back (120 at p95); the long ones pull the average a little higher. Each request passes a Load Balancer and the API Gateway (and any Rate Limiter you put on the way), then an App Server calls the Model Inference card and waits for the whole answer. In the simulator a gateway card passes requests on but can't call a model itself, so the App Server stands for the gateway's worker that makes the provider call.

The Model Inference card's "Requests per minute limit" and "Tokens per minute limit" are your account's limits with the provider: keep them at their defaults (4,000 and 400,000). Changing the prop doesn't change the contract.

A Rate Limiter keeps a token bucket for each API key: its Limit is requests a second per key, and its Burst is how many a key may send at once. It counts requests, not tokens. A request it refuses gets a 429. For the nine ordinary teams that is an error; for the team that floods, a refusal is exactly what should happen, so it counts as a success.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **One team runs a script that floods the gateway** and **A gateway node is lost**. Your design has to pass all three.
