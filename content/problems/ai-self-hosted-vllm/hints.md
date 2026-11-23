## 1

Counting slots is not counting capacity. A GPU with 64 slots holds 64 answers at once, but how many tokens a second does it write when all 64 are busy? Compare that with the tokens a second your peak needs, at double traffic, and with one card gone.

## 2

A full batch writes each answer at 40 tokens a second, so one GPU writes at most 64 × 40 = 2,560 tokens a second. Double traffic asks for 40 × 250 = 10,000 tokens a second, and more than that, because long answers pull the average up. Four GPUs would have to run almost full, and a GPU near full queues new messages, so their time to first token climbs by seconds. Plan for GPUs about half to two-thirds busy at the worst moment. And spread them over several cards of the same size: when one card is lost, the others must carry the normal peak.

## 3

Put an AWS ALB in front of 4 Go App Servers. Wire them to three vLLM on GPU cards, each with 2 Instances and Batch slots per GPU at 64. On the Web client, set the Timeout to 30,000 ms, Retries to 2, the Retry backoff to 200 ms and the Backoff jitter to Full (random wait).
