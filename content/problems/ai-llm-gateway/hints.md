## 1

The provider's limit belongs to the whole account, not to any one team. If one team sends forty times its usual traffic, whose calls get the 429s? And if the only gateway stops, who can reach the model at all?

## 2

Give every team its own limit before its calls reach the provider: a Rate Limiter keeps a bucket per API key, so one team's flood empties only its own bucket. It counts requests, so turn the 55,000 tokens a minute into requests a second with the average request size. Check that nine normal teams plus one team at its limit stay under the account's 400,000 tokens a minute. And don't let the gateway be one card.

## 3

Put an AWS ALB in front of two API Gateway cards, LiteLLM (LLM gateway), each wired to one shared Rate Limiter (Redis token bucket) with a Limit of 2 a second and a Burst of 10. The limiter sends the requests on to a Go App Server that calls a Model Inference card (Claude API) with its limits left at the defaults.
