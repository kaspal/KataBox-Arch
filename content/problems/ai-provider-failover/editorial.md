# Editorial

## Where the load goes

20 messages a second is 1,200 a minute. A message carries about 600 prompt tokens and 60 answer tokens, so at peak the assistant uses about 1,200 × 660 ≈ 792,000 tokens a minute. An App Server wired to two chat models sends its calls to them in turn, so on a normal day each provider gets about 600 requests and 396,000 tokens a minute, well inside both accounts.

An answer takes seconds, not milliseconds. On the Claude API the first token comes after about 1,100 ms plus 600 prompt tokens at 5,000 a second, about 1.22 s in all, and the 60 answer tokens take another 60 / 100 a second = 0.6 s. The OpenAI API starts sooner and writes slower: about 890 ms + 0.12 s ≈ 1.01 s to the first token, then 60 / 65 a second ≈ 0.92 s. A long answer of 150 tokens after a 1,500-token prompt takes about 2.9 s on the Claude API and 3.5 s on the OpenAI API, and the rare very long ones on the OpenAI API run past 5 s.

When one provider fails, the other one has to carry everything: 1,200 requests and about 792,000 tokens a minute. The OpenAI account allows 1,500 and 1,000,000, so it can, with about 20% to spare. That is the first thing to check in any failover plan: the backup must fit the whole load inside its own limits, not just its usual half.

The two hidden tests fail a provider in two different ways. "Down" means the card stops answering: calls to it fail quickly, and health checks notice it in about 4 s. "Hangs" means the card takes every call and answers 60 s later: nothing fails, health checks see a live card, and every message sent there just waits.

## The reference design

The Web client has a Timeout of 30,000 ms. An AWS ALB sends the messages to 2 Go App Servers with a Call timeout of 5,000 ms, the Circuit breaker at errors at 20% and Breaker open for 60 s. The App Servers sit in an Availability zone with a Linkerd Service mesh and Mesh retries 3, and they are wired to a Claude API card (4,000 requests and 2,000,000 tokens per minute) and an OpenAI API card (1,500 requests and 1,000,000 tokens per minute). This costs about $107,023 per month, and almost all of it is tokens: $52,438 for the Claude API, $54,420 for the OpenAI API, $140 for the App Servers and $25 for the ALB. The token bill uses the registry's list prices per million tokens, as if the peak lasted all month. A normal day has a p95 of about 3,082 ms for the whole answer, a p95 time to first token of about 1.4 s, and no errors.

Each setting does one job:

- **The call timeout** stops the wait. 5 s is above almost every normal answer: on a normal day it cuts about 0.35% of them (very long answers on the OpenAI API), and the retry answers them again. A shorter timeout cuts more long, healthy answers, and each cut answer is paid twice: with 3,000 ms the normal day costs about $116,315, and the messages that waited 3 s before their retry push the p95 time to first token over 4 s, far past its limit.
- **The circuit breaker** stops paying the timeout on every call. Once at least 20 calls to one provider in the last 10 s have failed at 20% or more, the App Server fails its calls to that provider at once for 60 s. With Breaker open for at 10 s, the breaker closes while the provider is still hanging, the next calls wait 5 s each again, and the p95 for the whole answer goes over its limit (about 7,080 ms).
- **The retry** turns a failed call into an answer. A retry goes to the next provider in turn. After an open breaker, that is always the other provider; after a timeout it can be the same hanging one again, which is why the reference allows 3 retries. With 2, about 0.4% of the messages run out of tries during the hang: it still passes, but with little margin.

"A model provider is down for 60 s" kills the first chat model card (in card id order) 20 seconds in, for 60 seconds, in a run of 100 seconds. Calls to it fail quickly, the mesh retries them on the other provider, and after about 4 s the health checks take the dead card out of turn. There are no errors, and the p95 is about 3,336 ms.

"A model provider hangs for 60 s" adds 60 s to every call on that card for the same minute. For the first seconds, the messages sent there wait for the timeout, then the retry sends them to the other provider. After a few seconds of timeouts the breaker opens, and from then on the calls that land on the hanging provider fail at once and are retried on the other one without waiting. There are no errors, and the p95 for the whole answer is about 3,757 ms. The p95 time to first token is about 1.4 s, well under this test's limit of 8 s: the messages that waited for a timeout first are fewer than 1 in 20.

For the cost challenge, the proof search found no design under 80% of the reference's cost, so this problem has no challenge. The bill is tokens, and every design pays for the same tokens.

## Why the first try breaks

A common first try is the same edge and App Servers with only the Claude API card, and no call timeout. It costs about $106,135, a little less than the reference: both providers charge the same list price per token, and the reference also pays again for the few long answers its timeout cuts and retries. It passes "A normal day" with a p95 of about 2,791 ms.

When the provider is down, there is nowhere else to go: about 62.1% of the messages fail, all of them in that minute.

When the provider hangs, it is worse. Messages wait until the Web client gives up at 30 s, so the first errors arrive 30 s after the hang starts, and the messages that get through after it ends were stuck for a long time: about 35.0% errors, and a p95 of about 25,145 ms.

Adding the second provider is not enough on its own. With both cards and no timeout, breaker or retry, "down" fails about 2.0% of the messages (the 4 s before the health checks notice), and "hangs" fails about 19.8%: half the calls go to a provider that looks healthy and never answers. Each missing piece shows up too:

- A timeout and retries without the breaker: about 2.3% errors and a p95 of about 12,573 ms during the hang. Every call to the hanging provider waits 5 s before it moves on, and the simulator bills the calls left hanging: that test's cost rises to about $153,540.
- A timeout and the breaker without retries: about 34.9% errors. The breaker fails fast, but a fast failure is still a failure.

The point is this: a backup provider is only useful if traffic actually reaches it within seconds. A provider that hangs looks healthy, so you need a timeout to notice, a breaker to stop paying that timeout on every call, and a retry to send the message to the backup. And the backup must fit the whole load inside its own limits.

## What the simulator leaves out

Answer quality is not simulated: the two providers' models give different answers, and nothing here measures that, or the work of keeping one prompt working well on both. The call timeout covers the whole answer, while real streaming clients usually also time out the first token on its own. Real failover usually prefers the main provider and uses the backup only when it fails; here an App Server sends its calls to both in turn, so the backup serves half the traffic every day. The circuit breaker has no half-open probes. The simulator bills a call that starts even if nobody waits for its answer; what a real provider bills for an abandoned call is not modelled. Provider status pages and Retry-After headers are not simulated. Prices are the registry's list prices per million tokens.

## Things to try

- Set Mesh retries to 2, then to 0, and run the hang. Count the messages that run out of tries.
- Set Breaker open for to 10 s and watch the p95 during the hang.
