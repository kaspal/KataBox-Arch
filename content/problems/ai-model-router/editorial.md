# Editorial

## Where the load goes

The services do little: they pass each question on and wait for the answer. The load is on the models, and on the large model it is counted in tokens a minute.

Token counts vary from question to question, and a few are much longer than the typical one, so the averages are a little above the figures in the statement: about 1,420 tokens per easy question (in and out together) and about 4,600 per hard one. At the peak, the easy questions send 8 × 60 × 1,420 ≈ 682,000 tokens a minute and the hard ones 2 × 60 × 4,600 ≈ 552,000. Easy questions are only small one by one; there are four times as many, so they are more than half of the tokens.

With every question on the large model, the key carries about 1,234,000 tokens a minute, 82% of its 1,500,000. A normal day fits. The newsletter triples the easy questions: about 2,046,000 + 552,000 ≈ 2,600,000 tokens a minute. The key holds one minute's allowance and refills as the minute goes by, so it runs dry after about 1,500,000 / (2,600,000 − 1,500,000) of a minute, roughly 80 seconds, and from then on it refuses about 4 calls in 10 with a 429.

The bill follows the same tokens. The large model costs $2 per million input tokens and $10 per million output tokens (the provider's list prices, as the registry cites them), so the easy questions alone cost about $86,000 a month on it. A vLLM GPU node costs $734 a month (a g5.xlarge-class node), whatever it serves.

## The reference design

A Web client with a Timeout of 60,000 ms sends questions to a Kong API Gateway, which passes easy ones to 2 Go App Servers and hard ones to 2 Microservices. The App Servers call a vLLM on GPU card running the small model, with 4 Instances (16 Batch slots each, the default) and the Output speed at 150. The Microservices call a Claude API card with the Tokens per minute limit at 1,500,000. This costs about $112,826 a month: $109,462 for the hard questions' tokens on the Claude API, $2,936 for the 4 GPU nodes, $148 for the gateway and $140 each for the App Servers and the Microservices.

On a normal day only the hard questions reach the key: about 552,000 tokens a minute, 37% of its limit. An easy answer holds a batch slot for about 1.3 s, so 8 a second keep about 11 of the 64 slots busy: 17%. The p95 for the whole answer is about 18.0 s, set by the hard answers (1,200 tokens at 100 a second is 12 s), there are no errors, and the time to first token stays under its 2.5 s target.

"Twice as many hard questions" doubles the hard traffic for 2 minutes. The key now carries about 1,100,000 tokens a minute, 74% of its limit. The p95 is about 19.8 s and there are no errors, and the time to first token stays under 2.5 s: a hard question's first token takes the provider's 1.1 s plus the time to read its prompt, and with more hard questions in the mix more of the slowest first tokens are theirs.

"A newsletter brings three times the easy questions" runs for 3 minutes. The key doesn't see it at all. The GPUs take 24 easy questions a second, about 36 slots busy out of 64; each answer decodes a little slower when more slots are busy, but stays around a second and a half. The p95 is about 12.8 s (more of the traffic is short answers) and there are no errors.

There is no cost challenge for this problem. The hard questions' tokens are 97% of the bill and the traffic sets them; the cheapest design the search found that passes every test costs about $110,868, 98% of the reference.

## Why the first try breaks

A common first try is the same gateway and services, with both the App Servers and the Microservices calling one Claude API card. It costs about $196,056 a month, under the budget, and passes "A normal day" with the same p95 of about 18.0 s.

"Twice as many hard questions" passes too: about 1,786,000 tokens a minute is more than the key allows, but the stored minute lasts about 5 minutes at that rate, and the test lasts 2. A longer wave would break it.

The newsletter breaks it with about 17.9% errors. Around 77 seconds in, the key's stored minute is used up, and from then on, second after second, 10% to 50% of the calls are refused with a 429, hard questions as much as easy ones. The services are not the problem: on a normal day the App Servers are about 16% busy and the Microservices about 21%. The key's token limit is.

Retries don't fix it. With 3 Retries, a Retry backoff of 200 ms and the Backoff jitter at Full (random wait) on the client, the errors fall to about 10.4%, still a failure: a retry waits a fraction of a second, and the key needs a minute to refill.

The reference's routing with too few GPUs fails too: with 2 Instances on the vLLM card, the 24 easy questions a second fill all 32 slots, answers queue and slow down, and the newsletter ends with about 20.7% errors and a p95 of about 57.8 s. 3 Instances pass; the reference keeps 4 for headroom.

The point is this: a large model is expensive in money and in quota, and most questions don't need it. Send each kind of question to the cheapest model that can take it. Then the large model's tokens a minute, and its bill, go to the work that needs them, and a wave of easy questions lands on capacity you size and pay for by the month.

## What the simulator leaves out

The simulator doesn't judge answers. Whether the small model's answers to easy questions are good enough is the real question behind this design, and you have to answer it outside the simulator: with an evaluation set, user feedback, and a way to send a question on to the large model when the small one isn't sure. Here the questions arrive already labelled easy or hard; a real router has to decide, with rules or a classifier, and it is sometimes wrong. The small model's speed is the Output speed you set, not a measured model. GPU nodes start instantly and never fail here; real ones take minutes to load a model and need spares. The provider's tier is set by the problem: the Claude API cards show its limits read-only. Prompt caching, streaming beyond the first and last token, and the provider's own queueing are not simulated. The token prices and the GPU node price are the registry's figures, from the providers' list prices.

## Things to try

- Set the vLLM card to 2 Instances and run the newsletter. Find the second when the answers start to queue.
- Wire the App Servers to the Claude API card as well as the vLLM card. What share of the easy questions now spend the key's tokens, and what does it do to the bill?
- Wire the Microservices to the vLLM card instead of the Claude API, and look at the GPUs' slots and the p95. The simulator lets you; whether the answers would still be good enough is the question it can't settle.
