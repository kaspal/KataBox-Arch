# Editorial

## Where the load goes

The App Servers do little: they pass each message on and wait for the answer. The real limits are the provider's, and they are counted in tokens per minute, per key.

A message sends about 800 tokens and gets about 300 back: 1,100 tokens. At the peak of 5 messages a second that is 5 × 60 × 1,100 = 330,000 tokens a minute, 82% of one key's 400,000. One key is enough for a normal day. The request limit is not the problem: 300 requests a minute against 4,000.

The launch multiplies that by 4: 1,320,000 tokens a minute. A key holds one minute's allowance and refills it as the minute goes by, so a single key at 400,000 a minute runs dry after about 400,000 / (1,320,000 − 400,000) of a minute, roughly 26 seconds. After that it lets through only 400,000 of every 1,320,000 tokens, and the rest are refused with a 429.

The overload test is different: from 20 seconds in, for 20 seconds, one Model Inference card fails every call. With one provider, every message in those 20 seconds fails.

## The reference design

A Web client with a Timeout of 30,000 ms, 3 Retries, a Retry backoff of 200 ms and the Backoff jitter set to Full (random wait) sends messages to an AWS ALB, which spreads them over 2 Go App Servers. They call two Model Inference cards in turn: a Claude API and an Amazon Bedrock, each with the Tokens per minute limit raised to 1,000,000. This costs about $93,747 per month, and the tokens are almost all of it: about $46,805 on the Claude API and $46,777 on Bedrock, $140 for the App Servers and $25 for the ALB. Two keys cost no more than one: the bill is per token, and the tokens are the same. A normal day has a time to first token (TTFT) p95 of about 1.0 s, a p95 for the whole answer of about 8.6 s, and no errors.

"Launch day" multiplies the traffic by 4 from 30 seconds in, for 3 minutes, in a run of 4 minutes. A real launch lasts hours; 3 minutes is long enough to use up any stored allowance, so what is left is the steady rate. The two keys take 2,000,000 tokens a minute together, and the launch uses 66% of that. There are no errors, the TTFT p95 stays at about 1.0 s and the whole answer at about 8.1 s.

"The model provider is overloaded for 20 s" stops one Model Inference card from 20 seconds in, for 20 seconds, in a run of 90 seconds. A call to it fails at once. For 2 to 4 seconds, until two health checks in a row find it down, the App Servers keep sending half the messages there; after that every message goes to the other provider, which has plenty of room at normal traffic (330,000 of its 1,000,000 tokens a minute). The messages that failed in those seconds are tried again by the client after a random wait of up to 200 ms, and the App Servers take turns between the two cards, so with 3 retries one of them reaches the provider that still works. There are no errors, the TTFT p95 is about 1.0 s and the whole answer about 9.5 s.

There is no cost challenge for this problem. The tokens are more than 99% of the bill and the traffic sets them, so the cheapest design found that passes every test costs the same as the reference.

## Why the first try breaks

A common first try is a Web client with the Timeout at 30,000 ms and no retries, the same ALB and 2 App Servers, and one Claude API at its current tier. It costs the same $93,747 and passes "A normal day" with the same numbers.

On launch day it breaks with about 53.7% errors. About 22 seconds into the launch, the key's stored minute is used up, and from then on most calls are refused with a 429, second after second, until the launch ends. The App Servers are not the problem: they are only about 17% busy on a normal day. The provider's limit is.

The overload is about 28.1% errors: every message in those 20 seconds fails at once, because there is nowhere else to send it.

Fixing one half is not enough:

- One Claude API raised to 1,000,000 tokens a minute, with the same retries, passes the launch (about 4.5% errors) and still fails the overload with about 26.7% errors. Retries with a backoff of 200 ms wait at most about 1.4 s in total; they can't wait out 20 seconds.
- Two keys at the current tier, with retries, pass the overload and fail the launch with about 15.9% errors: 800,000 tokens a minute is less than the launch's 1,320,000.
- The reference's two raised keys without retries fail the overload with about 2.9% errors: the messages sent to the failed card before the health checks noticed.

Three keys at the current tier with retries pass every test, but only just: 1,200,000 tokens a minute is a little less than the launch needs, and the design survives on each key's stored minute. A longer launch would break it.

The point is this: a hosted model's limits are part of your capacity plan. Work out tokens a minute at your peak and at your launch, and have keys and a tier that cover it at a steady rate, not by luck. And a single provider is a single point of failure: a second one, plus client retries with a backoff and a random wait, turns its bad day into slightly slower answers.

## What the simulator leaves out

The quality of the answers is not simulated: the Claude API and Bedrock cards give the same answers, at the same speed and price, as far as the simulator is concerned. A real second provider may need a different prompt and give different answers. Raising a tier is a prop here, free and instant; a real provider decides it, often based on your spending history, and it can take time. A real overload usually returns 429 or 529 errors for some calls rather than failing every call; here the card is simply down. Retry-After headers, prompt caching, growing conversation histories and the provider's own queueing are not simulated. Streaming is modelled only as timing: the time to the first token and the time to the last. The token prices are the registry's teaching figures.

## Things to try

- Remove the Bedrock card and run "Launch day" and the overload. Which test breaks, and why can't the raised tier help with it?
- Set Retries to 0 on the Web client and run the overload. Look at its first few seconds.
- Set both keys back to 400,000 tokens a minute and run "Launch day". Find the second when the errors start.
