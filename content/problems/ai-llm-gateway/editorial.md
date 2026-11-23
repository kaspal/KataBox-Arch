# Editorial

## Where the load goes

Eight requests a second is very little traffic for a gateway. What runs out first is the provider account: 400,000 tokens a minute, shared by ten teams. A typical request is about 400 tokens in and 60 out, about 460 tokens, so a normal day spends about 8 × 460 × 60 ≈ 221,000 tokens a minute, a bit over half the account. The account's request limit of 4,000 a minute is further away: 8 × 60 = 480.

Each team's share of 55,000 tokens a minute is 55,000 / 60 ≈ 917 tokens a second, about 2 typical requests a second. On a normal day each team sends about 0.8 requests a second, about 370 tokens a second.

In the flood, one team sends 40 times its usual traffic, about 32 requests a second. Together with the other nine teams' 7.2, that is about 39 × 460 × 60 ≈ 1,080,000 tokens a minute, almost three times what the account allows. The provider's limit is a bucket holding one minute's allowance, so it lasts a while and then refuses whatever comes, from any team.

## The reference design

An AWS ALB spreads the traffic over two API Gateway cards, LiteLLM (LLM gateway). Both are wired to one shared Rate Limiter, Redis token bucket, with its Count set to Tokens (prompt + answer), a Limit of 917 tokens a second and a Burst of 10,000 tokens. The limiter keeps a bucket per API key, and because it is one card, a team gets the same 917 tokens a second whichever gateway its calls land on. Each request takes its own prompt and answer tokens from the bucket, so a team that sends long prompts gets fewer requests through than one that sends short ones. The Burst is about 20 typical requests: a team's calls arrive at random, and long ones sometimes come close together. With a Burst of 3,000 a few of an ordinary team's calls get 429s even on a normal day, enough to miss the 99.9% availability goal. Behind the limiter, a Go App Server (2 instances) calls a Model Inference card, Claude API, held at the account's 4,000 requests and 400,000 tokens a minute.

It costs about $47,924 per month. Almost all of it is tokens: $47,449 for the Claude API. The two gateways cost $140 each, the App Server $140, the limiter $30 and the ALB $25. The token bill is a little above 8 requests a second × a month × $0.0021 per typical request ≈ $44,000, because the long requests pull the average up. A normal day has a p95 of about 2.2 s for the whole answer, a p95 time to first token of about 0.8 s, and no errors.

"One team runs a script that floods the gateway" runs for 120 seconds with that team at 40 times its traffic. Its bucket at the limiter lets 917 tokens a second through, about 2 typical requests, and refuses the rest, which counts as success. The model gets the other nine teams' usual traffic plus at most 55,000 tokens a minute from the script: by the estimate above, about 199,000 + 55,000 ≈ 254,000 tokens a minute, well under the account's 400,000. There are no errors, and the p95 stays at about 2.2 s.

"A gateway node is lost" stops the first gateway card 20 seconds in, for 20 seconds. This test allows 5% errors and 95% availability. About half the requests fail in the two seconds before the load balancer stops sending traffic to the dead card; then the second gateway carries everything. The test ends with about 2.7% errors. With a single gateway card, every request fails for the whole 20 seconds: about 38% errors.

The cost challenge has no target here: the search found nothing cheaper than the reference by more than a few dollars, because the tokens are 99% of the bill and every design pays for the same tokens on a normal day.

## Why the first try breaks

A common first try is the same ALB and two LiteLLM gateways, wired straight to the App Server, with no limiter. It costs about $47,894 and passes "A normal day" just like the reference.

In the flood nothing stands between the script and the account. The account's bucket starts with a full minute's allowance, and about 30 seconds in it is empty. From then on, roughly half of every second's requests are refused with a 429, and the nine teams that did nothing wrong lose their calls along with the flooding team's. The test ends with about 42.5% errors. The bill grows too: priced as a month, the flood costs about $125,029, against $59,705 for the reference in the same test.

The second gateway card makes the naive design pass "A gateway node is lost": the two failures are separate lessons.

The point is this: a shared provider limit is a shared resource, and the provider can't tell your teams apart. Only your gateway can. Give each tenant its own limit there, sized from its share of the tokens, and check that all the normal traffic plus one tenant at its limit still fits under the account.

## What the simulator leaves out

The Rate Limiter takes a request's prompt and answer tokens from the bucket as the request arrives, as if it knew the answer's length in advance; a real gateway counts the prompt up front and settles the answer after the call. In the simulator a gateway card can't call a model, so an App Server makes the call for it. LiteLLM's other features (falling back to another provider or model, caching, budgets in dollars per key) are not simulated. The provider's limit is a bucket holding one minute's allowance, refilled continuously; real providers count in their own ways. Streaming isn't simulated: an App Server waits for the whole answer, and the time to first token is measured inside the model call. Retry-After headers are not simulated. Prices are the registry's teaching prices for the Claude API ($3 per million input tokens and $15 per million output tokens), and a run's tokens are priced as if the peak lasted a month. The quality of the answers is not simulated at all.

## Things to try

- Remove the Rate Limiter and run the flood. Find the second when the account runs out, and compare it with 400,000 tokens over the flood's deficit a minute.
- Set the limiter's Limit to 10,000 tokens a second and run the flood again. Then try 6,000: it passes the 2-minute test, but work out how long the account's minute of tokens would last at that rate.
- Set the limiter's Count to Requests, with a Limit of 2 and a Burst of 10. Work out how many tokens a minute a team gets through if every one of its prompts is 1,000 tokens long.
- Delete the second gateway card and run "A gateway node is lost".
