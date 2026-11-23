## 1

Count what the model can take: 400,000 tokens a minute is about 6,700 a second, and a question uses about 620. How many questions a second can reach the model, and how many arrive? Then ask which of them the model really has to answer.

## 2

An ordinary cache only hits when the question is written exactly the same way, and people rarely write it the same way. A cache that matches questions by meaning answers the near-repeats, and when one topic goes viral almost every question about it is a near-repeat. That cache's copy is what keeps the model under its limit, so it must not come back empty after a restart.

## 3

Use a Go App Server card (its default 2 instances) behind the Web client, and a Claude API Model Inference card with its defaults. Read from a Redis Cache with Mode "Semantic (near-repeats)" and 2 GB of memory, wired with Replicate to a second Redis Cache with the same settings.
