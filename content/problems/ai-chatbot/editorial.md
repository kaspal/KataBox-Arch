# Editorial

## Where the load goes

The App Servers do little: they pass each message on and wait for the answer. The real limits are the provider's, and they are counted in tokens per minute, per key. Your account is already at the top tier, 1,000,000 tokens a minute per key, so the only way to get more is more keys.

A message sends about 800 tokens and gets about 300 back: 1,100 tokens. At the peak of 5 messages a second that is 5 × 60 × 1,100 = 330,000 tokens a minute, 33% of one key's 1,000,000. One key is plenty for a normal day. The request limit is not the problem: 300 requests a minute against 4,000.

The launch multiplies that by 4: 1,320,000 tokens a minute, more than one key allows. A key holds one minute's allowance and refills it as the minute goes by, so a single key runs dry after about 1,000,000 / (1,320,000 − 1,000,000) of a minute, a little over 3 minutes by the arithmetic. After that it lets through only 1,000,000 of every 1,320,000 tokens, and the rest are refused with a 429.

The overload test is different: from 20 seconds in, for 20 seconds, one Model Inference card fails every call. With one provider, every message in those 20 seconds fails.

The limit cut is different again: from 20 seconds in, for 40 seconds, the provider cuts one card's limit to 50,000 tokens a minute, and its stored allowance with it. The card is still up and still answers, so health checks see nothing wrong; it just refuses most calls with a 429.

## The reference design

A Web client with a Timeout of 30,000 ms, 3 Retries, a Retry backoff of 200 ms and the Backoff jitter set to Full (random wait) sends messages to an AWS ALB, which spreads them over 2 Go App Servers. They call two Model Inference cards in turn: a Claude API and an Amazon Bedrock, each at the launch tier of 1,000,000 tokens a minute. This costs about $62,553 per month, and the tokens are almost all of it: about $31,204 on the Claude API and $31,185 on Bedrock, $140 for the App Servers and $25 for the ALB. Two keys cost no more than one: the bill is per token, and the tokens are the same. A normal day has a time to first token (TTFT) p95 of about 1.45 s, a p95 for the whole answer of about 7.5 s, and no errors.

"Launch day" multiplies the traffic by 4 from 30 seconds in, for 4 minutes, in a run of 5 minutes. A real launch lasts hours; 4 minutes is long enough to use up a key's stored allowance, so what is left is the steady rate. The two keys take 2,000,000 tokens a minute together, and the launch uses 66% of that. There are no errors, the TTFT p95 is about 1.43 s and the whole answer about 7.1 s.

"The model provider is overloaded for 20 s" stops one Model Inference card from 20 seconds in, for 20 seconds, in a run of 90 seconds. A call to it fails at once. For 2 to 4 seconds, until two health checks in a row find it down, the App Servers keep sending half the messages there; after that every message goes to the other provider, which has plenty of room at normal traffic (330,000 of its 1,000,000 tokens a minute). The messages that failed in those seconds are tried again by the client after a random wait of up to 200 ms, and the App Servers take turns between the two cards, so with 3 retries one of them almost always reaches the provider that still works. About 0.2% of messages fail, the TTFT p95 is about 1.5 s and the whole answer about 8.8 s.

"The model provider cuts your limit for 40 s" cuts the Claude API card to 50,000 tokens a minute from 20 seconds in, for 40 seconds, in a run of 90 seconds. The App Servers still send it half the messages, about 165,000 tokens a minute, because the card is not down. Its stored 50,000 tokens last about 20 seconds; after that it refuses about two calls in three. The client tries each refused message again, and the next try usually goes to Bedrock, which is not cut. About 0.7% of messages fail, the TTFT p95 is about 1.5 s and the whole answer about 7.4 s.

There is no cost challenge for this problem. The tokens are more than 99% of the bill and the traffic sets them, so the cheapest design found that passes every test costs the same as the reference.

## Why the first try breaks

A common first try is a Web client with the Timeout at 30,000 ms and no retries, the same ALB and 2 App Servers, and one Claude API. It costs the same $62,553 and passes "A normal day" with the same numbers.

On launch day it breaks with about 12.0% errors. About 2 minutes into the launch, the key's stored minute is used up, and from then on roughly 3 calls in 10 are refused with a 429, second after second, until the launch ends. That is sooner than the arithmetic above: the token counts in the statement are typical values, and the long messages pull the average up. The App Servers are not the problem: they are only about 16% busy on a normal day. The provider's limit is.

The overload is about 27.9% errors: every message in those 20 seconds fails at once, because there is nowhere else to send it. The limit cut is about 28.3% errors: about 8 seconds after the cut, the key's store is empty, and from then on most messages are refused until the cut ends.

Fixing one half is not enough:

- One Claude API with the same retries fails all three hidden tests: the launch with about 7.4% errors, the overload with about 26.5% and the limit cut with about 26.7%. Every retry goes back to the same key, and retries with a backoff of 200 ms wait at most about 1.4 s in total; they can't wait out a key that stays short for minutes, a provider that is down for 20 seconds or a cut that lasts 40.
- The reference's two providers without retries pass the launch and fail the other two. The overload is about 2.7% errors: the messages sent to the failed card before the health checks noticed. The limit cut is about 7.3%: the health checks never notice a cut, so every message the App Servers send to the Claude API after its store runs out can be refused, and nothing tries it again.

The point is this: a hosted model's limits are part of your capacity plan. Work out tokens a minute at your peak and at your launch, and hold enough keys to cover it at a steady rate, not by luck. And a single provider is a single point of failure: a second one, plus client retries with a backoff and a random wait, turns its bad day, whether it is down or only refusing, into slightly slower answers.

## What the simulator leaves out

The quality of the answers is not simulated: the Claude API and Bedrock cards give the same answers, at the same speed and price, as far as the simulator is concerned. A real second provider may need a different prompt and give different answers. The tier is given here; a real provider decides it, often based on your spending history, and raising it can take time. A real overload usually returns 429 or 529 errors for some calls rather than failing every call; here the card is simply down. The overload and the limit cut each hit one card; a real provider's bad day hits every key you hold with it. Retry-After headers, prompt caching, growing conversation histories and the provider's own queueing are not simulated. Streaming is modelled only as timing: the time to the first token and the time to the last. The token prices are the registry's teaching figures.

## Things to try

- Remove the Bedrock card and run "Launch day". Find the second when the errors start, and compare it with the arithmetic above.
- Set Retries to 0 on the Web client and run the limit cut. Find the second when the errors start.
- Set Retries to 0 on the Web client and run the overload. Look at its first few seconds.
